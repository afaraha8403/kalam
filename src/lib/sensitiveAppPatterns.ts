import type { SensitiveAppPattern } from '../types'

/**
 * Whether `processName` matches a saved sensitive-app regex. Mirrors Rust `Regex::new` + `is_match`
 * on the process name (`foreground_matches_sensitive_patterns` in `privacy.rs`).
 * Patterns from Settings use a `(?i)` prefix; JavaScript needs the `i` flag instead of inline `(?i)`.
 */
export function processNameMatchesSensitivePattern(processName: string, p: SensitiveAppPattern): boolean {
  if (p.pattern_type !== 'ProcessName') return false
  const raw = p.pattern.trim()
  try {
    const re = raw.startsWith('(?i)') ? new RegExp(raw.slice(4), 'i') : new RegExp(raw)
    return re.test(processName)
  } catch {
    const normalized = processName.toLowerCase()
    const pl = raw.toLowerCase()
    return pl.includes(normalized) || normalized.includes(pl.replace(/[^a-z0-9]/g, ''))
  }
}

export function isProcessInSensitiveList(
  patterns: SensitiveAppPattern[] | undefined,
  processName: string
): boolean {
  if (!patterns?.length) return false
  return patterns.some((p) => processNameMatchesSensitivePattern(processName, p))
}
