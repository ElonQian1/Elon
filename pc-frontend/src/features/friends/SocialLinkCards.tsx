import { useEffect, useRef, useState } from 'react'
import { getAuthToken } from '../../api/client'
import { resolveApiUrl } from '../../api/runtime'
import { getDesktopInvoke } from '../shell/desktopShell'
import type { LinkPreview } from './socialLinks'
import SocialLinkBrowser from './SocialLinkBrowser'
import '../../../../server/src/assets/social_links.js'
import '../../../../server/src/assets/social_link_viewer.js'
import '../../../../server/src/assets/social_links.css'

async function previewApi(path: string, init: RequestInit) {
  const token = getAuthToken()
  const response = await fetch(resolveApiUrl(path), { ...init, headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) } })
  if (!response.ok) throw new Error('预览暂不可用')
  return response.json()
}
export default function SocialLinkCards({ text, owner }: { text: string; owner: string }) {
  const host = useRef<HTMLDivElement>(null)
  const [reading, setReading] = useState<LinkPreview | null>(null)
  useEffect(() => {
    if (!host.current) return
    return ElonSocialLinks.mount(host.current, text, { owner, api: previewApi, open: p => {
      if (getDesktopInvoke() && p.embed?.kind !== 'x') setReading(p)
      else ElonSocialLinkViewer.open(p)
    } })
  }, [text, owner])
  useEffect(() => () => { ElonSocialLinkViewer.close() }, [owner, text])
  return <><div ref={host} />{reading && <SocialLinkBrowser preview={reading} onClose={() => setReading(null)} />}</>
}
