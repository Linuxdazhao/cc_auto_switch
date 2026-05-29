import { describe, it, expect, vi, beforeEach } from "vitest";
import { createClient } from "./client";

describe("createClient", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("listSessions hits /api/sessions with limit", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(JSON.stringify({ items: [] }), { status: 200 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    await c.listSessions({ limit: 50 });
    expect(fetchMock).toHaveBeenCalledWith("/api/sessions?limit=50", expect.anything());
  });

  it("getSession returns parsed body", async () => {
    const body = { session_id: "s1", requests: [] };
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(JSON.stringify(body), { status: 200 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    const out = await c.getSession("s1");
    expect(out.session_id).toBe("s1");
  });

  it("throws on non-2xx", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response("nope", { status: 500 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    await expect(c.health()).rejects.toThrow(/500/);
  });
});
