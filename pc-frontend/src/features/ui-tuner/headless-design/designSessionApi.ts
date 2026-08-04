import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi, nodeApiBlob, probeLocalNode } from '../../node/localNodeApi'
import type {
  DesignCaptureResult,
  DesignBrowserInteraction,
  DesignBrowserResult,
  DesignCapabilities,
  DesignDraft,
  DesignDraftPreviewResult,
  DesignDraftSummary,
  DesignPlatform,
  DesignSessionListResult,
  DesignSessionRecord,
  DesignSurface,
  DesignSourceBindingCandidates,
  DesignTargetListResult,
  DesignTaskBindingResult,
  DesignEventPage,
  DesignViewport,
  DesignWritebackReceipt,
  DesignVerificationMatrix,
  TauriBehaviorEvidence,
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

export async function callDesignNode<T>(path: string, body: Record<string, unknown>, timeoutMs = 30000): Promise<T> {
  const baseUrl = adminUrl()
  await probeLocalNode(baseUrl)
  const response = await nodeApi<NodeResult<T>>(baseUrl, path, {
    method: 'POST',
    body: JSON.stringify(body),
  }, timeoutMs)
  if (!response.ok) throw new Error(response.error || '后台设计节点请求失败')
  return response.result
}

const call = callDesignNode

export function listDesignTargets(projectRoot: string) {
  return call<DesignTargetListResult>('/api/android-live/design/targets', { projectRoot })
}

export function getDesignCapabilities(projectRoot: string) {
  return call<DesignCapabilities>('/api/android-live/design/capabilities', { projectRoot })
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

export function prepareDesignBrowser(input: {
  projectRoot: string
  designSessionId: string
  restart?: boolean
  fixtureProfile?: string
}) {
  const capture = input.fixtureProfile ? { fixtureProfile: input.fixtureProfile } : undefined
  return call<DesignBrowserResult>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/browser/prepare`,
    { projectRoot: input.projectRoot, restart: input.restart ?? false, capture },
    60_000,
  )
}

export function interactDesignBrowser(input: {
  projectRoot: string
  designSessionId: string
  step: DesignBrowserInteraction
  fixtureProfile?: string
}) {
  return call<DesignBrowserResult>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/browser/interact`,
    { projectRoot: input.projectRoot, capture: {
      steps: [input.step],
      ...(input.fixtureProfile ? { fixtureProfile: input.fixtureProfile } : {}),
    } },
    60_000,
  )
}

export function stopDesignBrowser(input: { projectRoot: string; designSessionId: string }) {
  return call<DesignBrowserResult>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/browser/stop`,
    { projectRoot: input.projectRoot },
    30_000,
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

export function captureTauriBehavior(input: { projectRoot: string; designSessionId: string }) {
  return call<{ ok: boolean; status: string; nativeBehavior: TauriBehaviorEvidence }>(
    `/api/android-live/design/sessions/${encodeURIComponent(input.designSessionId)}/tauri/behavior`,
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

export async function listDesignDrafts(projectRoot: string, designSessionId?: string) {
  return call<{ schemaVersion: 1 | 2; drafts: DesignDraftSummary[]; contentEmbedded: false }>(
    '/api/android-live/design/drafts/list',
    { projectRoot, designSessionId: designSessionId || undefined },
  )
}

export async function createDesignDraft(input: {
  projectRoot: string
  designSessionId: string
  selector: string
  scope?: DesignDraft['scope']
  patches: DesignDraft['patches']
  operations?: DesignDraft['operations']
  targetPlatforms: DesignPlatform[]
}) {
  return call<{ draft: DesignDraft; next: string }>('/api/android-live/design/drafts', input)
}

export async function getDesignDraft(projectRoot: string, draftId: string) {
  return call<{ draft: DesignDraft; historyDepth: number; contentEmbedded: false }>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}`,
    { projectRoot },
  )
}

export async function updateDesignDraft(input: {
  projectRoot: string
  draftId: string
  expectedRevision: number
  patches?: DesignDraft['patches']
  operations?: DesignDraft['operations']
  sourceBinding?: DesignDraft['sourceBinding']
  targetPlatforms?: DesignPlatform[]
}) {
  const { draftId, ...body } = input
  return call<{ draft: DesignDraft; historyDepth: number }>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}/update`, body,
  )
}

export async function undoDesignDraft(input: {
  projectRoot: string
  draftId: string
  expectedRevision: number
}) {
  const { draftId, ...body } = input
  return call<{ draft: DesignDraft; historyDepth: number }>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}/undo`, body,
  )
}

export async function beginDesignWriteback(input: {
  projectRoot: string
  draftId: string
  expectedRevision: number
  writebackPlanId: string
}) {
  const { draftId, ...body } = input
  return call<{ draft: DesignDraft; writebackPlanId: string; receipt: DesignWritebackReceipt; next: string }>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}/writeback/begin`, body,
  )
}

export function getDesignVerificationMatrix(projectRoot: string, draftId: string) {
  return call<DesignVerificationMatrix>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}/verification-matrix`,
    { projectRoot },
  )
}

export function previewDesignDraft(projectRoot: string, draftId: string) {
  return call<DesignDraftPreviewResult>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}/preview`,
    { projectRoot },
    60_000,
  )
}

export function restoreDesignDraftPreview(projectRoot: string, draftId: string) {
  return call<DesignDraftPreviewResult>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}/preview/restore`,
    { projectRoot },
    60_000,
  )
}

export function suggestDesignSourceBinding(projectRoot: string, draftId: string, limit = 8) {
  return call<DesignSourceBindingCandidates>(
    `/api/android-live/design/drafts/${encodeURIComponent(draftId)}/source-binding/candidates`,
    { projectRoot, limit },
    30_000,
  )
}

export function bindDesignTask(input: {
  projectRoot: string
  taskId: string
  designSessionId: string
  draftId?: string
  expectedLeaseId?: string
  leaseSeconds?: number
}) {
  const { taskId, ...body } = input
  return call<DesignTaskBindingResult>(
    `/api/android-live/design/tasks/${encodeURIComponent(taskId)}/bind`, body,
  )
}

export function getDesignTaskBinding(projectRoot: string, taskId: string) {
  return call<DesignTaskBindingResult>(
    `/api/android-live/design/tasks/${encodeURIComponent(taskId)}/binding`, { projectRoot },
  )
}

export function renewDesignTaskBinding(input: {
  projectRoot: string
  taskId: string
  leaseId: string
  leaseSeconds?: number
}) {
  const { taskId, ...body } = input
  return call<DesignTaskBindingResult>(
    `/api/android-live/design/tasks/${encodeURIComponent(taskId)}/renew`, body,
  )
}

export function settleDesignTaskBinding(input: {
  projectRoot: string
  taskId: string
  leaseId: string
  succeeded?: boolean
}) {
  const { taskId, ...body } = input
  return call<DesignTaskBindingResult>(
    `/api/android-live/design/tasks/${encodeURIComponent(taskId)}/settle`, body,
  )
}

export function listDesignEvents(input: {
  projectRoot: string
  taskId?: string
  afterCursor?: string
  limit?: number
  waitMs?: number
}) {
  return call<DesignEventPage>('/api/android-live/design/events', input, Math.max(30_000, (input.waitMs ?? 0) + 5_000))
}
