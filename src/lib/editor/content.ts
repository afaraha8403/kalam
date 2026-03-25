import { marked } from 'marked'

/**
 * Convert stored entry `content` to HTML for Tiptap.
 * Legacy plain text / markdown is parsed with `marked`; strings that already look like HTML pass through.
 */
export function storedContentToEditorHtml(content: string): string {
  const raw = (content ?? '').trim()
  if (!raw) return '<p></p>'

  const looksLikeHtml =
    /^<\s*[a-z][\s\S]*>/i.test(raw) &&
    (raw.includes('</p>') ||
      raw.includes('</h') ||
      raw.includes('</li>') ||
      raw.includes('</ul>') ||
      raw.includes('</ol>') ||
      raw.includes('<br') ||
      raw.includes('</div>'))

  if (looksLikeHtml) {
    return raw
  }

  try {
    const html = marked(raw, { async: false }) as string
    return html?.trim() ? html : '<p></p>'
  } catch {
    return `<p>${escapeHtml(raw)}</p>`
  }
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}
