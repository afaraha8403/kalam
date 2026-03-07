/** Recognition language options (no auto-detect). Used for multi-select; first in list = default. */
export const LANGUAGE_OPTIONS: { code: string; label: string }[] = [
  { code: 'en', label: 'English' },
  { code: 'es', label: 'Spanish' },
  { code: 'fr', label: 'French' },
  { code: 'de', label: 'German' },
  { code: 'zh', label: 'Chinese' },
  { code: 'yue', label: 'Cantonese' },
  { code: 'ja', label: 'Japanese' },
  { code: 'ko', label: 'Korean' },
  { code: 'pt', label: 'Portuguese' },
  { code: 'it', label: 'Italian' },
  { code: 'ru', label: 'Russian' },
  { code: 'ar', label: 'Arabic' },
  { code: 'hi', label: 'Hindi' },
  { code: 'nl', label: 'Dutch' },
  { code: 'pl', label: 'Polish' },
  { code: 'tr', label: 'Turkish' },
  { code: 'sv', label: 'Swedish' },
  { code: 'id', label: 'Indonesian' },
  { code: 'th', label: 'Thai' },
]

/** Language codes supported by each STT provider. Groq (Whisper) supports 99+; SenseVoice supports 5. */
export const SUPPORTED_LANGUAGES_BY_PROVIDER: Record<string, string[]> = {
  groq: LANGUAGE_OPTIONS.map((o) => o.code),
  openai: LANGUAGE_OPTIONS.map((o) => o.code),
  sensevoice: ['en', 'zh', 'ja', 'ko', 'yue'],
  whisper_base: LANGUAGE_OPTIONS.map((o) => o.code),
}

export function languageLabel(code: string): string {
  return LANGUAGE_OPTIONS.find((o) => o.code === code)?.label ?? code
}

export function getSupportedLanguagesForProvider(providerKey: string): string[] {
  return SUPPORTED_LANGUAGES_BY_PROVIDER[providerKey] ?? []
}

export function isLanguageSupportedByProvider(code: string, providerKey: string): boolean {
  return getSupportedLanguagesForProvider(providerKey).includes(code)
}
