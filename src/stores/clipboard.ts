import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ClipRecord } from "../types";

export const useClipboardStore = defineStore("clipboard", () => {
  const clips = ref<ClipRecord[]>([]);
  const searchQuery = ref("");
  const selectedIndex = ref(0);
  const loading = ref(false);
  let unlistenClipAdded: (() => void) | null = null;

  const filteredClips = computed(() => {
    const q = searchQuery.value.trim().toLowerCase();
    if (!q) return clips.value;
    return clips.value.filter((c) => c.preview.toLowerCase().includes(q));
  });

  const favorites = computed(() => clips.value.filter((c) => c.isFavorite));

  async function fetchClips() {
    loading.value = true;
    try {
      clips.value = await invoke<ClipRecord[]>("get_clips", {
        limit: 100,
        offset: 0,
        search: searchQuery.value || null,
      });
      selectedIndex.value = 0;
    } finally {
      loading.value = false;
    }
  }

  async function deleteClip(id: number) {
    await invoke("delete_clip", { id });
    clips.value = clips.value.filter((c) => c.id !== id);
  }

  async function toggleFavorite(id: number) {
    const isFavorite = await invoke<boolean>("toggle_favorite", { id });
    const clip = clips.value.find((c) => c.id === id);
    if (clip) clip.isFavorite = isFavorite;
    clips.value = [...clips.value].sort(sortClips);
  }

  async function pasteClip(id: number) {
    await invoke("paste_clip", { id });
  }

  async function clearAll() {
    await invoke("clear_all_clips");
    clips.value = [];
  }

  function selectNext() {
    if (filteredClips.value.length === 0) return;
    selectedIndex.value = (selectedIndex.value + 1) % filteredClips.value.length;
  }

  function selectPrev() {
    if (filteredClips.value.length === 0) return;
    selectedIndex.value =
      (selectedIndex.value - 1 + filteredClips.value.length) % filteredClips.value.length;
  }

  async function initListeners() {
    if (unlistenClipAdded) return;
    unlistenClipAdded = await listen<ClipRecord>("clip-added", (event) => {
      const incoming = event.payload;
      clips.value = [incoming, ...clips.value.filter((clip) => clip.id !== incoming.id)]
        .sort(sortClips)
        .slice(0, 100);
    });
  }

  return {
    clips,
    searchQuery,
    selectedIndex,
    loading,
    filteredClips,
    favorites,
    fetchClips,
    deleteClip,
    toggleFavorite,
    pasteClip,
    clearAll,
    selectNext,
    selectPrev,
    initListeners,
  };
});

function sortClips(a: ClipRecord, b: ClipRecord) {
  if (a.isFavorite !== b.isFavorite) return a.isFavorite ? -1 : 1;
  return b.createdAt - a.createdAt;
}
