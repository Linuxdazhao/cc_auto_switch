import { describe, it, expect } from "vitest";
import { sortRows } from "./sort";

describe("sortRows", () => {
  const rows = [{ n: 3 }, { n: 1 }, { n: 2 }];
  it("sorts ascending", () => {
    expect(sortRows(rows, "n", "asc").map((r) => r.n)).toEqual([1, 2, 3]);
  });
  it("sorts descending", () => {
    expect(sortRows(rows, "n", "desc").map((r) => r.n)).toEqual([3, 2, 1]);
  });
  it("nulls sort last", () => {
    const r = [{ n: 2 }, { n: null }, { n: 1 }];
    expect(sortRows(r, "n", "asc").map((x) => x.n)).toEqual([1, 2, null]);
  });
});
