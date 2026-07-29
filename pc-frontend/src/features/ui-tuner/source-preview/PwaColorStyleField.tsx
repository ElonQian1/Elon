import { lazy, Suspense } from 'react'
import type { PwaStyleProperty } from './pwaDesignDraft'
import type { PwaDesignSession } from './usePwaDesignSession'
import styles from './SourcePreview.module.css'

const PwaColorStyleFieldControl = lazy(() => import('./PwaColorStyleFieldControl'))

interface Props {
  label: string
  property: PwaStyleProperty
  value: string
  session: PwaDesignSession
}

export function PwaColorStyleField({ label, property, value, session }: Props) {
  return (
    <Suspense fallback={(
      <label className={styles.pwaStyleField}>
        <span>{label}</span>
        <input
          aria-label={`${label}颜色值`}
          value={value}
          onChange={(event) => session.updateStyle(property, event.currentTarget.value)}
        />
      </label>
    )}>
      <PwaColorStyleFieldControl label={label} property={property} value={value} session={session} />
    </Suspense>
  )
}
