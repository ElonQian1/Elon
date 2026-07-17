import { useEffect, useRef } from 'react'

export function useProjectDocumentGraphFreshness(input: {
  catalogManifestRevision?: string
  expectedManifestRevision?: string
  refresh: () => Promise<void>
}) {
  const lastRefresh = useRef('')

  useEffect(() => {
    const catalogRevision = input.catalogManifestRevision?.trim()
    const expectedRevision = input.expectedManifestRevision?.trim()
    if (!catalogRevision || !expectedRevision || catalogRevision === expectedRevision) {
      if (catalogRevision === expectedRevision) lastRefresh.current = ''
      return
    }
    const refreshKey = `${catalogRevision}:${expectedRevision}`
    if (lastRefresh.current === refreshKey) return
    lastRefresh.current = refreshKey
    void input.refresh()
  }, [input.catalogManifestRevision, input.expectedManifestRevision, input.refresh])
}
