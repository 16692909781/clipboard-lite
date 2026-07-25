<script setup lang="ts">
import { useSettingsStore } from "../stores/settings";

const settingsStore = useSettingsStore();

function setTheme(theme: "light" | "dark" | "system") {
  settingsStore.settings.theme = theme;
  settingsStore.applyTheme(theme);
}
</script>

<template>
  <n-radio-group
    :value="settingsStore.settings.theme"
    size="small"
    @update:value="setTheme"
  >
    <n-radio-button value="light">浅色</n-radio-button>
    <n-radio-button value="dark">深色</n-radio-button>
    <n-radio-button value="system">跟随系统</n-radio-button>
  </n-radio-group>
</template>
