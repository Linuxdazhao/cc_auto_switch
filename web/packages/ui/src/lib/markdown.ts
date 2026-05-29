import { marked } from "marked";
import DOMPurify from "dompurify";

export function renderMarkdown(src: string): string {
  const raw = marked.parse(src, { async: false }) as string;
  return DOMPurify.sanitize(raw);
}
