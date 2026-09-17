import { useEffect, useRef } from 'react'
import { resolveApiUrl } from '../../api/runtime'
import { getDesktopInvoke } from '../shell/desktopShell'
import { useReaderTabs } from '../reader/readerTabsStore'
import { previewApi } from './socialReadBack'
import { cachedRead } from './socialReadPreview'
import '../../../../server/src/assets/social_links.js'
import '../../../../server/src/assets/social_link_viewer.js'
import '../../../../server/src/assets/social_links.css'

export default function SocialLinkCards({ text, owner, compact = false }: { text: string; owner: string; compact?: boolean }) {
  const host = useRef<HTMLDivElement>(null)
  const scope = resolveApiUrl('/') + '\n' + owner
  useEffect(() => {
    if (!host.current) return
    for (const item of ElonSocialLinks.links(text)) {
      const cached = cachedRead(scope, item); if (cached) ElonSocialLinks.remember(scope, cached.preview, cached.expires)
    }
    // Desktop opens a reading tab so chat stays interactive; browsers keep the embedded viewer.
    return ElonSocialLinks.mount(host.current, text, { owner: scope, compact, desktop: true, api: previewApi, open: p => {
      if (getDesktopInvoke()) useReaderTabs.getState().open(p, scope)
      else ElonSocialLinkViewer.open(p)
    } })
  }, [text, scope, compact])
  useEffect(() => () => { ElonSocialLinkViewer.close() }, [scope, text])
  return <div ref={host} />
}
