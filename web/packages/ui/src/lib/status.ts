export type StatusVariant = "success" | "warning" | "danger" | "muted";

export function statusVariant(status: number | null | undefined): StatusVariant {
  if (status == null) return "muted";
  if (status >= 500) return "danger";
  if (status >= 400) return "warning";
  if (status >= 200) return "success";
  return "muted";
}
