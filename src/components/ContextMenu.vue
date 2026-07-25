<script setup lang="ts">
defineProps<{
  x: number;
  y: number;
  visible: boolean;
  favoriteLabel?: string;
}>();

defineEmits<{
  favorite: [];
  delete: [];
  close: [];
}>();
</script>

<template>
  <div
    v-if="visible"
    class="context-menu"
    :style="{ top: `${y}px`, left: `${x}px` }"
    @click.stop
  >
    <button @click="$emit('favorite')">{{ favoriteLabel || "收藏" }}</button>
    <button class="danger" @click="$emit('delete')">删除</button>
  </div>
</template>

<style scoped>
.context-menu {
  position: fixed;
  z-index: 1000;
  background: var(--panel-bg);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 4px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
}

.context-menu button {
  display: block;
  width: 100%;
  padding: 8px 16px;
  border: none;
  background: none;
  text-align: left;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
  border-radius: 4px;
}

.context-menu button:hover {
  background: var(--clip-hover-bg);
}

.context-menu button.danger {
  color: #c2410c;
}
</style>
