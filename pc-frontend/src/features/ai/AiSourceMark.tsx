import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  aiSiteIdentity,
  aiSourceIconCandidates,
  type AiSourcePresentationInput,
} from './aiSourcePresentation'
import styles from './AiSourceMark.module.css'

interface AiSourceMarkProps {
  source: AiSourcePresentationInput
  variant?: 'card' | 'compact' | 'inline'
}

const ICON_LOAD_TIMEOUT_MS = 2_500

export default function AiSourceMark({ source, variant = 'card' }: AiSourceMarkProps) {
  const identity = aiSiteIdentity(source.url)
  const [failedUrls, setFailedUrls] = useState<string[]>([])
  const [loadedUrl, setLoadedUrl] = useState('')
  const iconCandidates = useMemo(
    () => aiSourceIconCandidates(source, identity.host),
    [identity.host, source.icon_url, source.url],
  )
  const candidateKey = iconCandidates.join('\n')
  const iconUrl = iconCandidates
    .find((candidate) => !failedUrls.includes(candidate)) ?? ''

  const rejectIcon = useCallback((url: string) => {
    setLoadedUrl((value) => value === url ? '' : value)
    setFailedUrls((values) => values.includes(url) ? values : [...values, url])
  }, [])

  useEffect(() => {
    setFailedUrls([])
    setLoadedUrl('')
  }, [candidateKey])

  useEffect(() => {
    if (!iconUrl || loadedUrl === iconUrl) return
    const timer = window.setTimeout(() => rejectIcon(iconUrl), ICON_LOAD_TIMEOUT_MS)
    return () => window.clearTimeout(timer)
  }, [iconUrl, loadedUrl, rejectIcon])

  return (
    <span
      className={[styles.mark, styles[variant]].join(' ')}
      data-tone={identity.tone}
      aria-hidden="true"
    >
      <span>{identity.initial}</span>
      {iconUrl && (
        <img
          className={[styles.logo, loadedUrl === iconUrl ? styles.logoReady : ''].join(' ')}
          src={iconUrl}
          alt=""
          loading="lazy"
          decoding="async"
          referrerPolicy="no-referrer"
          onLoad={() => setLoadedUrl(iconUrl)}
          onError={() => rejectIcon(iconUrl)}
        />
      )}
    </span>
  )
}
