export async function copyTextToClipboard(text: string): Promise<boolean> {
  if (!text) return false

  if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch {
      // Fall through to the textarea fallback when clipboard permissions are blocked.
    }
  }

  return fallbackCopyText(text)
}

function fallbackCopyText(text: string): boolean {
  if (typeof document === 'undefined') return false

  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.setAttribute('readonly', '')
  textarea.style.position = 'fixed'
  textarea.style.top = '0'
  textarea.style.left = '-9999px'
  textarea.style.opacity = '0'
  document.body.appendChild(textarea)
  textarea.focus()
  textarea.select()
  textarea.setSelectionRange(0, text.length)

  try {
    return document.execCommand('copy')
  } catch {
    return false
  } finally {
    document.body.removeChild(textarea)
  }
}

export type RichClipboardResult = 'rich' | 'text' | 'failed'

export async function copyRichTextToClipboard(html: string, fallbackText: string): Promise<RichClipboardResult> {
  if (html && typeof navigator !== 'undefined' && navigator.clipboard?.write && typeof ClipboardItem !== 'undefined') {
    try {
      await navigator.clipboard.write([
        new ClipboardItem({
          'text/html': new Blob([html], { type: 'text/html' }),
          'text/plain': new Blob([fallbackText], { type: 'text/plain' }),
        }),
      ])
      return 'rich'
    } catch {
      // Fall back to Markdown/plain text when rich clipboard writes are unavailable.
    }
  }

  return await copyTextToClipboard(fallbackText) ? 'text' : 'failed'
}

export function sanitizedRichHtmlFromElement(element: HTMLElement | null): string {
  if (!element) return ''

  const clone = element.cloneNode(true) as HTMLElement
  clone
    .querySelectorAll('script, style, iframe, object, embed, form, input, textarea, select, button, [data-copy-exclude="true"]')
    .forEach((node) => node.remove())

  clone.querySelectorAll('*').forEach((node) => {
    if (!(node instanceof HTMLElement)) return
    for (const attr of Array.from(node.attributes)) {
      const name = attr.name.toLowerCase()
      if (name.startsWith('on') || name === 'style' || name === 'class' || name.startsWith('data-') || name.startsWith('aria-')) {
        node.removeAttribute(attr.name)
      }
    }

    if (node instanceof HTMLAnchorElement) {
      if (!isSafeCopyUrl(node.getAttribute('href'))) node.removeAttribute('href')
      node.removeAttribute('target')
      node.removeAttribute('rel')
    }

    if (node instanceof HTMLImageElement && !isSafeCopyImageUrl(node.getAttribute('src'))) {
      node.removeAttribute('src')
    }
  })

  return `<div>${clone.innerHTML.trim()}</div>`
}

function isSafeCopyUrl(value: string | null): boolean {
  if (!value) return false
  return /^(https?:|mailto:|tel:|\/|#)/i.test(value)
}

function isSafeCopyImageUrl(value: string | null): boolean {
  if (!value) return false
  return /^(https?:|data:image\/(?:png|jpeg|jpg|gif|webp);base64,|\/)/i.test(value)
}
