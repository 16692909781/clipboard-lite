<script setup lang="ts">
const model = defineModel<string>({ default: "" });

defineEmits<{
  search: [];
}>();
</script>

<template>
  <n-input
    v-model:value="model"
    placeholder="搜索剪贴板历史…"
    clearable
    autofocus
    @update:value="$emit('search')"
  >
    <template #prefix>
      <span class="search-icon">🔍</span>
    </template>
  </n-input>
</template>

<style scoped>
.search-icon {
  opacity: 0.5;
  font-size: 14px;
}
</style>
