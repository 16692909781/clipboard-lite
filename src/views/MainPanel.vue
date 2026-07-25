<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch } from "vue";
import { useRouter } from "vue-router";
import {
  availableMonitors,
  getCurrentWindow,
  PhysicalPosition,
  PhysicalSize,
} from "@tauri-apps/api/window";
import { useClipboardStore } from "../stores/clipboard";
import { useSettingsStore } from "../stores/settings";
import { useKeyboardNavigation } from "../utils/keyboard";
import ClipItem from "../components/ClipItem.vue";
import SearchBar from "../components/SearchBar.vue";
import ContextMenu from "../components/ContextMenu.vue";

const router = useRouter();
const store = useClipboardStore();
const settingsStore = useSettingsStore();

const contextMenu = ref({ visible: false, x: 0, y: 0, clipId: 0 });
const contextClip = computed(() =>
  store.clips.find((clip) => clip.id === contextMenu.value.clipId),
);
let searchTimer: number | undefined;

const DEFAULT_WINDOW_WIDTH = 420;
const DEFAULT_WINDOW_HEIGHT = 520;
const MIN_WINDOW_WIDTH = 320;
const WINDOW_ASPECT = DEFAULT_WINDOW_HEIGHT / DEFAULT_WINDOW_WIDTH;

type WindowGeometry = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type ScreenBounds = {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
};

type WindowInteraction = {
  mode: "drag" | "resize";
  pointerId: number;
  target: HTMLElement;
  startMouseX: number;
  startMouseY: number;
  startGeometry: WindowGeometry;
  latestGeometry: WindowGeometry;
  scaleFactor: number;
  bounds: ScreenBounds | null;
  rafId: number | null;
};

let windowInteraction: WindowInteraction | null = null;

onMounted(async () => {
  await store.fetchClips();
  await store.initListeners();
});

onUnmounted(() => {
  cleanupWindowInteraction(false);
});

watch(
  () => store.searchQuery,
  () => {
    window.clearTimeout(searchTimer);
    searchTimer = window.setTimeout(() => store.fetchClips(), 150);
  },
);

useKeyboardNavigation({
  onArrowUp: () => store.selectPrev(),
  onArrowDown: () => store.selectNext(),
  onEnter: async () => {
    const clip = store.filteredClips[store.selectedIndex];
    if (clip) {
      await store.pasteClip(clip.id);
      await getCurrentWindow().hide();
    }
  },
  onEscape: () => getCurrentWindow().hide(),
});

async function onPaste(id: number) {
  await store.pasteClip(id);
  await getCurrentWindow().hide();
}

function openSettings() {
  router.push("/settings");
}

async function togglePinned() {
  await settingsStore.setPinned(!settingsStore.settings.is_pinned);
}

async function startWindowDrag(event: PointerEvent) {
  await startWindowInteraction("drag", event);
}

async function startWindowResize(event: PointerEvent) {
  await startWindowInteraction("resize", event);
}

async function startWindowInteraction(mode: "drag" | "resize", event: PointerEvent) {
  if (event.button !== 0) return;

  event.preventDefault();
  event.stopPropagation();
  cleanupWindowInteraction(false);

  const target = event.currentTarget as HTMLElement;
  const appWindow = getCurrentWindow();

  try {
    target.setPointerCapture(event.pointerId);
  } catch {
    // Pointer capture is best-effort in WebView2.
  }

  try {
    const [position, size, scaleFactor] = await Promise.all([
      appWindow.outerPosition(),
      appWindow.outerSize(),
      appWindow.scaleFactor(),
    ]);

    const startGeometry = {
      x: Math.round(position.x),
      y: Math.round(position.y),
      width: Math.round(size.width),
      height: Math.round(size.height),
    };

    windowInteraction = {
      mode,
      pointerId: event.pointerId,
      target,
      startMouseX: event.screenX,
      startMouseY: event.screenY,
      startGeometry,
      latestGeometry: startGeometry,
      scaleFactor,
      bounds: await getVisibleBounds(),
      rafId: null,
    };

    window.addEventListener("pointermove", onWindowPointerMove);
    window.addEventListener("pointerup", finishWindowInteraction);
    window.addEventListener("pointercancel", cancelWindowInteraction);
  } catch (error) {
    console.warn("window interaction start failed", error);
    cleanupWindowInteraction(false);
  }
}

