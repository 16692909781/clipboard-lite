/** Format Unix ms timestamp to relative time string. */
export function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  return `${days} 天前`;
}

/** Map clip type to display label. */
export function clipTypeLabel(type: string): string {
  switch (type) {
    case "image":
      return "图片";
    case "files":
      return "文件";
    default:
      return "文本";
  }
}

/** Truncate preview text for list display. */
export function truncate(text: string, max = 120): string {
  const singleLine = text.replace(/\s+/g, " ").trim();
  return singleLine.length > max ? `${singleLine.slice(0, max)}…` : singleLine;
}
