import { describe, it, expect } from "vitest";
import { statusVariant } from "./status";

describe("statusVariant", () => {
  it("maps 2xx to success", () => expect(statusVariant(200)).toBe("success"));
  it("maps 4xx to warning", () => expect(statusVariant(404)).toBe("warning"));
  it("maps 5xx to danger", () => expect(statusVariant(500)).toBe("danger"));
  it("maps null/pending to muted", () => expect(statusVariant(null)).toBe("muted"));
});
