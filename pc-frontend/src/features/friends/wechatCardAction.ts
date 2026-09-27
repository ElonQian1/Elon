import { getDesktopInvoke } from '../shell/desktopShell'
import { previewApi } from './socialReadBack'

/** No reader tab is created; temporary export IDs are not cached or logged. */
export async function openWechatCard(url: string, signal: AbortSignal) {
  const id = ElonSocialLinks.channelsId(url)
  const invoke = getDesktopInvoke()
  if (!id || !invoke) throw new Error('请在一龙客户端中打开，或复制链接到微信。')
  const value = await previewApi('/api/me/link-preview/wechat-open', { method: 'POST', body: JSON.stringify({ url }), signal })
  if (signal.aborted) return
  if (value?.schema !== 1 || typeof value.source_url !== 'string' || ElonSocialLinks.channelsId(value.source_url) !== id ||
      typeof value.launch_url !== 'string' || !value.launch_url.startsWith('weixin://biz/finder/openFinderFeed/') ||
      !Number.isSafeInteger(value.expires_at_ms) || value.expires_at_ms <= Date.now() + 1000) throw new Error('微信跳转链接无效或已过期，请重试。')
  // Native code independently validates the exact source, scheme and allowed fields.
  await invoke('open_wechat_feed_url', { sourceUrl: value.source_url, launchUrl: value.launch_url, expiresAtMs: value.expires_at_ms })
}
