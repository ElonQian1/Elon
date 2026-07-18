export const FIT_RUN_CODEX_REQUEST_EVENT = 'elon:ui-fit-run-codex-request'
export const FIT_RUN_CODEX_SETTLED_EVENT = 'elon:ui-fit-run-codex-settled'

export interface FitRunCodexRequest {
  runId: string
  handoffId: string
  handoffPath?: string
  workspacePath?: string
  reason: string
  handoffKind?: 'FIT_RUN' | 'PWA_DRAFT'
  contextPack?: UiTunerCodexContextPack
}

export interface FitRunWorkspaceResolution {
  workspacePath: string
  isOverride: boolean
}

export function resolveFitRunWorkspace(
  request: Pick<FitRunCodexRequest, 'workspacePath' | 'contextPack'>,
  defaultWorkspacePath: string,
): FitRunWorkspaceResolution {
  const fallback = defaultWorkspacePath.trim()
  const requested = request.workspacePath?.trim() ?? ''
  if (!requested) return { workspacePath: fallback, isOverride: false }

  const artifactSourceRoot = request.contextPack?.screen.sourceRoot?.trim() ?? ''
  if (!artifactSourceRoot || normalizeWorkspacePath(artifactSourceRoot) !== normalizeWorkspacePath(requested)) {
    throw new Error('PWA 草稿源码目录与 AI Context Artifact 不一致，已阻止写入错误工作区')
  }
  return {
    workspacePath: requested,
    isOverride: !fallback || normalizeWorkspacePath(requested) !== normalizeWorkspacePath(fallback),
  }
}

interface FitRunCodexRequestDetail extends FitRunCodexRequest {
  resolve: (value: { taskId: string }) => void
  reject: (error: Error) => void
  markHandled: () => void
}

export interface FitRunCodexSettledDetail {
  taskId: string
  succeeded: boolean
  settledAt?: string
}

const SETTLEMENT_STORAGE_KEY = 'elon.uiTuner.fitRunCodexSettlements.v1'
const LAUNCH_STORAGE_KEY = 'elon.uiTuner.fitRunCodexLaunches.v1'
const MAX_PERSISTED_SETTLEMENTS = 40

export interface FitRunCodexLaunchRecord {
  runId: string
  handoffId: string
  taskId: string
  createdAt: string
}

export function requestCodexForFitRun(request: FitRunCodexRequest, timeoutMs = 30_000) {
  return new Promise<{ taskId: string }>((resolve, reject) => {
    let handled = false
    let settled = false
    const finish = (callback: () => void) => {
      if (settled) return
      settled = true
      window.clearTimeout(timeout)
      callback()
    }
    const timeout = window.setTimeout(() => {
      finish(() => reject(new Error(handled
        ? 'AI 项目会话启动超时，请确认本机节点在线后重试；草稿仍已保留'
        : '当前页面的 AI 项目会话入口未就绪，请刷新页面或重新选择项目后重试；草稿仍已保留')))
    }, timeoutMs)
    window.dispatchEvent(new CustomEvent<FitRunCodexRequestDetail>(
      FIT_RUN_CODEX_REQUEST_EVENT,
      { detail: {
        ...request,
        markHandled: () => { handled = true },
        resolve: (value) => finish(() => resolve(value)),
        reject: (error) => finish(() => reject(error)),
      } },
    ))
    Promise.resolve().then(() => {
      if (!handled) finish(() => reject(new Error(
        '当前页面的 AI 项目会话入口未就绪，请刷新页面或重新选择项目后重试；草稿仍已保留',
      )))
    })
  })
}

export function listenForFitRunCodexRequests(
  listener: (detail: FitRunCodexRequestDetail) => void,
) {
  const handler = (event: Event) => {
    const detail = (event as CustomEvent<FitRunCodexRequestDetail>).detail
    detail.markHandled()
    try {
      listener(detail)
    } catch (error) {
      detail.reject(error instanceof Error ? error : new Error('AI 项目会话接力失败'))
    }
  }
  window.addEventListener(FIT_RUN_CODEX_REQUEST_EVENT, handler)
  return () => window.removeEventListener(FIT_RUN_CODEX_REQUEST_EVENT, handler)
}

