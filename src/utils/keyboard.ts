import { onMounted, onUnmounted } from "vue";

export interface KeyboardHandlers {
  onArrowUp?: () => void;
  onArrowDown?: () => void;
  onEnter?: () => void;
  onEscape?: () => void;
}

/** Register global keyboard handlers for panel navigation. */
export function useKeyboardNavigation(handlers: KeyboardHandlers) {
  function onKeyDown(e: KeyboardEvent) {
    switch (e.key) {
      case "ArrowUp":
        e.preventDefault();
        handlers.onArrowUp?.();
        break;
      case "ArrowDown":
        e.preventDefault();
        handlers.onArrowDown?.();
        break;
      case "Enter":
        e.preventDefault();
        handlers.onEnter?.();
        break;
      case "Escape":
        e.preventDefault();
        handlers.onEscape?.();
        break;
    }
  }

  onMounted(() => window.addEventListener("keydown", onKeyDown));
  onUnmounted(() => window.removeEventListener("keydown", onKeyDown));
}