function onWindowPointerMove(event: PointerEvent) {
  const current = windowInteraction;
  if (!current || event.pointerId !== current.pointerId) return;

  event.preventDefault();

  const dx = (event.screenX - current.startMouseX) * current.scaleFactor;
  const dy = (event.screenY - current.startMouseY) * current.scaleFactor;
  const start = current.startGeometry;

  if (current.mode === "drag") {
    const next = clampPosition(
      {
        ...start,
        x: Math.round(start.x + dx),
        y: Math.round(start.y + dy),
      },
      current.bounds,
    );
    scheduleWindowApply(next);
    return;
  }

  const widthDeltaFromY = dy / WINDOW_ASPECT;
  const dominantDelta = Math.abs(dx) >= Math.abs(widthDeltaFromY) ? dx : widthDeltaFromY;
  const minPhysicalWidth = Math.round(MIN_WINDOW_WIDTH * current.scaleFactor);
  const width = Math.max(minPhysicalWidth, Math.round(start.width + dominantDelta));
  const next = {
    ...start,
    width,
    height: Math.round(width * WINDOW_ASPECT),
  };

  scheduleWindowApply(next);
}

function scheduleWindowApply(geometry: WindowGeometry) {
  const current = windowInteraction;
  if (!current) return;

  current.latestGeometry = geometry;
  if (current.rafId !== null) return;

  current.rafId = window.requestAnimationFrame(() => {
    const latest = windowInteraction;
    if (!latest) return;

    latest.rafId = null;
    const appWindow = getCurrentWindow();
    const next = latest.latestGeometry;

    if (latest.mode === "resize") {
      void appWindow
        .setSize(new PhysicalSize(next.width, next.height))
        .catch((error) => console.warn("window resize failed", error));
    }

    void appWindow
      .setPosition(new PhysicalPosition(next.x, next.y))
      .catch((error) => console.warn("window move failed", error));
  });
}

async function finishWindowInteraction(event: PointerEvent) {
  const current = windowInteraction;
  if (!current || event.pointerId !== current.pointerId) return;

  event.preventDefault();
  const geometry = current.latestGeometry;
  const mode = current.mode;
  cleanupWindowInteraction(false);

  try {
    if (mode === "drag") {
      await settingsStore.saveWindowPosition(geometry.x, geometry.y);
    } else {
      await settingsStore.saveWindowGeometry(
        geometry.x,
        geometry.y,
        geometry.width,
        geometry.height,
      );
    }
  } catch (error) {
    console.warn("window state save failed", error);
  }
}

function cancelWindowInteraction(event?: PointerEvent) {
  if (event && windowInteraction && event.pointerId !== windowInteraction.pointerId) return;
  cleanupWindowInteraction(false);
}

function cleanupWindowInteraction(persistLatest: boolean) {
  const current = windowInteraction;
  if (!current) return;

  if (current.rafId !== null) {
    window.cancelAnimationFrame(current.rafId);
  }

  try {
    current.target.releasePointerCapture(current.pointerId);
  } catch {
    // The pointer may already be released by the platform.
  }

  window.removeEventListener("pointermove", onWindowPointerMove);
  window.removeEventListener("pointerup", finishWindowInteraction);
  window.removeEventListener("pointercancel", cancelWindowInteraction);

  if (persistLatest) {
    if (current.mode === "drag") {
      void settingsStore.saveWindowPosition(current.latestGeometry.x, current.latestGeometry.y);
    } else {
      void settingsStore.saveWindowGeometry(
        current.latestGeometry.x,
        current.latestGeometry.y,
        current.latestGeometry.width,
        current.latestGeometry.height,
      );
    }
  }

  windowInteraction = null;
}

async function getVisibleBounds(): Promise<ScreenBounds | null> {
  try {
    const monitors = await availableMonitors();
    if (monitors.length === 0) return null;

    return monitors.reduce<ScreenBounds>((bounds, monitor) => {
      const minX = monitor.workArea.position.x;
      const minY = monitor.workArea.position.y;
      const maxX = minX + monitor.workArea.size.width;
      const maxY = minY + monitor.workArea.size.height;

      return {
        minX: Math.min(bounds.minX, minX),
        minY: Math.min(bounds.minY, minY),
        maxX: Math.max(bounds.maxX, maxX),
        maxY: Math.max(bounds.maxY, maxY),
      };
    }, {
      minX: Number.POSITIVE_INFINITY,
      minY: Number.POSITIVE_INFINITY,
      maxX: Number.NEGATIVE_INFINITY,
      maxY: Number.NEGATIVE_INFINITY,
    });
  } catch (error) {
    console.warn("monitor bounds unavailable", error);
    return null;
  }
}

function clampPosition(geometry: WindowGeometry, bounds: ScreenBounds | null): WindowGeometry {
  if (!bounds) return geometry;

  return {
    ...geometry,
    x: clampAxis(geometry.x, bounds.minX, bounds.maxX - geometry.width),
    y: clampAxis(geometry.y, bounds.minY, bounds.maxY - geometry.height),
  };
}

function clampAxis(value: number, min: number, max: number) {
  if (max < min) return min;
  return Math.min(Math.max(value, min), max);
}

function openContextMenu(event: MouseEvent, clipId: number, index: number) {
  store.selectedIndex = index;
  contextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    clipId,
  };
}

