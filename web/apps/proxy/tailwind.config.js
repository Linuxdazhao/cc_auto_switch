import preset from "@ccs/ui/tailwind.preset";
export default {
  presets: [preset],
  content: [
    "./index.html",
    "./src/**/*.{svelte,ts}",
    "../../packages/ui/src/**/*.{svelte,ts}",
  ],
};
