import "normalize.css";
import "./global.sass";
import { mount } from "svelte";
import App from "./App.svelte";
import { applyTheme, storedTheme } from "./lib/theme.js";

applyTheme(storedTheme());

export default mount(App, { target: document.getElementById("app") });
