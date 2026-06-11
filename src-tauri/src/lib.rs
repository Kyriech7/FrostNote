use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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
    #[serde(default)]
    #[serde(rename = "deletedAt")]
    deleted_at: Option<String>,
}

struct DbState(Mutex<Connection>);
struct CompactState(Arc<AtomicBool>);
struct RestoreGuard(Arc<AtomicBool>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowRestoreStep {
    NativeRestore,
    Show,
    Focus,
}

fn restore_window_steps() -> [WindowRestoreStep; 3] {
    [
        WindowRestoreStep::NativeRestore,
        WindowRestoreStep::Show,
        WindowRestoreStep::Focus,
    ]
}

#[cfg(windows)]
fn native_restore_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_RESTORE};

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;

    unsafe {
        ShowWindow(hwnd.0, SW_RESTORE);
    }

    Ok(())
}

#[cfg(not(windows))]
fn native_restore_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    window.unminimize().map_err(|error| error.to_string())
}

fn restore_main_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    for step in restore_window_steps() {
        match step {
            WindowRestoreStep::NativeRestore => native_restore_window(window)?,
            WindowRestoreStep::Show => window.show().map_err(|error| error.to_string())?,
            WindowRestoreStep::Focus => window.set_focus().map_err(|error| error.to_string())?,
        }
    }

    Ok(())
}

#[cfg(windows)]
fn is_foreground_window(window: &tauri::WebviewWindow) -> Result<bool, String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    let foreground = unsafe { GetForegroundWindow() };

    Ok(foreground == hwnd.0)
}

#[cfg(windows)]
fn watch_foreground_minimized_window(window: tauri::WebviewWindow, restore_guard: Arc<AtomicBool>) {
    const RESTORE_AFTER: Duration = Duration::from_millis(1000);

    std::thread::spawn(move || {
        let mut foreground_minimized_since: Option<Instant> = None;

        loop {
            std::thread::sleep(Duration::from_millis(250));

            let is_minimized = window.is_minimized().unwrap_or(false);
            let is_foreground = is_foreground_window(&window).unwrap_or(false);

            if restore_guard.load(Ordering::Relaxed) && !is_minimized {
                restore_guard.store(false, Ordering::Relaxed);
            }

            if restore_guard.load(Ordering::Relaxed) && is_minimized {
                if !is_foreground {
                    restore_guard.store(false, Ordering::Relaxed);
                }
                foreground_minimized_since = None;
                continue;
            }

            if is_minimized && is_foreground {
                let since = foreground_minimized_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= RESTORE_AFTER {
                    let _ = restore_main_window(&window);
                    foreground_minimized_since = None;
                }
            } else {
                foreground_minimized_since = None;
            }
        }
    });
}

fn should_hide_on_shortcut(window: &tauri::WebviewWindow) -> bool {
    window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for name in columns {
        if name? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn ensure_sync_columns(conn: &Connection) -> rusqlite::Result<()> {
    if !column_exists(conn, "records", "deleted_at")? {
        conn.execute("ALTER TABLE records ADD COLUMN deleted_at TEXT", [])?;
    }

    if !column_exists(conn, "records", "sync_status")? {
        conn.execute(
            "ALTER TABLE records ADD COLUMN sync_status TEXT NOT NULL DEFAULT 'dirty'",
            [],
        )?;
    }

    conn.execute(
        "UPDATE records SET sync_status='dirty' WHERE sync_status IS NULL OR sync_status NOT IN ('dirty', 'synced')",
        [],
    )?;

    Ok(())
}

fn read_records(conn: &Connection, include_deleted: bool) -> rusqlite::Result<Vec<RecordItem>> {
    let query = if include_deleted {
        "SELECT id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date, deleted_at FROM records"
    } else {
        "SELECT id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date, deleted_at FROM records WHERE deleted_at IS NULL"
    };

    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map([], |row| {
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
            deleted_at: row.get(9)?,
        })
    })?;

    rows.collect()
}

fn query_records(conn: &Connection, query: &str) -> rusqlite::Result<Vec<RecordItem>> {
    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map([], |row| {
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
            deleted_at: row.get(9)?,
        })
    })?;

    rows.collect()
}

fn read_visible_records(conn: &Connection) -> rusqlite::Result<Vec<RecordItem>> {
    read_records(conn, false)
}

fn read_sync_records(conn: &Connection) -> rusqlite::Result<Vec<RecordItem>> {
    read_records(conn, true)
}

fn read_dirty_sync_records(conn: &Connection) -> rusqlite::Result<Vec<RecordItem>> {
    query_records(
        conn,
        "SELECT id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date, deleted_at
         FROM records
         WHERE sync_status='dirty'",
    )
}

fn soft_delete_record(conn: &Connection, id: &str) -> rusqlite::Result<usize> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE records SET deleted_at=?1, updated_at=?1, sync_status='dirty' WHERE id=?2",
        params![now, id],
    )
}

