use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RecordItem {
    id: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
    date: String,
    status: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "completedAt")]
    completed_at: Option<String>,
    #[serde(rename = "rolledOverFromDate")]
    rolled_over_from_date: Option<String>,
}

struct DbState(Mutex<Connection>);
struct CompactState(Arc<AtomicBool>);

fn normalize_todo_statuses(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE records
         SET status = CASE
             WHEN completed_at IS NOT NULL THEN 'done'
             ELSE 'pending'
         END
         WHERE type='todo' AND (status IS NULL OR status NOT IN ('pending', 'done'))",
        [],
    )
}

fn next_todo_status(current_status: Option<&str>) -> (Option<String>, Option<String>) {
    match current_status {
        Some("done") => (Some("pending".to_string()), None),
        _ => (
            Some("done".to_string()),
            Some(chrono::Utc::now().to_rfc3339()),
        ),
    }
}

fn rollover_overdue_todos(conn: &Connection, today: &str) -> rusqlite::Result<usize> {
    #[derive(Debug)]
    struct OverdueInfo {
        id: String,
        rolled_over_from_date: Option<String>,
        original_date: String,
    }

    let overdue = {
        let mut stmt = conn.prepare(
            "SELECT id, rolled_over_from_date, date FROM records
             WHERE type='todo'
               AND (status='pending' OR (status IS NULL AND completed_at IS NULL))
               AND date < ?1",
        )?;

        let rows = stmt.query_map(params![today], |row| {
            Ok(OverdueInfo {
                id: row.get(0)?,
                rolled_over_from_date: row.get(1)?,
                original_date: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let now = chrono::Utc::now().to_rfc3339();
    let mut updated = 0;
    for item in &overdue {
        let rolled_from = item
            .rolled_over_from_date
            .as_deref()
            .unwrap_or(&item.original_date);
        updated += conn.execute(
            "UPDATE records SET date=?1, status='pending', rolled_over_from_date=?2, updated_at=?3 WHERE id=?4",
            params![today, rolled_from, now, item.id],
        )?;
    }

    Ok(updated)
}

fn init_db(app_data_dir: &std::path::Path) -> Connection {
    std::fs::create_dir_all(app_data_dir).ok();
    let db_path = app_data_dir.join("frostnote.db");
    let conn = Connection::open(&db_path).expect("Failed to open database");

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS records (
            id          TEXT PRIMARY KEY,
            type        TEXT NOT NULL,
            content     TEXT NOT NULL,
            date        TEXT NOT NULL,
            status      TEXT,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            completed_at TEXT,
            rolled_over_from_date TEXT
        );",
    )
    .expect("Failed to create table");

    // Enable WAL mode for better concurrent access
    conn.execute_batch("PRAGMA journal_mode=WAL;").ok();
    normalize_todo_statuses(&conn).expect("Failed to normalize todo statuses");

    conn
}

#[tauri::command]
fn get_all_records(db: State<'_, DbState>) -> Result<Vec<RecordItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date FROM records")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(RecordItem {
                id: row.get(0)?,
                record_type: row.get(1)?,
                content: row.get(2)?,
                date: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                completed_at: row.get(7)?,
                rolled_over_from_date: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for row in rows {
        records.push(row.map_err(|e| e.to_string())?);
    }
    Ok(records)
}

#[tauri::command]
fn add_record(db: State<'_, DbState>, record: RecordItem) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO records (id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            record.id,
            record.record_type,
            record.content,
            record.date,
            record.status,
            record.created_at,
            record.updated_at,
            record.completed_at,
            record.rolled_over_from_date,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn update_record(db: State<'_, DbState>, record: RecordItem) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE records SET type=?1, content=?2, date=?3, status=?4, updated_at=?5, completed_at=?6, rolled_over_from_date=?7 WHERE id=?8",
        params![
            record.record_type,
            record.content,
            record.date,
            record.status,
            record.updated_at,
            record.completed_at,
            record.rolled_over_from_date,
            record.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_record(db: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM records WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn toggle_todo(db: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // Read current status
    let current_status: Option<String> = conn
        .query_row(
            "SELECT status FROM records WHERE id=?1 AND type='todo'",
            params![id],
            |row| row.get(0),
        )
        .ok();

    let (new_status, completed_at) = next_todo_status(current_status.as_deref());

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE records SET status=?1, completed_at=?2, updated_at=?3 WHERE id=?4",
        params![new_status, completed_at, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn rollover_todos(db: State<'_, DbState>, today: String) -> Result<Vec<RecordItem>, String> {
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        rollover_overdue_todos(&conn, &today).map_err(|e| e.to_string())?;
    }

    get_all_records(db)
}

#[tauri::command]
fn migrate_records(db: State<'_, DbState>, records: Vec<RecordItem>) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    for record in &records {
        conn.execute(
            "INSERT OR IGNORE INTO records (id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.record_type,
                record.content,
                record.date,
                record.status,
                record.created_at,
                record.updated_at,
                record.completed_at,
                record.rolled_over_from_date,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    normalize_todo_statuses(&conn).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");
            let conn = init_db(&app_data_dir);
            app.manage(DbState(Mutex::new(conn)));
            let compact_flag = Arc::new(AtomicBool::new(false));
            app.manage(CompactState(compact_flag.clone()));

            // Snap compact window to top of screen when moved
            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            let flag = compact_flag.clone();
            let w = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Moved(_) = event {
                    if flag.load(Ordering::Relaxed) {
                        if let Ok(Some(monitor)) = w.current_monitor() {
                            let monitor_pos = monitor.position();
                            let target_y = monitor_pos.y + 8;
                            if let Ok(pos) = w.outer_position() {
                                if pos.y != target_y {
                                    let _ = w.set_position(tauri::PhysicalPosition::new(
                                        pos.x.max(0),
                                        target_y.max(0),
                                    ));
                                }
                            }
                        }
                    }
                }
            });

            // Build tray menu
            let show_item = MenuItemBuilder::with_id("show", "显示 FrostNote").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .items(&[&show_item, &quit_item])
                .build()?;

            // Create tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .tooltip("FrostNote")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // No-op: plugin is registered at builder level

            // Register the shortcut
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.global_shortcut().register("Ctrl+Shift+F")?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            minimize_window,
            toggle_maximize_window,
            close_window,
            get_all_records,
            add_record,
            update_record,
            delete_record,
            toggle_todo,
            rollover_todos,
            migrate_records,
            compact_mode,
            restore_mode,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FrostNote");
}

#[cfg(windows)]
#[tauri::command]
fn minimize_window(window: tauri::Window) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;

    unsafe {
        ShowWindow(hwnd.0, SW_MINIMIZE);
    }

    Ok(())
}

#[cfg(not(windows))]
#[tauri::command]
fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|error| error.to_string())
}

#[tauri::command]
fn toggle_maximize_window(window: tauri::Window) -> Result<(), String> {
    let is_maximized = window.is_maximized().map_err(|error| error.to_string())?;

    if is_maximized {
        window.unmaximize().map_err(|error| error.to_string())
    } else {
        window.maximize().map_err(|error| error.to_string())
    }
}

#[tauri::command]
fn close_window(window: tauri::Window) -> Result<(), String> {
    // Hide to tray instead of closing
    window.hide().map_err(|error| error.to_string())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn compact_mode(
    window: tauri::Window,
    compact_state: State<'_, CompactState>,
) -> Result<(), String> {
    use tauri::LogicalSize;

    compact_state.0.store(true, Ordering::Relaxed);

    // Set compact size
    window
        .set_size(LogicalSize::new(380.0, 480.0))
        .map_err(|e| e.to_string())?;

    // Move to top-right, y locked at top of screen
    if let Ok(Some(monitor)) = window.current_monitor() {
        let monitor_size = monitor.size();
        let monitor_pos = monitor.position();
        let window_size = window.outer_size().map_err(|e| e.to_string())?;

        let x = (monitor_pos.x as f64 + monitor_size.width as f64 - window_size.width as f64 - 12.0) as i32;
        let y = monitor_pos.y + 8;

        window
            .set_position(tauri::PhysicalPosition::new(x.max(0), y.max(0)))
            .map_err(|e| e.to_string())?;
    }

    // Pin on top
    window.set_always_on_top(true).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn restore_mode(
    window: tauri::Window,
    compact_state: State<'_, CompactState>,
) -> Result<(), String> {
    use tauri::LogicalSize;

    compact_state.0.store(false, Ordering::Relaxed);

    // Restore normal size
    window
        .set_size(LogicalSize::new(980.0, 680.0))
        .map_err(|e| e.to_string())?;

    // Center the window
    if let Ok(Some(monitor)) = window.current_monitor() {
        let monitor_size = monitor.size();
        let monitor_pos = monitor.position();

        let x = monitor_pos.x + (monitor_size.width as i32 - 980) / 2;
        let y = monitor_pos.y + (monitor_size.height as i32 - 680) / 2;

        window
            .set_position(tauri::PhysicalPosition::new(x.max(0), y.max(0)))
            .map_err(|e| e.to_string())?;
    }

    // Unpin from top
    window.set_always_on_top(false).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(
            "CREATE TABLE records (
                id          TEXT PRIMARY KEY,
                type        TEXT NOT NULL,
                content     TEXT NOT NULL,
                date        TEXT NOT NULL,
                status      TEXT,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                completed_at TEXT,
                rolled_over_from_date TEXT
            );",
        )
        .expect("create records table");
        conn
    }

    fn insert_todo(
        conn: &Connection,
        id: &str,
        date: &str,
        status: Option<&str>,
        completed_at: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO records (id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date)
             VALUES (?1, 'todo', 'legacy todo', ?2, ?3, '2026-06-10T00:00:00Z', '2026-06-10T00:00:00Z', ?4, NULL)",
            params![id, date, status, completed_at],
        )
        .expect("insert todo");
    }

    #[test]
    fn toggling_done_returns_to_pending() {
        let (status, completed_at) = next_todo_status(Some("done"));

        assert_eq!(status.as_deref(), Some("pending"));
        assert!(completed_at.is_none());
    }

    #[test]
    fn normalizes_legacy_null_todo_status() {
        let conn = test_db();
        insert_todo(&conn, "legacy-null", "2026-06-10", None, None);

        normalize_todo_statuses(&conn).expect("normalize todo statuses");

        let status: String = conn
            .query_row(
                "SELECT status FROM records WHERE id='legacy-null'",
                [],
                |row| row.get(0),
            )
            .expect("read status");
        assert_eq!(status, "pending");
    }

    #[test]
    fn rolls_over_legacy_null_pending_todo() {
        let conn = test_db();
        insert_todo(&conn, "legacy-overdue", "2026-06-10", None, None);

        let updated = rollover_overdue_todos(&conn, "2026-06-11").expect("rollover todos");

        assert_eq!(updated, 1);
        let (date, status, rolled_over_from_date): (String, String, String) = conn
            .query_row(
                "SELECT date, status, rolled_over_from_date FROM records WHERE id='legacy-overdue'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read rolled todo");
        assert_eq!(date, "2026-06-11");
        assert_eq!(status, "pending");
        assert_eq!(rolled_over_from_date, "2026-06-10");
    }
}
