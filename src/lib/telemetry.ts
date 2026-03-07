/**
 * Opt-in PostHog telemetry. Only initializes and sends events when telemetry_enabled is true.
 * Respects privacy.telemetry_enabled from settings.
 */

const POSTHOG_KEY = typeof import.meta !== 'undefined' && (import.meta as unknown as { env?: { VITE_POSTHOG_KEY?: string } }).env?.VITE_POSTHOG_KEY

let initialized = false

export function initTelemetry(enabled: boolean): void {
  if (!enabled || !POSTHOG_KEY) {
    return
  }
  if (initialized) {
    return
  }
  try {
    // Dynamic import so we don't load PostHog when disabled
    import('posthog-js').then(({ default: posthog }) => {
      posthog.init(POSTHOG_KEY, {
        api_host: 'https://us.i.posthog.com',
        person_profiles: 'identified_only',
        capture_pageview: false,
      })
      posthog.opt_in_capturing()
      initialized = true
      posthog.capture('app_loaded')
    }).catch(() => {})
  } catch {
    // no-op
  }
}

export function captureEvent(event: string, properties?: Record<string, unknown>): void {
  if (!initialized) return
  try {
    import('posthog-js').then(({ default: posthog }) => {
      posthog.capture(event, properties)
    }).catch(() => {})
  } catch {
    // no-op
  }
}

export function optOut(): void {
  if (!initialized) return
  try {
    import('posthog-js').then(({ default: posthog }) => {
      posthog.opt_out_capturing()
    }).catch(() => {})
  } catch {
    // no-op
  }
  initialized = false
}
