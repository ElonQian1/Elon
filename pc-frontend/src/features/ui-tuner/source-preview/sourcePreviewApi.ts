import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import type { ComposePreviewEntry, ComposePreviewRender, SourcePreviewDocument, SourceRendererCapabilities } from './types'
import type { PwaExplicitStyleBinding } from './pwaDesignDraft'
import type { PwaBuildVerificationResult, PwaSourceSavedEvidence } from './pwaVerificationModel'
import { androidProjectRootCandidates } from './sourcePreviewProjectRoot'

export function sourcePreviewAdminUrl(): string {
  return safeNodeAdminUrl()
}
export async function loadSourcePreview(projectRoot: string, layoutFile?: string): Promise<SourcePreviewDocument> {
  const baseUrl = sourcePreviewAdminUrl()
  await probeLocalNode(baseUrl)
  const candidates = layoutFile ? [projectRoot] : androidProjectRootCandidates(projectRoot)
  let lastError: unknown
  for (const candidate of candidates) {
    try {
      return await nodeApi<SourcePreviewDocument>(baseUrl, '/api/source-preview/load', {
        method: 'POST',
        body: JSON.stringify({ projectRoot: candidate, layoutFile }),
      }, 15000)
    } catch (error) {
      lastError = error
      const message = String(error)
      const canTryAnotherRoot = message.includes('没有找到 src/main/res/layout')
        || message.includes('请选择有效的本机 Android 项目目录')
        || message.includes('无法解析项目目录')
      if (!canTryAnotherRoot) throw error
    }
  }
  throw lastError
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

export async function commitPwaStylePreview(input: {
  projectRoot: string
  binding: PwaExplicitStyleBinding
  sourceRevision: string
  changes: Record<string, string>
}): Promise<{ ok: boolean; sourceRevision: string; changedFiles: string[] }> {
  return nodeApi(sourcePreviewAdminUrl(), '/api/source-preview/commit-pwa-style', {
    method: 'POST',
    body: JSON.stringify(input),
  }, 15000)
}

export async function verifyPwaSourceBuild(
  evidence: PwaSourceSavedEvidence,
): Promise<PwaBuildVerificationResult> {
  return nodeApi(sourcePreviewAdminUrl(), '/api/source-preview/verify-pwa-source', {
    method: 'POST',
    body: JSON.stringify({
      projectRoot: evidence.projectRoot,
      changedFiles: evidence.changedFiles.map((sourceFile) => ({
        sourceFile,
        sourceRevision: evidence.sourceRevisions[sourceFile],
      })),
      expectedValues: evidence.expectedValues,
    }),
  }, 10 * 60_000)
}