export function notifyFitRunCodexSettled(detail: FitRunCodexSettledDetail) {
  const persisted = { ...detail, settledAt: detail.settledAt ?? new Date().toISOString() }
  persistSettlement(persisted)
  window.dispatchEvent(new CustomEvent<FitRunCodexSettledDetail>(
    FIT_RUN_CODEX_SETTLED_EVENT,
    { detail: persisted },
  ))
}

export function readFitRunCodexSettlement(taskId: string) {
  return readSettlements().find((item) => item.taskId === taskId) ?? null
}

export function clearFitRunCodexSettlement(taskId: string) {
  writeSettlements(readSettlements().filter((item) => item.taskId !== taskId))
}

export function fitRunSettlementCommandId(
  runId: string,
  handoffId: string,
  detail: FitRunCodexSettledDetail,
) {
  const outcome = detail.succeeded ? 'completed' : 'failed'
  return `codex-${outcome}:${runId}:${handoffId}:${detail.taskId}`
}

export function fitRunStartCommandId(runId: string, handoffId: string, taskId: string) {
  return `codex-started:${runId}:${handoffId}:${taskId}`
}

export function readFitRunCodexLaunch(runId: string, handoffId: string) {
  return readLaunches().find((item) => item.runId === runId && item.handoffId === handoffId) ?? null
}

export function persistFitRunCodexLaunch(record: FitRunCodexLaunchRecord) {
  const previous = readLaunches().filter((item) => (
    item.runId !== record.runId || item.handoffId !== record.handoffId
  ))
  writeLaunches([...previous, record].slice(-MAX_PERSISTED_SETTLEMENTS))
}

export function clearFitRunCodexLaunch(runId: string, handoffId: string) {
  writeLaunches(readLaunches().filter((item) => (
    item.runId !== runId || item.handoffId !== handoffId
  )))
}

export function listenForFitRunCodexSettled(
  listener: (detail: FitRunCodexSettledDetail) => void,
) {
  const handler = (event: Event) => listener(
    (event as CustomEvent<FitRunCodexSettledDetail>).detail,
  )
  window.addEventListener(FIT_RUN_CODEX_SETTLED_EVENT, handler)
  return () => window.removeEventListener(FIT_RUN_CODEX_SETTLED_EVENT, handler)
}

function persistSettlement(detail: FitRunCodexSettledDetail) {
  const previous = readSettlements().filter((item) => item.taskId !== detail.taskId)
  writeSettlements([...previous, detail].slice(-MAX_PERSISTED_SETTLEMENTS))
}

function readSettlements(): FitRunCodexSettledDetail[] {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(SETTLEMENT_STORAGE_KEY) ?? '[]')
    if (!Array.isArray(parsed)) return []
    return parsed.filter((item): item is FitRunCodexSettledDetail => (
      typeof item?.taskId === 'string' && typeof item?.succeeded === 'boolean'
    ))
  } catch {
    return []
  }
}

function writeSettlements(items: FitRunCodexSettledDetail[]) {
  try {
    window.localStorage.setItem(SETTLEMENT_STORAGE_KEY, JSON.stringify(items))
  } catch {
    // Storage can be unavailable in hardened browser contexts; the live event still works.
  }
}

function readLaunches(): FitRunCodexLaunchRecord[] {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(LAUNCH_STORAGE_KEY) ?? '[]')
    if (!Array.isArray(parsed)) return []
    return parsed.filter((item): item is FitRunCodexLaunchRecord => (
      typeof item?.runId === 'string'
      && typeof item?.handoffId === 'string'
      && typeof item?.taskId === 'string'
    ))
  } catch {
    return []
  }
}

function writeLaunches(items: FitRunCodexLaunchRecord[]) {
  try {
    window.localStorage.setItem(LAUNCH_STORAGE_KEY, JSON.stringify(items))
  } catch {
    // The in-memory launch lock remains active when storage is unavailable.
  }
}

function normalizeWorkspacePath(value: string) {
  return value.trim().replace(/\//g, '\\').replace(/\\+$/, '').toLocaleLowerCase('en-US')
}
import type { UiTunerCodexContextPack } from '../contextPack'
