import { safeNodeAdminUrl } from '../../../lib/utils'
import { getAuthIdentityLabel, getAuthToken } from '../../../api/client'
import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import type { ComposePreviewEntry, ComposePreviewRender, SourcePreviewDocument, SourceRendererCapabilities } from './types'
import type { PwaExplicitStyleBinding } from './pwaDesignDraft'
import type { PwaBuildVerificationResult, PwaRuntimeCaptureResult, PwaSourceSavedEvidence } from './pwaVerificationModel'
import { captureWithTemporaryPwaAuthProfile, type PreparedPwaAuthProfile } from './pwaRuntimeAuth'
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
}): Promise<{
  ok: boolean
  sourceRevision: string
  changedFiles?: string[]
  sourceHashes?: Record<string, string>
}> {
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

export async function resolvePwaStyleBinding(input: {
  projectRoot: string
  selectors: string[]
}): Promise<{
  ok: boolean
  binding?: PwaExplicitStyleBinding
  candidateCount: number
  detail: string
}> {
  return nodeApi(sourcePreviewAdminUrl(), '/api/source-preview/resolve-pwa-style-binding', {
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

export async function capturePwaSourceRuntime(
  evidence: PwaSourceSavedEvidence,
  runtimeUrl: string,
): Promise<PwaRuntimeCaptureResult> {
  if (!runtimeUrl.trim()) throw new Error('PWA_RUNTIME_URL_REQUIRED：当前画面没有可捕获的真实 PWA URL')
  const target = new URL(runtimeUrl, window.location.origin)
  if (evidence.route.path?.startsWith('/')) target.pathname = evidence.route.path
  target.search = evidence.route.search || target.search
  target.hash = evidence.route.hash || ''
  const url = target.toString()
  const adminUrl = sourcePreviewAdminUrl()
  const token = getAuthToken()
  const accountLabel = getAuthIdentityLabel()
  return captureWithTemporaryPwaAuthProfile(evidence.projectRoot, token, accountLabel, {
    prepare: (projectRoot, currentToken, currentAccountLabel) => nodeApi<PreparedPwaAuthProfile>(
      adminUrl,
      '/api/source-preview/pwa-auth-profile/prepare',
      {
        method: 'POST',
        body: JSON.stringify({
          projectRoot,
          token: currentToken || undefined,
          remember: true,
          accountLabel: currentAccountLabel || undefined,
        }),
      },
      15_000,
    ),
    capture: (profile) => nodeApi<PwaRuntimeCaptureResult>(
      adminUrl,
      '/api/source-preview/capture-pwa-runtime',
      {
        method: 'POST',
        body: JSON.stringify({
          projectRoot: evidence.projectRoot,
          url,
          viewport: { ...evidence.viewport, deviceScaleFactor: window.devicePixelRatio || 1 },
          waitFor: { condition: 'networkidle', selector: 'body', timeoutMs: 30_000, settleMs: 500 },
          authProfile: profile,
          evidence: {
            sourceRevisions: evidence.sourceRevisions,
            routeRevision: `pwa-draft-r${evidence.draftRevision}`,
          },
        }),
      },
      45_000,
    ),
    cleanup: (projectRoot, profile) => nodeApi(
      adminUrl,
      '/api/source-preview/pwa-auth-profile/cleanup',
      { method: 'POST', body: JSON.stringify({ projectRoot, profile }) },
      15_000,
    ),
  })
}

export async function capturePwaViewportSnapshot(input: {
  projectRoot: string
  sourceRevision: string
  runtimeUrl: string
  route?: { path: string; search: string; hash: string } | null
  viewport: { width: number; height: number; deviceScaleFactor: number }
}): Promise<PwaRuntimeCaptureResult> {
  if (!input.runtimeUrl.trim()) {
    throw new Error('PWA_RUNTIME_URL_REQUIRED：当前画面没有可捕获的真实 PWA URL')
  }
  const target = new URL(input.runtimeUrl, window.location.origin)
  if (input.route?.path?.startsWith('/')) target.pathname = input.route.path
  target.search = input.route?.search || target.search
  target.hash = input.route?.hash || ''
  const adminUrl = sourcePreviewAdminUrl()
  const token = getAuthToken()
  const accountLabel = getAuthIdentityLabel()
  const width = Math.max(240, Math.min(1440, Math.round(input.viewport.width)))
  const height = Math.max(240, Math.min(2048, Math.round(input.viewport.height)))
  const deviceScaleFactor = Math.max(.5, Math.min(4, input.viewport.deviceScaleFactor))
  return captureWithTemporaryPwaAuthProfile(input.projectRoot, token, accountLabel, {
    prepare: (projectRoot, currentToken, currentAccountLabel) => nodeApi<PreparedPwaAuthProfile>(
      adminUrl,
      '/api/source-preview/pwa-auth-profile/prepare',
      {
        method: 'POST',
        body: JSON.stringify({
          projectRoot,
          token: currentToken || undefined,
          remember: true,
          accountLabel: currentAccountLabel || undefined,
        }),
      },
      15_000,
    ),
    capture: (profile) => nodeApi<PwaRuntimeCaptureResult>(
      adminUrl,
      '/api/source-preview/capture-pwa-runtime',
      {
        method: 'POST',
        body: JSON.stringify({
          projectRoot: input.projectRoot,
          url: target.toString(),
          viewport: { width, height, deviceScaleFactor },
          waitFor: { condition: 'networkidle', selector: 'body', timeoutMs: 30_000, settleMs: 500 },
          capture: { fullPage: false },
          authProfile: profile,
          evidence: {
            sourceRevision: input.sourceRevision || 'source-preview-unavailable',
            routeRevision: `viewport-${width}x${height}-${input.route?.path || '/web'}`,
          },
        }),
      },
      45_000,
    ),
    cleanup: (projectRoot, profile) => nodeApi(
      adminUrl,
      '/api/source-preview/pwa-auth-profile/cleanup',
      { method: 'POST', body: JSON.stringify({ projectRoot, profile }) },
      15_000,
    ),
  })
}
