import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi, nodeApiBlob, probeLocalNode } from '../../node/localNodeApi'
import type {
  DesignCaptureResult,
  DesignPlatform,
  DesignSessionListResult,
  DesignSessionRecord,
  DesignSurface,
  DesignTargetListResult,
  DesignViewport,
  TauriRuntimeResult,
} from './types'

interface NodeResult<T> {
  ok: boolean
  result: T
  error?: string
}

function adminUrl() {
  return safeNodeAdminUrl()
}

async function call<T>(path: string, body: Record<string, unknown>, timeoutMs = 30000): Promise<T> {
  const baseUrl = adminUrl()
  await probeLocalNode(baseUrl)
  const response = await nodeApi<NodeResult<T>>(baseUrl, path, {
    method: 'POST',
    body: JSON.stringify(body),
  }, timeoutMs)
  if (!response.ok) throw new Error(response.error || '后台设计节点请求失败')
  return response.result
}

export function listDesignTargets(projectRoot: string) {
  return call<DesignTargetListResult>('/api/android-live/design/targets', { projectRoot })
}

export function listDesignSessions(projectRoot: string, limit = 20) {
  return call<DesignSessionListResult>('/api/android-live/design/sessions/list', {
    projectRoot,
    limit,
  })
}

export async function openDesignSession(input: {
  projectRoot: string
  platform: DesignPlatform
  route: string
  url?: string
  viewport: DesignViewport
}): Promise<DesignSessionRecord> {
  const result = await call<{ session: DesignSessionRecord }>(
    '/api/android-live/design/sessions',
    input,
  )
  return result.session
}

export function captureDesignSession(input: {
  projectRoot: string
  designSessionId: string
  capture?: Record<string, unknown>
}) {
  return call<DesignCaptureResult>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/capture`,
    { projectRoot: input.projectRoot, ...(input.capture ? { capture: input.capture } : {}) },
    60000,
  )
}

export function getDesignSurface(input: {
  projectRoot: string
  designSessionId: string
  query?: string
  limit?: number
}) {
  return call<DesignSurface>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/surface`,
    {
      projectRoot: input.projectRoot,
      query: input.query || undefined,
      limit: input.limit ?? 80,
    },
  )
}

export async function loadDesignPixel(projectRoot: string, designSessionId: string): Promise<Blob> {
  const baseUrl = adminUrl()
  await probeLocalNode(baseUrl)
  return nodeApiBlob(
    baseUrl,
    `/api/android-live/design/sessions/${encodeURIComponent(designSessionId)}/artifact`,
    { method: 'POST', body: JSON.stringify({ projectRoot }) },
    30000,
  )
}

export function prepareTauriRuntime(input: { projectRoot: string; designSessionId: string; restart?: boolean }) {
  return call<TauriRuntimeResult>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/tauri/prepare`,
    { projectRoot: input.projectRoot, restart: input.restart ?? false },
    30_000,
  )
}

export function captureTauriHost(input: { projectRoot: string; designSessionId: string }) {
  return call<TauriRuntimeResult>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/tauri/capture`,
    { projectRoot: input.projectRoot },
    30_000,
  )
}

export function stopTauriRuntime(input: { projectRoot: string; designSessionId: string }) {
  return call<TauriRuntimeResult>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/tauri/stop`,
    { projectRoot: input.projectRoot },
    20_000,
  )
}

export async function loadTauriNativePixel(projectRoot: string, designSessionId: string): Promise<Blob> {
  const baseUrl = adminUrl()
  await probeLocalNode(baseUrl)
  return nodeApiBlob(
    baseUrl,
    `/api/android-live/design/sessions/${encodeURIComponent(designSessionId)}/tauri/artifact`,
    { method: 'POST', body: JSON.stringify({ projectRoot }) },
    30_000,
  )
}
