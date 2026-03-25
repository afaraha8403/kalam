import { invoke } from '../backend'

/** Returns a data URL for the cached PNG icon, or null. */
export async function getAppIcon(processName: string): Promise<string | null> {
  const base64 = await invoke<string | null>('get_app_icon', { processName })
  return base64 ? `data:image/png;base64,${base64}` : null
}

export async function resolveTargetApp(
  processName: string
): Promise<{ displayName: string; iconDataUrl: string | null }> {
  const [displayName, iconBase64] = await invoke<[string, string | null]>('resolve_target_app', {
    processName
  })
  return {
    displayName,
    iconDataUrl: iconBase64 ? `data:image/png;base64,${iconBase64}` : null
  }
}
