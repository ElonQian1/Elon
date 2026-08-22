import { useState } from 'react'
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

export default function AiSourceMark({ source, variant = 'card' }: AiSourceMarkProps) {
  const identity = aiSiteIdentity(source.url)
  const [failedUrls, setFailedUrls] = useState<string[]>([])
  const iconUrl = aiSourceIconCandidates(source, identity.host)
    .find((candidate) => !failedUrls.includes(candidate)) ?? ''

  return (
    <span
      className={[styles.mark, styles[variant]].join(' ')}
      data-tone={identity.tone}
      aria-hidden="true"
    >
      <span>{identity.initial}</span>
      {iconUrl && (
        <img
          className={styles.logo}
          src={iconUrl}
          alt=""
          loading="lazy"
          decoding="async"
          referrerPolicy="no-referrer"
          onError={() => setFailedUrls((values) => values.includes(iconUrl) ? values : [...values, iconUrl])}
        />
      )}
    </span>
  )
}
