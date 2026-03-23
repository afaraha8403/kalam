/**
 * OS / branding label for the Meta (Windows/Command/Super) modifier in hotkey UI.
 * Backend parse_rdev_hotkey accepts win|super|command|cmd|meta (case-insensitive).
 */
export function superKeyLabel(platform: string): string {
  const p = platform.toLowerCase()
  if (p === 'windows') return 'Win'
  if (p === 'macos') return 'Cmd'
  return 'Super'
}

/** True if this hotkey segment is the OS/meta modifier (any stored spelling). */
export function isMetaKeyToken(part: string): boolean {
  return /^(win|super|command|cmd|meta)$/i.test(part.trim())
}

/** Sort order: Ctrl, Alt, Shift, meta, then non-modifiers (stable tie-break by name). */
export function modifierSortIndex(key: string): number {
  if (key === 'Ctrl') return 0
  if (key === 'Alt') return 1
  if (key === 'Shift') return 2
  if (isMetaKeyToken(key)) return 3
  return 4
}

/** Show each segment with the platform-appropriate meta label. */
export function formatHotkeyForDisplay(hotkey: string, platform: string): string {
  if (!hotkey?.trim()) return hotkey
  return hotkey
    .split('+')
    .map((s) => s.trim())
    .map((part) => (isMetaKeyToken(part) ? superKeyLabel(platform) : part))
    .join('+')
}
