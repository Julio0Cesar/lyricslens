import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import SettingsWindow from "./settings/SettingsWindow";
import "./styles.css";

// As duas janelas carregam o mesmo bundle e se distinguem pelo rótulo. O
// overlay nunca monta a árvore de configurações, e vice-versa.
const rotulo = getCurrentWindow().label;
const Root = rotulo === "settings" ? SettingsWindow : App;

document.body.dataset.window = rotulo;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
