<script lang="ts">
  import { Button, applyTheme, resolveInitialTheme, type Theme } from "@ccs/ui";
  let theme = $state<Theme>(resolveInitialTheme());
  function toggle() {
    theme = theme === "dark" ? "light" : "dark";
    applyTheme(theme);
  }
</script>

<header class="flex items-center justify-between border-b border-border px-4 py-3">
  <span class="font-semibold">ccs-daemon</span>
  <Button variant="ghost" onclick={toggle}>{theme === "dark" ? "☀" : "☾"}</Button>
</header>
