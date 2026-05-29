import { describe, it, expect } from "vitest";
import { renderMarkdown } from "./markdown";

describe("renderMarkdown", () => {
  it("renders bold", () => {
    expect(renderMarkdown("**hi**")).toContain("<strong>hi</strong>");
  });
  it("sanitizes script tags", () => {
    expect(renderMarkdown("<script>alert(1)</script>")).not.toContain("<script>");
  });
});
