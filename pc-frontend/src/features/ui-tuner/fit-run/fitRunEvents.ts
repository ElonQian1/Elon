export const FIT_RUN_CODEX_REQUEST_EVENT = 'elon:ui-fit-run-codex-request'
export const FIT_RUN_CODEX_SETTLED_EVENT = 'elon:ui-fit-run-codex-settled'

export interface FitRunCodexRequest {
  runId: string
  handoffId: string
  handoffPath?: string
  reason: string
  handoffKind?: 'FIT_RUN' | 'PWA_DRAFT'
  contextPack?: UiTunerCodexContextPack
}

interface FitRunCodexRequestDetail extends FitRunCodexRequest {
  resolve: (value: { taskId: string }) => void
  reject: (error: Error) => void
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

export function requestCodexForFitRun(request: FitRunCodexRequest) {
  return new Promise<{ taskId: string }>((resolve, reject) => {
    window.dispatchEvent(new CustomEvent<FitRunCodexRequestDetail>(
      FIT_RUN_CODEX_REQUEST_EVENT,
      { detail: { ...request, resolve, reject } },
    ))
  })
}

export function listenForFitRunCodexRequests(
  listener: (detail: FitRunCodexRequestDetail) => void,
) {
  const handler = (event: Event) => listener(
    (event as CustomEvent<FitRunCodexRequestDetail>).detail,
  )
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
import type { UiTunerCodexContextPack } from '../contextPack'
