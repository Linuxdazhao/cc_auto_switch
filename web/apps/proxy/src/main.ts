import { mount } from "svelte";
import "@ccs/ui/app.css";
import { applyTheme, resolveInitialTheme } from "@ccs/ui";
import App from "./App.svelte";

applyTheme(resolveInitialTheme());
const app = mount(App, { target: document.getElementById("app")! });
export default app;
