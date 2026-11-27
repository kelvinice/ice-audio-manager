import { mount } from "svelte";
// Figtree (OFL-1.1), bundled with the app: no font is fetched from the network (CSP allows only self).
import "@fontsource-variable/figtree";
import "./palettes.css";
import "./app.css";
import App from "./App.svelte";

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
