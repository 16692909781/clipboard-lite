<script setup lang="ts">
import { onMounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useClipboardStore } from "../stores/clipboard";
import ClipItem from "../components/ClipItem.vue";

const store = useClipboardStore();

onMounted(() => store.fetchClips());

async function onPaste(id: number) {
  await store.pasteClip(id);
  await getCurrentWindow().hide();
}
</script>

<template>
  <div class="favorites-page">
    <header class="page-header">
      <h2>收藏夹</h2>
      <n-button text @click="$router.push('/')">返回</n-button>
    </header>

    <div class="clip-list">
      <ClipItem
        v-for="clip in store.favorites"
        :key="clip.id"
        :clip="clip"
        @paste="onPaste(clip.id)"
        @favorite="store.toggleFavorite(clip.id)"
        @delete="store.deleteClip(clip.id)"
      />
      <div v-if="store.favorites.length === 0" class="empty">暂无收藏</div>
    </div>
  </div>
</template>

<style scoped>
.favorites-page {
  padding: 12px;
  height: 100vh;
  box-sizing: border-box;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.page-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.empty {
  text-align: center;
  padding: 32px;
  color: var(--text-secondary);
}
</style>
