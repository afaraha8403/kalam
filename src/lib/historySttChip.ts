/** Cloud / Local / Hybrid / Auto chips; `unknown` when path cannot be inferred. */
export type SttChipKind = 'cloud' | 'local' | 'hybrid' | 'auto' | 'unknown'

/** @deprecated Prefer `recognitionDisplay` for list/detail copy; kept for any code that only has mode. */
export function sttChipLabel(stt: string | null | undefined): string {
  const t = stt?.trim()
  return t && t.length > 0 ? t : 'Unknown'
}

/** Human-readable provider id (settings `stt_config.provider`, e.g. groq → Groq). */
export function prettySttProviderId(id: string): string {
  const k = id.trim().toLowerCase()
  const map: Record<string, string> = {
    groq: 'Groq',
    openai: 'OpenAI',
    openrouter: 'OpenRouter',
    gemini: 'Gemini',
    anthropic: 'Anthropic',
    sensevoice: 'SenseVoice',
  }
  if (map[k]) return map[k]
  return k
    .split(/[_\s]+/)
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
    .join(' ')
}

/**
 * List/detail label: **Groq (Cloud)** when both provider and mode exist; falls back to whichever is present.
 * Legacy rows may have mode without provider or neither → **Unknown** only then.
 */
export function recognitionDisplay(
  provider: string | null | undefined,
  mode: string | null | undefined
): string {
  const modePart = mode?.trim()
  const providerRaw = provider?.trim()
  const pretty = providerRaw && providerRaw.length > 0 ? prettySttProviderId(providerRaw) : ''
  if (pretty && modePart) return `${pretty} (${modePart})`
  if (pretty) return pretty
  if (modePart) return modePart
  return 'Unknown'
}

export function sttChipKind(
  stt: string | null | undefined,
  provider?: string | null | undefined
): SttChipKind {
  const s = (stt ?? '').trim().toLowerCase()
  if (s === 'local') return 'local'
  if (s === 'hybrid') return 'hybrid'
  if (s === 'auto') return 'auto'
  if (s === 'cloud') return 'cloud'
  if (!s) {
    const p = (provider ?? '').trim().toLowerCase()
    if (p.includes('sense') || p.includes('whisper') || p === 'local') return 'local'
    if (p.length > 0) return 'cloud'
  }
  return 'unknown'
}
