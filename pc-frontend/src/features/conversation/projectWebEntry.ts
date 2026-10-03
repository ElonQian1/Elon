import { getDesktopInvoke } from '../shell/desktopShell'
import { useReaderTabs } from '../reader/readerTabsStore'

/** A project web app is unprivileged external content, never an AI provider session. */
export function projectWebUrl(value?: string | null): string | null {
  if (!value || value.length > 2048 || /[\s\\\u0000-\u001f]/.test(value)) return null
  try {
    const url = new URL(value)
    return url.protocol === 'https:' && !url.username && !url.password ? url.href : null
  } catch { return null }
}

export function openProjectWeb(value: string, title: string, projectId: string): void {
  const url = projectWebUrl(value)
  if (!url) throw new Error('网页端地址无效，需要 HTTPS 地址。')
  if (!getDesktopInvoke()) {
    window.open(url, '_blank', 'noopener,noreferrer')
    return
  }
  const tab = useReaderTabs.getState().open({
    schema: 1, url, title, site: new URL(url).hostname,
    author: '', description: '', image: null, embed: null,
    status: 'unavailable', source: 'server',
  }, `project:${projectId}`)
  if (!tab) throw new Error('网页标签已满，请先关闭一个标签后重试。')
}
