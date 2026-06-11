import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import type { User } from "@supabase/supabase-js";
import { isCloudConfigured, supabase } from "./cloud/supabaseClient";
import { syncRecords, type RecordItem, type RecordType } from "./cloud/sync";
import "./styles.css";

type FilterMode = "all" | "note" | "todo" | "pending" | "done";
type AuthMode = "login" | "signup";
type SyncStatus = "local" | "idle" | "syncing" | "synced" | "error";

const STORAGE_KEY = "label-notes.records.v1";

function formatDate(date: Date) {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function offsetDate(base: string, offset: number) {
  const date = new Date(`${base}T00:00:00`);
  date.setDate(date.getDate() + offset);
  return formatDate(date);
}

function formatShortDate(date: string) {
  return date.slice(5).replace("-", "/");
}

function dateLabel(date: string, today: string) {
  if (date === today) return "今天";
  if (date === offsetDate(today, -1)) return "昨天";
  if (date === offsetDate(today, 1)) return "明天";
  return date;
}

function createId() {
  if (crypto.randomUUID) return crypto.randomUUID();
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

async function loadRecords(today: string): Promise<RecordItem[]> {
  try {
    // Rollover overdue todos on the backend, get all records back
    const records = await invoke<RecordItem[]>("rollover_todos", { today });
    return records;
  } catch (error) {
    console.error("Failed to load records:", error);
    return [];
  }
}

async function migrateLocalStorage(): Promise<boolean> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return false;

    const parsed = (JSON.parse(raw) as Array<Omit<RecordItem, "deletedAt"> & Partial<Pick<RecordItem, "deletedAt">>>)
      .map((record) => ({ ...record, deletedAt: record.deletedAt ?? null }));
    if (parsed.length === 0) return false;

    await invoke("migrate_records", { records: parsed });
    localStorage.removeItem(STORAGE_KEY);
    return true;
  } catch {
    return false;
  }
}

function runWindowCommand(command: "minimize_window" | "toggle_maximize_window" | "close_window") {
  void invoke(command);
}

