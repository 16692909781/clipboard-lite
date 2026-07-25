import { defineStore } from "pinia";
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";

const defaultSettings: AppSettings = {
  maxCount: 500,
  hotkey: "Ctrl+Shift+V",
  theme: "system",
  retentionDays: 30,
  ignoredApps: [],
  autostart: false,
  is_first_launch: true,
  is_pinned: false,
  window_x: null,
  window_y: null,
  window_width: 420,
  window_height: 520,
};

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<AppSettings>({ ...defaultSettings });
  const isDark = ref(false);

  async function load() {
    settings.value = await invoke<AppSettings>("get_settings");
    applyTheme(settings.value.theme);
  }

  async function save(partial?: Partial<AppSettings>) {
    if (partial) {
      settings.value = { ...settings.value, ...partial };
    }
    await invoke("save_settings", { settings: settings.value });
  }

  async function completeFirstLaunch() {
    settings.value = await invoke<AppSettings>("complete_first_launch");
  }

  async function setPinned(pinned: boolean) {
    settings.value = await invoke<AppSettings>("set_panel_pinned", { pinned });
    applyTheme(settings.value.theme);
  }

  async function saveWindowPosition(x: number, y: number) {
    settings.value = await invoke<AppSettings>("set_window_position", { x, y });
  }

  async function saveWindowGeometry(x: number, y: number, width: number, height: number) {
    settings.value = await invoke<AppSettings>("set_window_geometry", {
      x,
      y,
      width,
      height,
    });
  }

  function applyTheme(theme: AppSettings["theme"]) {
    if (theme === "dark") {
      isDark.value = true;
    } else if (theme === "light") {
      isDark.value = false;
    } else {
      isDark.value = window.matchMedia("(prefers-color-scheme: dark)").matches;
    }
    document.documentElement.dataset.theme = isDark.value ? "dark" : "light";
  }

  watch(
    () => settings.value.theme,
    (theme) => applyTheme(theme),
  );

  return {
    settings,
    isDark,
    load,
    save,
    completeFirstLaunch,
    setPinned,
    saveWindowPosition,
    saveWindowGeometry,
    applyTheme,
  };
});
