<script setup lang="ts">
import { computed } from "vue";
import type { ClipRecord } from "../types";
import { formatRelativeTime, clipTypeLabel, truncate } from "../utils/format";

const props = defineProps<{
  clip: ClipRecord;
  selected?: boolean;
}>();

defineEmits<{
  select: [];
  paste: [];
  delete: [];
  favorite: [];
  context: [event: MouseEvent];
}>();

const preview = computed(() => truncate(props.clip.preview));
</script>

<template>
  <div
    class="clip-item"
    :class="{ selected }"
    @click="$emit('select')"
    @dblclick="$emit('paste')"
    @contextmenu.prevent="$emit('context', $event)"
  >
    <div class="clip-meta">
      <span class="clip-type">{{ clipTypeLabel(clip.clipType) }}</span>
      <span v-if="clip.isFavorite" class="clip-star">★</span>
      <span class="clip-time">{{ formatRelativeTime(clip.createdAt) }}</span>
    </div>
    <div class="clip-preview">{{ preview }}</div>
  </div>
</template>

<style scoped>
.clip-item {
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s;
}

.clip-item:hover,
.clip-item.selected {
  background: var(--clip-hover-bg);
}

.clip-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.clip-star {
  color: #f5a623;
}

.clip-preview {
  font-size: 13px;
  line-height: 1.4;
  word-break: break-all;
  color: var(--text-primary);
}
</style>