fn apply_remote_records_to_db(conn: &Connection, records: &[RecordItem]) -> rusqlite::Result<()> {
    for record in records {
        let local_updated_at: Option<String> = conn
            .query_row(
                "SELECT updated_at FROM records WHERE id=?1",
                params![record.id],
                |row| row.get(0),
            )
            .optional()?;

        match local_updated_at {
            Some(local_updated_at) if local_updated_at >= record.updated_at => {}
            Some(_) => {
                conn.execute(
                    "UPDATE records
                     SET type=?1, content=?2, date=?3, status=?4, created_at=?5, updated_at=?6,
                         completed_at=?7, rolled_over_from_date=?8, deleted_at=?9, sync_status='synced'
                     WHERE id=?10",
                    params![
                        record.record_type,
                        record.content,
                        record.date,
                        record.status,
                        record.created_at,
                        record.updated_at,
                        record.completed_at,
                        record.rolled_over_from_date,
                        record.deleted_at,
                        record.id,
                    ],
                )?;
            }
            None => {
                conn.execute(
                    "INSERT INTO records
                     (id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date, deleted_at, sync_status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'synced')",
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
                        record.deleted_at,
                    ],
                )?;
            }
        }
    }

    Ok(())
}

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
    ensure_sync_columns(conn)?;

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
               AND deleted_at IS NULL
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
            "UPDATE records SET date=?1, status='pending', rolled_over_from_date=?2, updated_at=?3, sync_status='dirty' WHERE id=?4",
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
            rolled_over_from_date TEXT,
            deleted_at TEXT,
            sync_status TEXT NOT NULL DEFAULT 'dirty'
        );",
    )
    .expect("Failed to create table");

    ensure_sync_columns(&conn).expect("Failed to migrate sync columns");
    // Enable WAL mode for better concurrent access
    conn.execute_batch("PRAGMA journal_mode=WAL;").ok();
    normalize_todo_statuses(&conn).expect("Failed to normalize todo statuses");

    conn
}

