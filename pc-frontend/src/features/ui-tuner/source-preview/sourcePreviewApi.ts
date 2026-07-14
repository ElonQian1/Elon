import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import type { ComposePreviewEntry, ComposePreviewRender, SourcePreviewDocument, SourceRendererCapabilities } from './types'

export function sourcePreviewAdminUrl(): string {
  return safeNodeAdminUrl()
}
export async function loadSourcePreview(projectRoot: string, layoutFile?: string): Promise<SourcePreviewDocument> {
  const baseUrl = sourcePreviewAdminUrl()
  await probeLocalNode(baseUrl)
  return nodeApi<SourcePreviewDocument>(baseUrl, '/api/source-preview/load', {
    method: 'POST',
    body: JSON.stringify({ projectRoot, layoutFile }),
  }, 15000)
}

export async function loadSourceRenderers(projectRoot: string): Promise<SourceRendererCapabilities> {
  const baseUrl = sourcePreviewAdminUrl()
  await probeLocalNode(baseUrl)
  return nodeApi<SourceRendererCapabilities>(baseUrl, '/api/source-preview/renderers', {
    method: 'POST', body: JSON.stringify({ projectRoot }),
  }, 30000)
}

export async function renderComposePreview(projectRoot: string, preview: ComposePreviewEntry): Promise<ComposePreviewRender> {
  return nodeApi<ComposePreviewRender>(sourcePreviewAdminUrl(), '/api/source-preview/render-compose', {
    method: 'POST',
    body: JSON.stringify({ projectRoot, kotlinFile: preview.kotlinFile, composable: preview.composable }),
  }, 130000)
}

export async function commitSourcePreview(input: {
  projectRoot: string
  layoutFile: string
  sourceRevision: string
  nodeKey: string
  startTagStart: number
  startTagEnd: number
  changes: Record<string, string>
}): Promise<{ ok: boolean; sourceRevision: string }> {
  return nodeApi(sourcePreviewAdminUrl(), '/api/source-preview/commit', {
    method: 'POST',
    body: JSON.stringify(input),
  }, 15000)
}
