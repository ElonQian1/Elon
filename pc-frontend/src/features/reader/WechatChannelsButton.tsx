import { useEffect, useRef, useState } from 'react'
import { Copy, MessageCircle } from 'lucide-react'
import { controlInternalBrowserTab } from '../user-browser/internalBrowserApi'
import { copyTextToClipboard } from '../../lib/clipboard'

function isWechatChannelsUrl(value: string): boolean {
  try {
    const u = new URL(value)
    return u.protocol === 'https:' && !u.username && !u.password && !u.port && (
      (u.hostname === 'weixin.qq.com' && /^\/sph\/[A-Za-z0-9_-]{1,128}$/.test(u.pathname)) ||
      (u.hostname === 'channels.weixin.qq.com' && u.pathname === '/finder-preview/pages/sph')
    )
  } catch { return false }
}

export function WechatChannelsButton({ tabId, url, onStatus }: { tabId: string; url: string; onStatus: (message: string) => void }) {
  const [busy, setBusy] = useState(false)
  const pending = useRef(false)
  const generation = useRef(0)
  useEffect(() => { generation.current++; pending.current = false; setBusy(false); return () => { generation.current++ } }, [tabId, url])
  if (!isWechatChannelsUrl(url)) return null
  const open = async () => {
    if (pending.current) return
    const current = generation.current
    pending.current = true; setBusy(true); onStatus('正在获取微信跳转链接…')
    try {
      await controlInternalBrowserTab('wechat', tabId)
      if (generation.current === current) onStatus('已请求微信打开；若未显示此视频，可使用页面二维码。')
    } catch (error) {
      if (generation.current === current) onStatus(error instanceof Error ? error.message : String(error))
    } finally {
      if (generation.current === current) { pending.current = false; setBusy(false) }
    }
  }
  const copy = async () => {
    const current = generation.current
    const page = new URL(url)
    const id = page.hostname === 'weixin.qq.com' ? page.pathname.slice(5) : page.searchParams.get('id')
    if (!id || !/^[A-Za-z0-9_-]{1,128}$/.test(id)) { onStatus('原链接格式无效。'); return }
    const ok = await copyTextToClipboard(`https://weixin.qq.com/sph/${id}`)
    if (generation.current === current) onStatus(ok ? '视频号原链接已复制，可粘贴到微信打开。' : '复制失败，请使用页面二维码。')
  }
  return <>
    <button type="button" disabled={busy} aria-busy={busy} title="尝试在微信打开" aria-label="尝试在微信打开" onClick={() => void open()}><MessageCircle size={14} /></button>
    <button type="button" title="复制视频号链接" aria-label="复制视频号链接" onClick={() => void copy()}><Copy size={14} /></button>
  </>
}
