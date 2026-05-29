import { describe, it, expect, vi, beforeEach } from "vitest";
import { createClient, parseSseEvent } from "./client";

describe("createClient", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("listSessions hits /api/sessions with limit and returns a bare array", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(JSON.stringify([]), { status: 200 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    const out = await c.listSessions({ limit: 50 });
    expect(fetchMock).toHaveBeenCalledWith("/api/sessions?limit=50", expect.anything());
    expect(Array.isArray(out)).toBe(true);
  });

  it("getSession returns parsed {meta, requests} body", async () => {
    const body = { meta: { session_id: "s1" }, requests: [] };
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(JSON.stringify(body), { status: 200 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    const out = await c.getSession("s1");
    expect(out.meta.session_id).toBe("s1");
  });

  it("throws on non-2xx", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response("nope", { status: 500 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    await expect(c.health()).rejects.toThrow(/500/);
  });
});

describe("parseSseEvent", () => {
  it("extracts JSON data line", () => {
    const ev = parseSseEvent("event: request\ndata: {\"seq\":3}\n");
    expect(ev).toEqual({ event: "request", data: { seq: 3 } });
  });
  it("returns null on heartbeat/comment", () => {
    expect(parseSseEvent(": keepalive\n")).toBeNull();
  });
});
