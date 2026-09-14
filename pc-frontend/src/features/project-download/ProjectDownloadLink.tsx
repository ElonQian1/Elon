import type { ReactNode } from 'react'
import { cloudResourceUrl } from '../../lib/cloudResourceUrl'

/** A same-window download works with WebView2 and does not require a popup. */
export default function ProjectDownloadLink({ url, enabled, memberProtected, platform, className, onMemberDownload, children }: {
  url?: string
  enabled: boolean
  memberProtected: boolean
  platform: string
  className: string
  onMemberDownload: () => void
  children: ReactNode
}) {
  const href = cloudResourceUrl(url)
  if (!enabled || !href || memberProtected) {
    return <button className={className} type="button" disabled={!enabled || !href} onClick={onMemberDownload}>{children}</button>
  }
  return <a className={className} href={href} download={platform === 'web' || platform === 'ios' ? undefined : true}>{children}</a>
}
