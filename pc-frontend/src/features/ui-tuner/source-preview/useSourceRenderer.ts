import { useCallback, useState } from 'react'
import { loadSourceRenderers, renderComposePreview } from './sourcePreviewApi'
import type { ComposePreviewRender, SourceRendererCapabilities } from './types'

export function useSourceRenderer() {
  const [capabilities, setCapabilities] = useState<SourceRendererCapabilities | null>(null)
  const [render, setRender] = useState<ComposePreviewRender | null>(null)
  const [rendering, setRendering] = useState(false)
  const [error, setError] = useState('')
  const [selectedPreviewId, setSelectedPreviewId] = useState('')

  const renderById = useCallback(async (projectRoot: string, next: SourceRendererCapabilities, previewId?: string) => {
    const preview = next.composePreviews.find((entry) => entry.id === previewId) ?? next.composePreviews[0]
    if (!preview || !next.layoutlib.available) return null
    setRendering(true); setError(''); setSelectedPreviewId(preview.id)
    try {
      const result = await renderComposePreview(projectRoot, preview)
      setRender(result)
      return result
    } catch (reason) {
      setRender(null); setError(reason instanceof Error ? reason.message : String(reason))
      return null
    } finally { setRendering(false) }
  }, [])

  const refresh = useCallback(async (projectRoot: string) => {
    setRendering(true); setError('')
    try {
      const next = await loadSourceRenderers(projectRoot)
      setCapabilities(next)
      if (next.layoutlib.available && next.composePreviews.length > 0) await renderById(projectRoot, next, selectedPreviewId)
      return next
    } catch (reason) {
      setCapabilities(null); setRender(null); setError(reason instanceof Error ? reason.message : String(reason))
      return null
    } finally { setRendering(false) }
  }, [renderById, selectedPreviewId])

  const rerender = useCallback(async (projectRoot: string, previewId?: string) => {
    if (!capabilities) return null
    return renderById(projectRoot, capabilities, previewId)
  }, [capabilities, renderById])

  const beginLocalDraft = useCallback(() => { setRender(null) }, [])
  return { capabilities, render, rendering, error, selectedPreviewId, refresh, rerender, beginLocalDraft }
}
