/**
 * Single backend entry point for Tauri invoke.
 * - In Tauri: uses @tauri-apps/api/core invoke (real backend).
 * - In browser: uses dev bridge HTTP API (http://localhost:1430) when the Tauri app
 *   is running with dev-bridge feature, so the browser tab can show real DB data.
 */

/** True when running inside the Tauri webview (not the Vite dev browser tab). */
export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && !!(window as { __TAURI__?: unknown }).__TAURI__
}

// Use 127.0.0.1 so we match the bridge (avoids IPv6 localhost ::1 when only IPv4 is bound)
const DEV_BRIDGE_URL =
  typeof import.meta !== 'undefined' && import.meta.env?.VITE_DEV_BRIDGE_URL
    ? String(import.meta.env.VITE_DEV_BRIDGE_URL)
    : 'http://127.0.0.1:1430'

/**
 * Invoke a Tauri command. In browser, forwards to the dev bridge (if Tauri is running with dev-bridge).
 */
/**
 * Subscribe to Tauri events. In a plain browser (Cypress / Vite without Tauri), returns a no-op unlisten
 * so pages that call `listen` during onMount still load.
 */
export async function listenSafe<T = unknown>(
  event: string,
  handler: (event: { payload: T }) => void
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {}
  }
  const { listen } = await import('@tauri-apps/api/event')
  return listen<T>(event, handler)
}

export async function invoke<T = unknown>(cmd: string, payload?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core')
    return tauriInvoke<T>(cmd, payload ?? {})
  }
  const args = payload ?? {}
  let res: Response
  try {
    res = await fetch(`${DEV_BRIDGE_URL}/api/invoke`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ cmd, args }),
      cache: 'no-store'
    })
  } catch (e) {
    const msg =
      e instanceof TypeError && String(e.message).toLowerCase().includes('fetch')
        ? 'Dev bridge not reachable. Run npm run dev and wait until you see "Dev bridge listening" in the terminal, then refresh this page. If it still fails, open http://127.0.0.1:1430/health in a new tab to check.'
        : e instanceof Error
          ? e.message
          : String(e)
    throw new Error(msg)
  }
  const data = await res.json().catch(() => ({}))
  if (!res.ok) {
    const msg = data?.error ?? res.statusText ?? 'Dev bridge request failed'
    throw new Error(msg)
  }
  return data as T
}
