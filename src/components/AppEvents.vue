<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { useMessage } from "naive-ui";

const message = useMessage();
let unlistenWarning: (() => void) | null = null;

onMounted(async () => {
  unlistenWarning = await listen<string>("app-warning", (event) => {
    message.warning(event.payload, { duration: 3500 });
  });
});

onUnmounted(() => {
  unlistenWarning?.();
});
</script>

<template></template>
