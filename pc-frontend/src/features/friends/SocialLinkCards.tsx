import { useEffect, useRef, useState } from 'react'
import { resolveApiUrl } from '../../api/runtime'
import { getDesktopInvoke } from '../shell/desktopShell'
import { useReaderTabs } from '../reader/readerTabsStore'
import { previewApi } from './socialReadBack'
import { cachedRead } from './socialReadPreview'
import { openWechatCard } from './wechatCardAction'
import { copyTextToClipboard } from '../../lib/clipboard'
import type { LinkPreview } from './socialLinks'
import '../../../../server/src/assets/social_links.js'
import '../../../../server/src/assets/social_link_viewer.js'
import '../../../../server/src/assets/social_links.css'

export default function SocialLinkCards({ text, owner, compact = false, onDesktopOpen }: { text: string; owner: string; compact?: boolean; onDesktopOpen?: () => void }) {
  const host = useRef<HTMLDivElement>(null)
  const scope = resolveApiUrl('/') + '\n' + owner
  const [status, setStatus] = useState('')
  const [recovery, setRecovery] = useState<LinkPreview | null>(null)
  useEffect(() => {
    if (!host.current) return
    for (const item of ElonSocialLinks.links(text)) {
      const cached = cachedRead(scope, item); if (cached) ElonSocialLinks.remember(scope, cached.preview, cached.expires)
    }
    let active = true
    let pending: AbortController | null = null
    setStatus(''); setRecovery(null)
    const original = (p: LinkPreview) => {
      pending?.abort(); pending = null; setStatus(''); setRecovery(null)
      if (getDesktopInvoke()) { useReaderTabs.getState().open(p, scope); onDesktopOpen?.() }
      else ElonSocialLinkViewer.open(p)
    }
    const cancel = () => { if (document.hidden) { pending?.abort(); pending = null; setStatus(''); } }
    document.addEventListener('visibilitychange', cancel)
    const dispose = ElonSocialLinks.mount(host.current, text, { owner: scope, compact, desktop: true, channelsHandoff: !!getDesktopInvoke(), api: previewApi, openOriginal: original, open: p => {
      if (!ElonSocialLinks.channelsId(p.url) || !getDesktopInvoke()) { original(p); return }
      if (pending) return
      const controller = new AbortController(); pending = controller
      const timer = setTimeout(() => controller.abort(), 12000)
      setStatus('正在打开微信…'); setRecovery(null)
      void openWechatCard(p.url, controller.signal).then(() => {
        if (active && !controller.signal.aborted) { setStatus('已请求微信打开。若未进入视频，可复制链接或查看原网页扫码。'); setRecovery(p) }
      }).catch(() => {
        if (active && pending === controller && !document.hidden) { setStatus('未能确认微信跳转，请重试、复制链接或查看原网页。'); setRecovery(p) }
      }).finally(() => { clearTimeout(timer); if (pending === controller) pending = null })
    } })
    return () => { active = false; pending?.abort(); document.removeEventListener('visibilitychange', cancel); dispose() }
  }, [text, scope, compact, onDesktopOpen])
  useEffect(() => () => { ElonSocialLinkViewer.close() }, [scope, text])
  return <div><div ref={host} /><div role="status" style={{ maxWidth: 280, fontSize: 12, overflowWrap: 'anywhere' }}>{status}</div>
    {recovery && <button type="button" className="social-link-original-action" onClick={() => { void copyTextToClipboard(recovery.url).then(ok => setStatus(ok ? '链接已复制，可粘贴到微信。' : '复制失败，请查看原网页。')) }}>复制链接</button>}
  </div>
}
