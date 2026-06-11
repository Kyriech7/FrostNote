import type { SupabaseClient } from "@supabase/supabase-js";
import { invoke } from "@tauri-apps/api/core";

export type RecordType = "note" | "todo";
export type TodoStatus = "pending" | "done";

export type RecordItem = {
  id: string;
  type: RecordType;
  content: string;
  date: string;
  status: TodoStatus | null;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  rolledOverFromDate: string | null;
  deletedAt: string | null;
};

export type CloudRecordRow = {
  id: string;
  user_id: string;
  type: RecordType;
  content: string;
  date: string;
  status: TodoStatus | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  rolled_over_from_date: string | null;
  deleted_at: string | null;
};

export type SyncResult = {
  records: RecordItem[];
  syncedAt: string;
};

export function recordToCloudRow(record: RecordItem, userId: string): CloudRecordRow {
  return {
    id: record.id,
    user_id: userId,
    type: record.type,
    content: record.content,
    date: record.date,
    status: record.status,
    created_at: record.createdAt,
    updated_at: record.updatedAt,
    completed_at: record.completedAt,
    rolled_over_from_date: record.rolledOverFromDate,
    deleted_at: record.deletedAt,
  };
}

export function cloudRowToRecord(row: CloudRecordRow): RecordItem {
  return {
    id: row.id,
    type: row.type,
    content: row.content,
    date: row.date,
    status: row.status,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    completedAt: row.completed_at,
    rolledOverFromDate: row.rolled_over_from_date,
    deletedAt: row.deleted_at,
  };
}

export async function syncRecords(
  supabase: SupabaseClient,
  userId: string,
  today: string,
): Promise<SyncResult> {
  const { data: remoteRows, error: fetchError } = await supabase
    .from("records")
    .select("*")
    .eq("user_id", userId);

  if (fetchError) throw fetchError;

  const remoteRecords = ((remoteRows ?? []) as CloudRecordRow[]).map(cloudRowToRecord);
  await invoke("apply_remote_records", { records: remoteRecords });

  const dirtyRecords = await invoke<RecordItem[]>("get_dirty_sync_records");
  if (dirtyRecords.length > 0) {
    const rows = dirtyRecords.map((record) => recordToCloudRow(record, userId));
    const { error: upsertError } = await supabase.from("records").upsert(rows, { onConflict: "id" });
    if (upsertError) throw upsertError;
    await invoke("mark_records_synced", { ids: dirtyRecords.map((record) => record.id) });
  }

  const records = await invoke<RecordItem[]>("rollover_todos", { today });
  return {
    records,
    syncedAt: new Date().toISOString(),
  };
}