function App() {
  const [today, setToday] = useState(() => formatDate(new Date()));
  const [selectedDate, setSelectedDate] = useState(today);
  const [records, setRecords] = useState<RecordItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [content, setContent] = useState("");
  const [recordType, setRecordType] = useState<RecordType>("note");
  const [entryDate, setEntryDate] = useState(today);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<FilterMode>("all");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingContent, setEditingContent] = useState("");
  const [compactMode, setCompactMode] = useState(false);
  const [user, setUser] = useState<User | null>(null);
  const [authMode, setAuthMode] = useState<AuthMode>("login");
  const [authEmail, setAuthEmail] = useState("");
  const [authPassword, setAuthPassword] = useState("");
  const [authCustomUid, setAuthCustomUid] = useState("");
  const [authCustomUidMessage, setAuthCustomUidMessage] = useState("");
  const [authMessage, setAuthMessage] = useState("");
  const [syncStatus, setSyncStatus] = useState<SyncStatus>(isCloudConfigured ? "idle" : "local");
  const [syncMessage, setSyncMessage] = useState(isCloudConfigured ? "本地模式" : "云同步未配置");
  const [lastSyncedAt, setLastSyncedAt] = useState<string | null>(null);
  const [syncVersion, setSyncVersion] = useState(0);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const todayRef = useRef(today);

  const requestCloudSync = useCallback(() => {
    setSyncVersion((version) => version + 1);
  }, []);

  const runCloudSync = useCallback(async function runCloudSync() {
    if (!supabase || !user) {
      setSyncStatus(isCloudConfigured ? "idle" : "local");
      return;
    }

    setSyncStatus("syncing");
    setSyncMessage("同步中...");
    try {
      const customUid = (user.user_metadata?.custom_uid as string | undefined) ?? user.id;
      const result = await syncRecords(supabase, customUid, today);
      setRecords(result.records);
      setLastSyncedAt(result.syncedAt);
      setSyncStatus("synced");
      setSyncMessage("已同步");
    } catch (error) {
      console.error("Failed to sync records:", error);
      setSyncStatus("error");
      setSyncMessage("同步失败，可稍后重试");
    }
  }, [today, user]);

  // Load records on mount: migrate from localStorage, then fetch from SQLite
  useEffect(() => {
    let cancelled = false;

    async function init() {
      await migrateLocalStorage();
      const data = await loadRecords(today);
      if (!cancelled) {
        setRecords(data);
        setLoading(false);
      }
    }

    init();

    return () => {
      cancelled = true;
    };
  }, [today]);

  useEffect(() => {
    if (!supabase) return;

    let cancelled = false;
    supabase.auth.getSession().then(({ data }) => {
      if (!cancelled) setUser(data.session?.user ?? null);
    });

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setUser(session?.user ?? null);
    });

    return () => {
      cancelled = true;
      subscription.unsubscribe();
    };
  }, []);

  useEffect(() => {
    if (!user) {
      setSyncStatus(isCloudConfigured ? "idle" : "local");
      return;
    }

    void runCloudSync();
  }, [runCloudSync, user]);

  useEffect(() => {
    if (!user || syncVersion === 0) return;
    void runCloudSync();
  }, [runCloudSync, syncVersion, user]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      const nextToday = formatDate(new Date());
      const previousToday = todayRef.current;
      if (nextToday === previousToday) return;

      todayRef.current = nextToday;
      setToday(nextToday);
      setSelectedDate((current) => (current === previousToday ? nextToday : current));
      setEntryDate((current) => (current === previousToday ? nextToday : current));
    }, 60_000);

    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    setEntryDate(selectedDate);
  }, [selectedDate]);

  const dateOptions = useMemo(() => {
    const uniqueDates = new Set(records.map((record) => record.date));
    // Also include the currently selected date so arrow navigation to empty dates works
    uniqueDates.add(selectedDate);
    return [...uniqueDates].sort((a, b) => b.localeCompare(a));
  }, [records, selectedDate]);

  useEffect(() => {
    if (dateOptions.length > 0 && !dateOptions.includes(selectedDate)) {
      setSelectedDate(dateOptions[0]);
    }
  }, [dateOptions, selectedDate]);

  const selectedRecords = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();

    const filtered = records
      .filter((record) => record.date === selectedDate)
      .filter((record) => {
        if (filter === "note") return record.type === "note";
        if (filter === "todo") return record.type === "todo";
        if (filter === "pending") return record.type === "todo" && record.status === "pending";
        if (filter === "done") return record.type === "todo" && record.status === "done";
        return true;
      })
      .filter((record) => record.content.toLowerCase().includes(normalizedQuery));

    return [...filtered].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  }, [filter, query, records, selectedDate]);

  const todayTodos = useMemo(
    () => records.filter((r) => r.date === selectedDate && r.type === "todo"),
    [records, selectedDate],
  );
  const pendingCount = todayTodos.filter((r) => r.status === "pending").length;
  const totalCount = todayTodos.length;
  const compactRecords = useMemo(
    () => records.filter((record) => record.date === today).sort((a, b) => b.createdAt.localeCompare(a.createdAt)),
    [records, today],
  );

  const clearCompleted = useCallback(async function clearCompleted() {
    const doneIds = todayTodos.filter((r) => r.status === "done").map((r) => r.id);
    if (doneIds.length === 0) return;
    for (const id of doneIds) {
      await invoke("delete_record", { id }).catch((e) => console.error(e));
    }
    setRecords((current) => current.filter((r) => !doneIds.includes(r.id)));
    requestCloudSync();
  }, [requestCloudSync, todayTodos]);

  const addRecord = useCallback(
    async function addRecord(event: React.FormEvent<HTMLFormElement>) {
      event.preventDefault();
      const trimmed = content.trim();
      if (!trimmed) {
        inputRef.current?.focus();
        return;
      }

      const now = new Date().toISOString();
      const record: RecordItem = {
        id: createId(),
        type: recordType,
        content: trimmed,
        date: entryDate,
        status: recordType === "todo" ? "pending" : null,
        createdAt: now,
        updatedAt: now,
        completedAt: null,
        rolledOverFromDate: null,
        deletedAt: null,
      };

      try {
        await invoke("add_record", { record });
        setRecords((current) => [record, ...current]);
        setContent("");
        setSelectedDate(entryDate);
        inputRef.current?.focus();
        requestCloudSync();
      } catch (error) {
        console.error("Failed to add record:", error);
      }
    },
    [content, recordType, entryDate, requestCloudSync],
  );

  const toggleTodo = useCallback(async function toggleTodo(record: RecordItem) {
    try {
      await invoke("toggle_todo", { id: record.id });
      // Reload records to get the updated state
      const today = formatDate(new Date());
      const updated = await loadRecords(today);
      setRecords(updated);
      requestCloudSync();
    } catch (error) {
      console.error("Failed to toggle todo:", error);
    }
  }, [requestCloudSync]);

  const deleteRecord = useCallback(async function deleteRecord(id: string) {
    try {
      await invoke("delete_record", { id });
      setRecords((current) => current.filter((item) => item.id !== id));
      requestCloudSync();
    } catch (error) {
      console.error("Failed to delete record:", error);
    }
  }, [requestCloudSync]);

  const saveEditing = useCallback(
    async function saveEditing(id: string) {
      const trimmed = editingContent.trim();
      if (!trimmed) return;

      const now = new Date().toISOString();

      setRecords((current) => {
        const updated = current.map((item) =>
          item.id === id ? { ...item, content: trimmed, updatedAt: now } : item,
        );
        const record = updated.find((r) => r.id === id);
        if (record) {
          invoke("update_record", { record: { ...record, content: trimmed, updatedAt: now } }).catch(
            (error) => console.error("Failed to update record:", error),
          );
        }
        return updated;
      });

      setEditingId(null);
      setEditingContent("");
      requestCloudSync();
    },
    [editingContent, requestCloudSync],
  );

  function startEditing(record: RecordItem) {
    setEditingId(record.id);
    setEditingContent(record.content);
  }

  async function handleCompactMode() {
    const next = !compactMode;
    setCompactMode(next);
    if (next) {
      await invoke("compact_mode");
    } else {
      await invoke("restore_mode");
    }
  }

  async function handleAuthSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!supabase) {
      setAuthMessage("请先配置 Supabase 环境变量");
      return;
    }

    const email = authEmail.trim();
    if (!email || authPassword.length < 6) {
      setAuthMessage("请输入邮箱和至少 6 位密码");
      return;
    }

    if (authMode === "signup") {
      const customUid = authCustomUid.trim();
      const CUSTOM_UID_RE = /^[a-zA-Z0-9_-]{3,30}$/;
      if (!CUSTOM_UID_RE.test(customUid)) {
        setAuthCustomUidMessage("用户名需 3-30 位，仅限字母、数字、- 和 _");
        return;
      }
      setAuthCustomUidMessage("");
      setAuthMessage("检查用户名可用性...");
      const { data: available, error: checkError } = await supabase
        .rpc("check_custom_uid_available", { uid: customUid });
      if (checkError || !available) {
        setAuthMessage("");
        setAuthCustomUidMessage("该用户名已被使用，请换一个");
        return;
      }
    }

    setAuthMessage(authMode === "login" ? "登录中..." : "注册中...");
    setAuthCustomUidMessage("");
    const result =
      authMode === "login"
        ? await supabase.auth.signInWithPassword({ email, password: authPassword })
        : await supabase.auth.signUp({
            email,
            password: authPassword,
            options: {
              data: { custom_uid: authCustomUid.trim() },
            },
          });

    if (result.error) {
      setAuthMessage(result.error.message);
      return;
    }

    setAuthPassword("");
    setAuthCustomUid("");
    setAuthMessage(authMode === "login" ? "已登录" : "账号已创建");
  }

  async function handleSignOut() {
    if (!supabase) return;
    await supabase.auth.signOut();
    setUser(null);
    setSyncStatus("idle");
    setSyncMessage("本地模式");
  }

  const syncStatusText =
    syncStatus === "syncing"
      ? "同步中"
      : syncStatus === "synced"
        ? "已同步"
        : syncStatus === "error"
          ? "同步失败"
          : syncStatus === "local"
            ? "仅本地"
            : "未登录";

  if (loading) {
    return (
      <main className="app-shell">
        <div className="empty-state" style={{ gridColumn: "1 / -1" }}>
          <p className="empty-title">加载中...</p>
        </div>
      </main>
    );
  }

  return (
    <main className={`app-shell${compactMode ? " compact" : ""}`}>
      <div className="window-chrome">
        <div className="window-drag-zone" data-tauri-drag-region aria-hidden="true" />
        <div className="window-controls" aria-label="窗口控制">
          <button
            aria-label={compactMode ? "还原" : "紧凑模式"}
            className="compact-action"
            onClick={handleCompactMode}
            type="button"
          >
            {compactMode ? "⊠" : "⊡"}
          </button>
          <button aria-label="最小化" onClick={() => runWindowCommand("minimize_window")} type="button">
            −
          </button>
          <button aria-label="最大化或还原" onClick={() => runWindowCommand("toggle_maximize_window")} type="button">
            □
          </button>
          <button aria-label="关闭" className="close-window" onClick={() => runWindowCommand("close_window")} type="button">
            ×
          </button>
        </div>
      </div>

      {!compactMode ? (
        <>
          <aside className="date-rail" aria-label="日期列表">
            <div className="brand">
              <span className="brand-mark" aria-hidden="true" />
              <div>
                <p className="brand-label">霜笺</p>
                <h1>FrostNote</h1>
              </div>
            </div>

            <section className="cloud-panel" aria-label="云同步">
              <div className="cloud-panel-header">
                <span className={`sync-dot ${syncStatus}`} aria-hidden="true" />
                <span>{syncStatusText}</span>
                {user ? (
                  <span className="cloud-copy">
                    {syncMessage}
                    {lastSyncedAt ? ` · ${new Date(lastSyncedAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}` : ""}
                  </span>
                ) : null}
              </div>
              {!isCloudConfigured ? (
                <p className="cloud-copy">当前记录仅保存在本机。</p>
              ) : user ? (
                <>
                  <div className="cloud-user-row">
                    <p className="cloud-user" title={user.email ?? ""}>
                      @{(user.user_metadata?.custom_uid as string | undefined) ?? user.id.slice(0, 8)}
                    </p>
                    <div className="cloud-actions">
                      <button onClick={runCloudSync} type="button" disabled={syncStatus === "syncing"}>
                        同步
                      </button>
                      <button onClick={handleSignOut} type="button">
                        退出
                      </button>
                    </div>
                  </div>
                </>
              ) : (
                <form className="auth-form" onSubmit={handleAuthSubmit}>
                  <div className="auth-tabs" aria-label="账号操作">
                    <button
                      className={authMode === "login" ? "selected" : ""}
                      onClick={() => setAuthMode("login")}
                      type="button"
                    >
                      登录
                    </button>
                    <button
                      className={authMode === "signup" ? "selected" : ""}
                      onClick={() => setAuthMode("signup")}
                      type="button"
                    >
                      注册
                    </button>
                  </div>
                  <input
                    aria-label="邮箱"
                    autoComplete="email"
                    onChange={(event) => setAuthEmail(event.target.value)}
                    placeholder="邮箱"
                    type="email"
                    value={authEmail}
                  />
                  {authMode === "signup" ? (
                    <input
                      aria-label="用户名"
                      autoComplete="username"
                      onChange={(event) => setAuthCustomUid(event.target.value)}
                      placeholder="用户名（3-30位字母、数字、- 和 _）"
                      type="text"
                      value={authCustomUid}
                    />
                  ) : null}
                  {authCustomUidMessage ? <p className="auth-message">{authCustomUidMessage}</p> : null}
                  <input
                    aria-label="密码"
                    autoComplete={authMode === "login" ? "current-password" : "new-password"}
                    onChange={(event) => setAuthPassword(event.target.value)}
                    placeholder="密码"
                    type="password"
                    value={authPassword}
                  />
                  <button className="auth-submit" type="submit">
                    {authMode === "login" ? "登录同步" : "创建账号"}
                  </button>
                  {authMessage ? <p className="auth-message">{authMessage}</p> : null}
                </form>
              )}
            </section>

            <nav className="date-list" aria-label="最近日期">
              {dateOptions.length > 0 ? (
                dateOptions.map((date) => (
                  <button
                    className={`date-item ${date === selectedDate ? "active" : ""}`}
                    key={date}
                    onClick={() => setSelectedDate(date)}
                    type="button"
                  >
                    {dateLabel(date, today)}
                    <span>{formatShortDate(date)}</span>
                  </button>
                ))
              ) : (
                <p className="empty-dates">暂无记录日期</p>
              )}
            </nav>
          </aside>

          <section className="workspace" aria-label="当前日期记录">
            <header className="workspace-header">
              <div>
                <p className="eyebrow">{dateLabel(selectedDate, today)} · {selectedDate}</p>
                <h2>
                  {totalCount > 0
                    ? `待完成 ${pendingCount} / 共 ${totalCount}`
                    : "今日宜休息"}
                </h2>
              </div>
              <div className="date-nav">
                <button
                  className="date-nav-btn"
                  onClick={() => setSelectedDate(offsetDate(selectedDate, -1))}
                  type="button"
                  title="前一天"
                >
                  ←
                </button>
                <button
                  className={`date-nav-btn${selectedDate === today ? " today" : ""}`}
                  onClick={() => setSelectedDate(today)}
                  type="button"
                  title="回到今天"
                >
                  今天
                </button>
                <button
                  className="date-nav-btn"
                  onClick={() => setSelectedDate(offsetDate(selectedDate, 1))}
                  type="button"
                  title="后一天"
                >
                  →
                </button>
              </div>
            </header>

            <form className="quick-input" onSubmit={addRecord}>
              <div className="entry-toolbar">
                <div className="segmented" aria-label="记录类型">
                  <button
                    className={recordType === "note" ? "selected" : ""}
                    onClick={() => setRecordType("note")}
                    type="button"
                  >
                    FreeNote
                  </button>
                  <button
                    className={recordType === "todo" ? "selected" : ""}
                    onClick={() => setRecordType("todo")}
                    type="button"
                  >
                    To do
                  </button>
                </div>
                <input
                  aria-label="记录日期"
                  className="date-input"
                  onChange={(event) => setEntryDate(event.target.value)}
                  type="date"
                  value={entryDate}
                />
              </div>
              <textarea
                ref={inputRef}
                onChange={(event) => setContent(event.target.value)}
                placeholder="写下想做的事，或一条临时记录..."
                rows={1}
                value={content}
              />
              <div className="entry-actions">
                <span>{recordType === "todo" ? "会以未完成事项保存" : "会以 FreeNote 保存"}</span>
                <button className="primary-action" type="submit">
                  保存
                </button>
              </div>
            </form>

            <div className="controls">
              <input
                aria-label="搜索记录"
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索..."
                type="search"
                value={query}
              />
              <select aria-label="筛选记录" onChange={(event) => setFilter(event.target.value as FilterMode)} value={filter}>
                <option value="all">全部</option>
                <option value="note">FreeNote</option>
                <option value="todo">To do</option>
                <option value="pending">未完成</option>
                <option value="done">已完成</option>
              </select>
              {todayTodos.some((r) => r.status === "done") ? (
                <button
                  className="clear-done"
                  onClick={clearCompleted}
                  type="button"
                  title="删除当天所有已完成事项"
                >
                  清除已完成
                </button>
              ) : null}
            </div>

            <div className="records-panel" aria-label="记录列表">
              {selectedRecords.length === 0 ? (
                <div className="empty-state">
                  <div>
                    <p className="empty-title">这一天还没有记录</p>
                    <p className="empty-copy">直接在输入框里写点什么，保存后日期会出现在左栏。</p>
                  </div>
                </div>
              ) : (
                <div className="record-list">
                  {selectedRecords.map((record) => {
                    const isDone = record.type === "todo" && record.status === "done";
                    const isOverdue = record.type === "todo" && record.status === "pending" && record.rolledOverFromDate;

                    return (
                      <article className={`record-card ${isDone ? "done" : ""} ${isOverdue ? "overdue" : ""}`} key={record.id}>
                        <div className="record-main">
                          <div className="record-meta">
                            <span className="record-type">{record.type === "todo" ? "To do" : "记录"}</span>
                            {isDone && record.completedAt ? (
                              <time className="completed-time">
                                {new Date(record.completedAt).toLocaleString("zh-CN", {
                                  month: "numeric",
                                  day: "numeric",
                                  hour: "2-digit",
                                  minute: "2-digit",
                                })}
                              </time>
                            ) : null}
                            {isOverdue ? <span className="overdue-label">来自 {record.rolledOverFromDate}</span> : null}
                          </div>
                          {editingId === record.id ? (
                            <textarea
                              className="edit-input"
                              onChange={(event) => setEditingContent(event.target.value)}
                              rows={2}
                              value={editingContent}
                            />
                          ) : (
                            <>
                              <p>{record.content}</p>
                            </>
                          )}
                        </div>
                        <div className="record-actions">
                          {record.type === "todo" ? (
                            <button
                              aria-label={isDone ? "标记为未完成" : "标记为完成"}
                              className={`check-action ${isDone ? "checked" : ""}`}
                              onClick={() => toggleTodo(record)}
                              type="button"
                            >
                              ✓
                            </button>
                          ) : null}
                          {editingId === record.id ? (
                            <button onClick={() => saveEditing(record.id)} type="button">
                              保存
                            </button>
                          ) : (
                            <button onClick={() => startEditing(record)} type="button">
                              编辑
                            </button>
                          )}
                          <button className="danger-action" onClick={() => deleteRecord(record.id)} type="button">
                            删除
                          </button>
                        </div>
                      </article>
                    );
                  })}
                </div>
              )}
            </div>
          </section>
        </>
      ) : (
        <div className="compact-panel">
          <div className="compact-header">
            <div className="brand">
              <span className="brand-mark" aria-hidden="true" />
              <div>
                <p className="brand-label">霜笺</p>
                <h1>FrostNote</h1>
              </div>
            </div>
          </div>

          <p className="compact-date">{dateLabel(today, today)} · {today}</p>

          <div className="compact-records">
            {compactRecords.length === 0 ? (
              <p className="compact-empty">今天还没有记录</p>
            ) : (
              compactRecords.map((record) => {
                  const isDone = record.type === "todo" && record.status === "done";
                  const isOverdue = record.type === "todo" && record.status === "pending" && record.rolledOverFromDate;

                  return (
                    <article className={`record-card ${isDone ? "done" : ""} ${isOverdue ? "overdue" : ""}`} key={record.id}>
                      <div className="record-main">
                        <div className="record-meta compact-record-meta">
                          <span className="record-type">{record.type === "todo" ? "To do" : "记录"}</span>
                          {isDone && record.completedAt ? (
                            <time className="completed-time compact-completed-time">
                              {new Date(record.completedAt).toLocaleString("zh-CN", {
                                month: "numeric",
                                day: "numeric",
                                hour: "2-digit",
                                minute: "2-digit",
                              })}
                            </time>
                          ) : null}
                          {isOverdue ? <span className="overdue-label">来自 {record.rolledOverFromDate}</span> : null}
                        </div>
                        <p>{record.content}</p>
                      </div>
                      {record.type === "todo" ? (
                        <div className="record-actions">
                          <button
                            aria-label={isDone ? "标记为未完成" : "标记为完成"}
                            className={`check-action ${isDone ? "checked" : ""}`}
                            onClick={() => toggleTodo(record)}
                            type="button"
                          >
                            ✓
                          </button>
                        </div>
                      ) : null}
                    </article>
                  );
                })
            )}
          </div>
        </div>
      )}
    </main>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