async function favoriteFromContext() {
  if (!contextMenu.value.clipId) return;
  await store.toggleFavorite(contextMenu.value.clipId);
  contextMenu.value.visible = false;
}

async function deleteFromContext() {
  if (!contextMenu.value.clipId) return;
  await store.deleteClip(contextMenu.value.clipId);
  contextMenu.value.visible = false;
}

async function clearAll() {
  if (!window.confirm("清空所有剪贴板历史记录？")) return;
  await store.clearAll();
}
</script>

<template>
  <div class="main-panel" @click="contextMenu.visible = false">
    <header class="panel-header">
      <div class="drag-titlebar" @pointerdown="startWindowDrag"></div>
      <div class="header-top">
        <SearchBar v-model="store.searchQuery" />
        <n-tooltip trigger="hover">
          <template #trigger>
            <button
              class="pin-button"
              :class="{ pinned: settingsStore.settings.is_pinned }"
              :aria-label="settingsStore.settings.is_pinned ? '取消固定窗口' : '固定窗口'"
              :title="settingsStore.settings.is_pinned ? '取消固定窗口' : '固定窗口'"
              @click.stop="togglePinned"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M14 3l7 7-3.2 1.1-3.7 3.7v3.6l-2.1 2.1-2.7-5.3L4 12.5l2.1-2.1h3.6l3.7-3.7L14 3z" />
              </svg>
            </button>
          </template>
          {{ settingsStore.settings.is_pinned ? "取消固定窗口" : "固定窗口" }}
        </n-tooltip>
      </div>
      <div class="header-actions">
        <n-button text size="small" @click="router.push('/favorites')">收藏</n-button>
        <n-button text size="small" @click="clearAll">清空</n-button>
        <n-button text size="small" @click="openSettings">设置</n-button>
      </div>
    </header>

    <n-spin :show="store.loading">
      <div class="clip-list">
        <ClipItem
          v-for="(clip, index) in store.filteredClips"
          :key="clip.id"
          :clip="clip"
          :selected="index === store.selectedIndex"
          @select="store.selectedIndex = index"
          @paste="onPaste(clip.id)"
          @context="openContextMenu($event, clip.id, index)"
        />
        <div v-if="store.filteredClips.length === 0" class="empty">
          暂无记录
        </div>
      </div>
    </n-spin>

    <footer class="panel-footer">
      <span>{{ store.filteredClips.length }} 条记录</span>
    </footer>

    <ContextMenu
      :visible="contextMenu.visible"
      :x="contextMenu.x"
      :y="contextMenu.y"
      :favorite-label="contextClip?.isFavorite ? '取消收藏' : '收藏'"
      @favorite="favoriteFromContext"
      @delete="deleteFromContext"
      @close="contextMenu.visible = false"
    />
    <button class="resize-handle" aria-label="Resize window" @pointerdown="startWindowResize">
      <span></span>
      <span></span>
    </button>
  </div>
</template>

<style scoped>
.main-panel {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 10px 12px 14px;
  box-sizing: border-box;
}

.panel-header {
  margin-bottom: 8px;
}

.drag-titlebar {
  height: 18px;
  margin: -2px 36px 6px 2px;
  cursor: move;
  user-select: none;
  touch-action: none;
}

.drag-titlebar:active {
  cursor: grabbing;
}

.header-top {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 32px;
  align-items: center;
  gap: 8px;
}

.pin-button {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
}

.pin-button:hover {
  background: var(--clip-hover-bg);
  color: var(--text-primary);
}

.pin-button svg {
  width: 17px;
  height: 17px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linejoin: round;
}

.pin-button.pinned {
  color: #2563eb;
  background: rgba(37, 99, 235, 0.12);
  border-color: rgba(37, 99, 235, 0.2);
}

.pin-button.pinned svg {
  fill: currentColor;
}

.header-actions {
  display: flex;
  justify-content: flex-end;
  gap: 4px;
  margin-top: 6px;
}

.clip-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.empty {
  text-align: center;
  padding: 32px;
  color: var(--text-secondary);
  font-size: 13px;
}

.panel-footer {
  display: flex;
  justify-content: center;
  gap: 16px;
  padding-top: 8px;
  padding-right: 18px;
  font-size: 11px;
  color: var(--text-secondary);
}

.resize-handle {
  position: absolute;
  right: 4px;
  bottom: 4px;
  width: 18px;
  height: 18px;
  display: grid;
  place-items: end;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  cursor: nwse-resize;
  touch-action: none;
}

.resize-handle span {
  position: absolute;
  right: 3px;
  bottom: 3px;
  display: block;
  height: 1px;
  background: currentColor;
  opacity: 0.75;
  transform: rotate(135deg);
  transform-origin: right center;
}

.resize-handle span:first-child {
  width: 12px;
  bottom: 6px;
}

.resize-handle span:last-child {
  width: 7px;
}

.resize-handle:hover {
  color: var(--text-primary);
}
</style>
