<script lang="ts">
  import { renderMarkdown } from "@ccs/ui";
  let { text = "" }: { text?: string } = $props();
</script>

<div class="prose prose-sm max-w-none break-words dark:prose-invert">{@html renderMarkdown(text ?? "")}</div>
