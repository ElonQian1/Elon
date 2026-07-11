import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import type { SourcePreviewDocument } from './types'

export function sourcePreviewAdminUrl(): string {
  return safeNodeAdminUrl(new URLSearchParams(window.location.search).get('node_admin'))
}
export async function loadSourcePreview(projectRoot: string, layoutFile?: string): Promise<SourcePreviewDocument> {
  const baseUrl = sourcePreviewAdminUrl()
  await probeLocalNode(baseUrl)
  return nodeApi<SourcePreviewDocument>(baseUrl, '/api/source-preview/load', {
    method: 'POST',
    body: JSON.stringify({ projectRoot, layoutFile }),
  }, 15000)
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
