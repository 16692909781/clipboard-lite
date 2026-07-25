export interface ClipRecord {
  id: number;
  content: string;
  contentHash: string;
  clipType: "text" | "image" | "files";
  preview: string;
  isFavorite: boolean;
  sourceApp?: string | null;
  createdAt: number;
}

export interface AppSettings {
  maxCount: number;
  hotkey: string;
  theme: "light" | "dark" | "system";
  retentionDays: number;
  ignoredApps: string[];
  autostart: boolean;
  is_first_launch: boolean;
  is_pinned: boolean;
  window_x: number | null;
  window_y: number | null;
  window_width: number;
  window_height: number;
}
