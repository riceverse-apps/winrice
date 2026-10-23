import { createApp, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "./api";
import { createRouter, createWebHashHistory } from "vue-router";
import App from "./App.vue";
import { i18n, setLocale } from "./i18n";
import { initAppearance } from "./composables/appearance";
import "./styles/tokens.css";
import "./styles/base.css";

const pages = ["home", "tools", "themes", "profiles", "history", "settings"] as const;

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: () => import("./views/HomeView.vue") },
    { path: "/tools", name: "tools", component: () => import("./views/ToolsView.vue") },
    { path: "/tools/:id", name: "tool", component: () => import("./views/ToolDetailView.vue") },
    { path: "/themes", name: "themes", component: () => import("./views/ThemesView.vue") },
    { path: "/themes/:id", name: "theme", component: () => import("./views/ThemeDetailView.vue") },
    { path: "/profiles", name: "profiles", component: () => import("./views/ProfilesView.vue") },
    { path: "/widget", name: "widget", component: () => import("./views/WidgetView.vue") },
    { path: "/history", name: "history", component: () => import("./views/HistoryView.vue") },
    { path: "/settings", name: "settings", component: () => import("./views/SettingsView.vue") },
  ],
});

initAppearance();
setLocale(i18n.global.locale.value);

// Widget windows (tray flyout, floating widget) are the same app opened on another route.
let label = "main";
try {
  label = getCurrentWindow().label;
} catch {
  /* running in a plain browser */
}
if (label !== "main") router.replace("/widget");

// The tray menu lives in Rust, so it gets its text from the UI language.
if (label === "main") {
  const labels = () => api.setTrayLabels(i18n.global.t("widget.tray.open"), i18n.global.t("widget.tray.widget"), i18n.global.t("widget.tray.quit")).catch(() => {});
  labels();
  watch(i18n.global.locale, labels);
}
createApp(App, { pages }).use(router).use(i18n).mount("#app");
