import { describe, it, expect, beforeEach } from "vitest";
import { applyTheme, resolveInitialTheme } from "./theme";

describe("theme", () => {
  beforeEach(() => {
    document.documentElement.className = "";
    localStorage.clear();
  });

  it("applyTheme('dark') adds .dark and persists", () => {
    applyTheme("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(localStorage.getItem("ccs-theme")).toBe("dark");
  });

  it("applyTheme('light') removes .dark", () => {
    document.documentElement.classList.add("dark");
    applyTheme("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("resolveInitialTheme reads persisted value", () => {
    localStorage.setItem("ccs-theme", "dark");
    expect(resolveInitialTheme()).toBe("dark");
  });
});
