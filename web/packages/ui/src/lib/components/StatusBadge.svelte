<script lang="ts">
  import { statusVariant } from "../status";
  import { cn } from "../utils";
  let { status }: { status: number | null | undefined } = $props();
  const cls: Record<string, string> = {
    success: "bg-success/15 text-success",
    warning: "bg-warning/15 text-warning",
    danger: "bg-danger/15 text-danger",
    muted: "bg-muted text-muted-foreground",
  };
</script>

<span class={cn("inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium tabular-nums", cls[statusVariant(status)])}>
  {status ?? "···"}
</span>
