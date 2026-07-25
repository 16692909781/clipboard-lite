<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { darkTheme, NConfigProvider, NMessageProvider, NDialogProvider } from "naive-ui";
import { useSettingsStore } from "./stores/settings";
import AppEvents from "./components/AppEvents.vue";

const settingsStore = useSettingsStore();
const router = useRouter();

const theme = computed(() => (settingsStore.isDark ? darkTheme : null));
let unlistenNavigate: (() => void) | null = null;

function hideOnBlur() {
  if (settingsStore.settings.is_pinned) return;
  getCurrentWindow().hide();
}

onMounted(async () => {
  await settingsStore.load();
  await nextTick();

  if (settingsStore.settings.is_first_launch) {
    await settingsStore.completeFirstLaunch();
  }

  unlistenNavigate = await listen<string>("navigate", (event) => router.push(event.payload));
  window.addEventListener("blur", hideOnBlur);
});

onUnmounted(() => {
  unlistenNavigate?.();
  window.removeEventListener("blur", hideOnBlur);
});
</script>

<template>
  <NConfigProvider :theme="theme">
    <NMessageProvider>
      <NDialogProvider>
        <AppEvents />
        <div class="app-shell">
          <router-view />
        </div>
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style scoped>
.app-shell {
  height: 100vh;
  background: var(--panel-bg);
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid var(--border-color);
}
</style>
