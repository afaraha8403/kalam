/**
 * OS-locale-aware options for svelty-picker: calendar labels from Intl, week start from
 * Intl.Locale weekInfo when available, and a displayFormat derived from the user's date/time
 * patterns. Storage format stays `yyyy-mm-ddThh:ii` (24h) for DB/API compatibility.
 */
import bundled from 'svelty-picker/i18n'
import type { i18nType } from 'svelty-picker/i18n'

const bundledLocales = bundled as Record<string, i18nType>

function resolveLocaleTag(): string {
  if (typeof navigator !== 'undefined' && navigator.language) return navigator.language
  return 'en-US'
}

function pickBundled(locale: string): i18nType {
  const short = locale.split('-')[0]?.toLowerCase() ?? 'en'
  const underscored = locale.replace(/-/g, '_').toLowerCase()
  if (bundledLocales[underscored]) return bundledLocales[underscored]
  if (short === 'zh') return bundledLocales.zh_CN
  if (short === 'pt') return bundledLocales.pt_BR
  if (short === 'ar') return bundledLocales.ar_DZ
  if (bundledLocales[short]) return bundledLocales[short]
  return bundledLocales.en
}

/** Intl.Locale weekInfo.firstDay: 1 = Monday … 7 = Sunday (ECMA-402). Svelty: 0 = Sunday … 6 = Saturday. */
function weekStartFromLocale(locale: string): number {
  try {
    const loc = new Intl.Locale(locale)
    const wi = (loc as Intl.Locale & { weekInfo?: { firstDay?: number } }).weekInfo
    const fd = wi?.firstDay
    if (fd === 7) return 0
    if (fd != null && fd >= 1 && fd <= 6) return fd
  } catch {
    /* ignore */
  }
  return 1
}

function localePrefersHour12(locale: string): boolean {
  const h12 = new Intl.DateTimeFormat(locale, { hour: 'numeric', minute: '2-digit' }).resolvedOptions().hour12
  return h12 === true
}

/**
 * Build svelty `displayFormat` from the user's locale ordering and separators (numeric date parts).
 *
 * svelty-picker "standard" tokens are not the same as typical strftime: `hh`/`h` are 24-hour from
 * `getHours()`; `HH`/`H` are 12-hour when `i18n.meridiem` has two entries. Map locale 12h → `HH`,
 * 24h → `hh`. Use `P` so meridiem displays uppercase (e.g. "PM"); parsing still accepts `p`/`P`.
 */
function buildDisplayFormat(locale: string): string {
  const hour12 = localePrefersHour12(locale)
  const sample = new Date(2025, 2, 4, 15, 7)
  const dtf = new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12
  })
  let out = ''
  for (const p of dtf.formatToParts(sample)) {
    if (p.type === 'month') out += 'mm'
    else if (p.type === 'day') out += 'dd'
    else if (p.type === 'year') out += 'yyyy'
    else if (p.type === 'hour') out += hour12 ? 'HH' : 'hh'
    else if (p.type === 'minute') out += 'ii'
    else if (p.type === 'dayPeriod') out += 'P'
    else out += p.value
  }
  return out
}

function intlMeridiem(locale: string): [string, string] {
  const fmt = new Intl.DateTimeFormat(locale, { hour: 'numeric', hour12: true })
  const am =
    fmt.formatToParts(new Date(2025, 0, 1, 0, 30)).find((x) => x.type === 'dayPeriod')?.value?.toLowerCase() ?? 'am'
  const pm =
    fmt.formatToParts(new Date(2025, 0, 1, 12, 30)).find((x) => x.type === 'dayPeriod')?.value?.toLowerCase() ?? 'pm'
  return [am, pm]
}

/** Sunday-first week: names aligned with svelty-picker `en` (index 0 = Sunday, last = duplicate Sunday). */
function intlDayMonthNames(locale: string, base: i18nType): Pick<i18nType, 'days' | 'daysShort' | 'daysMin' | 'months' | 'monthsShort' | 'meridiem'> {
  const months: string[] = []
  const monthsShort: string[] = []
  for (let m = 0; m < 12; m++) {
    const d = new Date(2025, m, 1)
    months.push(new Intl.DateTimeFormat(locale, { month: 'long' }).format(d))
    monthsShort.push(new Intl.DateTimeFormat(locale, { month: 'short' }).format(d))
  }
  const sunday = new Date(2025, 0, 5)
  const days: string[] = []
  const daysShort: string[] = []
  const daysMin: string[] = []
  for (let i = 0; i < 7; i++) {
    const d = new Date(sunday.getTime() + i * 86400000)
    days.push(new Intl.DateTimeFormat(locale, { weekday: 'long' }).format(d))
    daysShort.push(new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(d))
    daysMin.push(new Intl.DateTimeFormat(locale, { weekday: 'narrow' }).format(d))
  }
  days.push(days[0])
  daysShort.push(daysShort[0])
  daysMin.push(daysMin[0])
  let meridiem: [string, string]
  try {
    meridiem = intlMeridiem(locale)
  } catch {
    meridiem = [base.meridiem[0] ?? 'am', base.meridiem[1] ?? 'pm']
  }
  return {
    months,
    monthsShort,
    days,
    daysShort,
    daysMin,
    meridiem
  }
}

export type KalamSveltyPickerLocaleOptions = {
  format: string
  displayFormat: string
  displayFormatType: 'standard'
  i18n: i18nType
  weekStart: number
}

/** Props to spread onto `<SveltyPicker>` for OS locale + Kalam storage format. */
export function getKalamSveltyPickerLocaleOptions(): KalamSveltyPickerLocaleOptions {
  const locale = resolveLocaleTag()
  const base = pickBundled(locale)
  const names = intlDayMonthNames(locale, base)
  const i18n: i18nType = {
    ...base,
    ...names,
    suffix: base.suffix
  }
  return {
    format: 'yyyy-mm-ddThh:ii',
    displayFormat: buildDisplayFormat(locale),
    displayFormatType: 'standard',
    i18n,
    weekStart: weekStartFromLocale(locale)
  }
}
