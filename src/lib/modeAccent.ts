import type { DictationMode } from '../types'

/** Preset accents for the mode editor (OKLCH — matches overlay tokens). */
export const MODE_ACCENT_PRESETS: { label: string; value: string }[] = [
  { label: 'Blue', value: 'oklch(68% 0.1 240)' },
  { label: 'Teal', value: 'oklch(70% 0.08 180)' },
  { label: 'Green', value: 'oklch(65% 0.09 140)' },
  { label: 'Gold', value: 'oklch(72% 0.06 85)' },
  { label: 'Coral', value: 'oklch(68% 0.13 15)' },
  { label: 'Purple', value: 'oklch(65% 0.1 290)' },
  { label: 'Slate', value: 'oklch(62% 0.04 250)' },
  { label: 'Rose', value: 'oklch(62% 0.12 350)' },
]

/** Default accent by built-in mode id (custom modes use deterministic hash). */
export function defaultAccentForModeId(id: string): string {
  const map: Record<string, string> = {
    voice: 'oklch(68% 0.1 240)',
    email: 'oklch(68% 0.1 220)',
    message: 'oklch(70% 0.1 160)',
    notes: 'oklch(72% 0.06 85)',
  }
  if (map[id]) return map[id]
  let hash = 0
  for (let i = 0; i < id.length; i++) {
    hash = id.charCodeAt(i) + (hash * 31)
  }
  const hue = ((hash % 360) + 360) % 360
  return `oklch(68% 0.1 ${hue})`
}

/** Stored color or id-based default (mirrors Rust `effective_accent_color`). */
export function effectiveModeAccent(m: Pick<DictationMode, 'id' | 'accent_color'>): string {
  const t = m.accent_color?.trim()
  return t || defaultAccentForModeId(m.id)
}
