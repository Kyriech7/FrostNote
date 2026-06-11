import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import { cloudRowToRecord, recordToCloudRow, syncRecords, type CloudRecordRow, type RecordItem } from "./sync";

describe("Supabase record mapping", () => {
  it("maps local records to Supabase rows with user ownership and tombstone fields", () => {
    const row = recordToCloudRow(
      {
        id: "record-1",
        type: "todo",
        content: "同步测试",
        date: "2026-06-11",
        status: "pending",
        createdAt: "2026-06-11T00:00:00.000Z",
        updatedAt: "2026-06-11T01:00:00.000Z",
        completedAt: null,
        rolledOverFromDate: "2026-06-10",
        deletedAt: "2026-06-11T02:00:00.000Z",
      },
      "user-1",
    );

    expect(row).toEqual({
      id: "record-1",
      user_id: "user-1",
      type: "todo",
      content: "同步测试",
      date: "2026-06-11",
      status: "pending",
      created_at: "2026-06-11T00:00:00.000Z",
      updated_at: "2026-06-11T01:00:00.000Z",
      completed_at: null,
      rolled_over_from_date: "2026-06-10",
      deleted_at: "2026-06-11T02:00:00.000Z",
    });
  });

  it("maps Supabase rows back to local records", () => {
    const record = cloudRowToRecord({
      id: "record-1",
      user_id: "user-1",
      type: "note",
      content: "云端记录",
      date: "2026-06-11",
      status: null,
      created_at: "2026-06-11T00:00:00.000Z",
      updated_at: "2026-06-11T01:00:00.000Z",
      completed_at: null,
      rolled_over_from_date: null,
      deleted_at: null,
    });

    expect(record).toEqual({
      id: "record-1",
      type: "note",
      content: "云端记录",
      date: "2026-06-11",
      status: null,
      createdAt: "2026-06-11T00:00:00.000Z",
      updatedAt: "2026-06-11T01:00:00.000Z",
      completedAt: null,
      rolledOverFromDate: null,
      deletedAt: null,
    });
  });

  it("uploads only dirty local records after applying remote changes", async () => {
    const remoteRow: CloudRecordRow = {
      id: "remote-record",
      user_id: "user-1",
      type: "note",
      content: "remote",
      date: "2026-06-11",
      status: null,
      created_at: "2026-06-11T00:00:00.000Z",
      updated_at: "2026-06-11T00:00:00.000Z",
      completed_at: null,
      rolled_over_from_date: null,
      deleted_at: null,
    };
    const dirtyRecord: RecordItem = {
      id: "dirty-record",
      type: "todo",
      content: "dirty",
      date: "2026-06-11",
      status: "pending",
      createdAt: "2026-06-11T00:00:00.000Z",
      updatedAt: "2026-06-11T01:00:00.000Z",
      completedAt: null,
      rolledOverFromDate: null,
      deletedAt: null,
    };
    const visibleRecords = [dirtyRecord, cloudRowToRecord(remoteRow)];
    const eq = vi.fn().mockResolvedValue({ data: [remoteRow], error: null });
    const select = vi.fn().mockReturnValue({ eq });
    const upsert = vi.fn().mockResolvedValue({ error: null });
    const supabase = {
      from: vi.fn().mockReturnValue({ select, upsert }),
    };

    invokeMock.mockImplementation((command: string) => {
      if (command === "apply_remote_records") return Promise.resolve();
      if (command === "get_dirty_sync_records") return Promise.resolve([dirtyRecord]);
      if (command === "mark_records_synced") return Promise.resolve();
      if (command === "rollover_todos") return Promise.resolve(visibleRecords);
      throw new Error(`unexpected command: ${command}`);
    });

    const result = await syncRecords(supabase as never, "user-1", "2026-06-11");

    expect(result.records).toEqual(visibleRecords);
    expect(invokeMock).toHaveBeenCalledWith("get_dirty_sync_records");
    expect(upsert).toHaveBeenCalledWith([recordToCloudRow(dirtyRecord, "user-1")], { onConflict: "id" });
    expect(invokeMock).toHaveBeenCalledWith("mark_records_synced", { ids: ["dirty-record"] });
  });
});

beforeEach(() => {
  invokeMock.mockReset();
});
