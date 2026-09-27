import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import { SessionProvider } from "./state/SessionContext";
import { SettingsProvider } from "./state/SettingsContext";
import "./index.css";

// 屏蔽网页右键菜单（刷新/另存为/打印等）
document.addEventListener("contextmenu", (event) => event.preventDefault());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <SettingsProvider>
      <SessionProvider>
        <App />
      </SessionProvider>
    </SettingsProvider>
  </React.StrictMode>,
);
