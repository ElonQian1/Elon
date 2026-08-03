import { lazy, Suspense, useEffect, useState } from 'react'
import type { SourcePreviewMode } from '../source-preview/types'

const HeadlessDesignWorkspace = lazy(() => (
  import('./HeadlessDesignWorkspace')
    .then((module) => ({ default: module.HeadlessDesignWorkspace }))
))

interface Props {
  active: boolean
  initialProjectRoot: string
  onModeChange: (mode: SourcePreviewMode) => void
}

export function LazyHeadlessDesignWorkspace({ active, initialProjectRoot, onModeChange }: Props) {
  const [loaded, setLoaded] = useState(active)

  useEffect(() => {
    if (active) setLoaded(true)
  }, [active])

  if (!loaded) return null
  return (
    <Suspense fallback={null}>
      <HeadlessDesignWorkspace
        active={active}
        initialProjectRoot={initialProjectRoot}
        onModeChange={onModeChange}
      />
    </Suspense>
  )
}