#[tauri::command]
fn get_all_records(db: State<'_, DbState>) -> Result<Vec<RecordItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    read_visible_records(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_sync_records(db: State<'_, DbState>) -> Result<Vec<RecordItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    read_sync_records(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_dirty_sync_records(db: State<'_, DbState>) -> Result<Vec<RecordItem>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    read_dirty_sync_records(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_record(db: State<'_, DbState>, record: RecordItem) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO records (id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date, deleted_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'dirty')",
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
            record.deleted_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn update_record(db: State<'_, DbState>, record: RecordItem) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE records SET type=?1, content=?2, date=?3, status=?4, updated_at=?5, completed_at=?6, rolled_over_from_date=?7, deleted_at=?8, sync_status='dirty' WHERE id=?9",
        params![
            record.record_type,
            record.content,
            record.date,
            record.status,
            record.updated_at,
            record.completed_at,
            record.rolled_over_from_date,
            record.deleted_at,
            record.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_record(db: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    soft_delete_record(&conn, &id).map_err(|e| e.to_string())?;
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

    conn.execute(
        "UPDATE records SET sync_status='dirty' WHERE id=?1",
        params![id],
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
            "INSERT OR IGNORE INTO records (id, type, content, date, status, created_at, updated_at, completed_at, rolled_over_from_date, deleted_at, sync_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'dirty')",
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
                record.deleted_at,
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    normalize_todo_statuses(&conn).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn apply_remote_records(db: State<'_, DbState>, records: Vec<RecordItem>) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    apply_remote_records_to_db(&conn, &records).map_err(|e| e.to_string())
}

#[tauri::command]
fn mark_records_synced(db: State<'_, DbState>, ids: Vec<String>) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    for id in ids {
        conn.execute(
            "UPDATE records SET sync_status='synced' WHERE id=?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    }
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
                            if should_hide_on_shortcut(&window) {
                                let _ = window.hide();
                            } else {
                                let _ = restore_main_window(&window);
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
            let restore_guard = Arc::new(AtomicBool::new(false));
            app.manage(RestoreGuard(restore_guard.clone()));

            // Snap compact window to top of screen when moved
            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            #[cfg(windows)]
            watch_foreground_minimized_window(window.clone(), restore_guard);

            let flag = compact_flag.clone();
            let w = window.clone();
            window.on_window_event(move |event| {
                match event {
                    tauri::WindowEvent::Focused(true) => {
                        if w.is_minimized().unwrap_or(false) {
                            let _ = restore_main_window(&w);
                        }
                    }
                    tauri::WindowEvent::Moved(_) => {
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
                    _ => {}
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
                            let _ = restore_main_window(&window);
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
                            let _ = restore_main_window(&window);
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
            get_sync_records,
            get_dirty_sync_records,
            add_record,
            update_record,
            delete_record,
            toggle_todo,
            rollover_todos,
            migrate_records,
            apply_remote_records,
            mark_records_synced,
            compact_mode,
            restore_mode,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FrostNote");
}

#[tauri::command]
fn minimize_window(
    window: tauri::Window,
    restore_guard: State<'_, RestoreGuard>,
) -> Result<(), String> {
    restore_guard.0.store(true, Ordering::Relaxed);
    if let Err(error) = window.minimize() {
        restore_guard.0.store(false, Ordering::Relaxed);
        return Err(error.to_string());
    }

    Ok(())
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

        let x = (monitor_pos.x as f64 + monitor_size.width as f64 - window_size.width as f64 - 12.0)
            as i32;
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

    #[test]
    fn migrates_existing_database_with_sync_columns() {
        let conn = test_db();

        ensure_sync_columns(&conn).expect("migrate sync columns");

        let deleted_at_exists =
            column_exists(&conn, "records", "deleted_at").expect("check deleted_at");
        let sync_status_exists =
            column_exists(&conn, "records", "sync_status").expect("check sync_status");
        assert!(deleted_at_exists);
        assert!(sync_status_exists);
    }

    #[test]
    fn soft_deleted_records_are_hidden_from_visible_list() {
        let conn = test_db();
        ensure_sync_columns(&conn).expect("migrate sync columns");
        insert_todo(&conn, "delete-me", "2026-06-11", Some("pending"), None);

        soft_delete_record(&conn, "delete-me").expect("soft delete record");

        let records = read_visible_records(&conn).expect("read visible records");
        assert!(records.iter().all(|record| record.id != "delete-me"));
        let (deleted_at, sync_status): (String, String) = conn
            .query_row(
                "SELECT deleted_at, sync_status FROM records WHERE id='delete-me'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read tombstone");
        assert!(!deleted_at.is_empty());
        assert_eq!(sync_status, "dirty");
    }

    #[test]
    fn remote_newer_record_replaces_local_record() {
        let conn = test_db();
        ensure_sync_columns(&conn).expect("migrate sync columns");
        insert_todo(&conn, "shared", "2026-06-11", Some("pending"), None);

        let remote = RecordItem {
            id: "shared".to_string(),
            record_type: "todo".to_string(),
            content: "remote content".to_string(),
            date: "2026-06-12".to_string(),
            status: Some("done".to_string()),
            created_at: "2026-06-10T00:00:00Z".to_string(),
            updated_at: "2026-06-12T00:00:00Z".to_string(),
            completed_at: Some("2026-06-12T00:00:00Z".to_string()),
            rolled_over_from_date: None,
            deleted_at: None,
        };

        apply_remote_records_to_db(&conn, &[remote]).expect("apply remote records");

        let (content, date, status): (String, String, String) = conn
            .query_row(
                "SELECT content, date, status FROM records WHERE id='shared'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read merged record");
        assert_eq!(content, "remote content");
        assert_eq!(date, "2026-06-12");
        assert_eq!(status, "done");
    }

    #[test]
    fn remote_older_record_does_not_replace_local_record() {
        let conn = test_db();
        ensure_sync_columns(&conn).expect("migrate sync columns");
        insert_todo(&conn, "shared", "2026-06-11", Some("pending"), None);

        let remote = RecordItem {
            id: "shared".to_string(),
            record_type: "todo".to_string(),
            content: "old remote content".to_string(),
            date: "2026-06-09".to_string(),
            status: Some("done".to_string()),
            created_at: "2026-06-09T00:00:00Z".to_string(),
            updated_at: "2026-06-09T00:00:00Z".to_string(),
            completed_at: Some("2026-06-09T00:00:00Z".to_string()),
            rolled_over_from_date: None,
            deleted_at: None,
        };

        apply_remote_records_to_db(&conn, &[remote]).expect("apply remote records");

        let (content, date, status): (String, String, String) = conn
            .query_row(
                "SELECT content, date, status FROM records WHERE id='shared'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read merged record");
        assert_eq!(content, "legacy todo");
        assert_eq!(date, "2026-06-11");
        assert_eq!(status, "pending");
    }

    #[test]
    fn reads_only_dirty_records_for_upload() {
        let conn = test_db();
        ensure_sync_columns(&conn).expect("migrate sync columns");
        insert_todo(&conn, "dirty-record", "2026-06-11", Some("pending"), None);
        insert_todo(&conn, "synced-record", "2026-06-11", Some("pending"), None);
        conn.execute(
            "UPDATE records SET sync_status='synced' WHERE id='synced-record'",
            [],
        )
        .expect("mark synced");

        let records = read_dirty_sync_records(&conn).expect("read dirty records");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "dirty-record");
    }

    #[test]
    fn restore_window_steps_restore_before_show_and_focus() {
        assert_eq!(
            restore_window_steps(),
            [
                WindowRestoreStep::NativeRestore,
                WindowRestoreStep::Show,
                WindowRestoreStep::Focus,
            ]
        );
    }
}
