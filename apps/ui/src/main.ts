import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { initializeTheme } from "./design-system/theme";

const cleanupTheme = initializeTheme();
if (import.meta.hot) import.meta.hot.dispose(cleanupTheme);
if (import.meta.env.DEV) import("virtual:stylex:runtime");

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
