import { nodeApi } from '../../node/localNodeApi'
import { inspectorAdminUrl } from '../device/deviceInspectorApi'
import type { CreateFitRunInput, FitBatchAcceptResult, FitRunCommand, FitRunDocument } from './types'

function fitRunsPath(sessionId: string) {
  return `/api/android-live/sessions/${encodeURIComponent(sessionId)}/fit-runs`
}

function fitRunPath(sessionId: string, runId: string) {
  return `${fitRunsPath(sessionId)}/${encodeURIComponent(runId)}`
}

export async function createFitRun(sessionId: string, input: CreateFitRunInput) {
  const response = await nodeApi<{ run: FitRunDocument }>(
    inspectorAdminUrl(),
    fitRunsPath(sessionId),
    { method: 'POST', body: JSON.stringify(input) },
    90_000,
  )
  return response.run
}

export async function listFitRuns(sessionId: string) {
  const response = await nodeApi<{ runs: FitRunDocument[] }>(
    inspectorAdminUrl(),
    fitRunsPath(sessionId),
    {},
    10_000,
  )
  return response.runs
}

export async function getFitRun(sessionId: string, runId: string) {
  const response = await nodeApi<{ run: FitRunDocument }>(
    inspectorAdminUrl(),
    fitRunPath(sessionId, runId),
    {},
    10_000,
  )
  return response.run
}

export async function sendFitRunCommand(
  sessionId: string,
  runId: string,
  command: FitRunCommand,
) {
  const response = await nodeApi<{ run: FitRunDocument; idempotent?: boolean }>(
    inspectorAdminUrl(),
    `${fitRunPath(sessionId, runId)}/commands`,
    { method: 'POST', body: JSON.stringify(command) },
    20 * 60_000,
  )
  return response
}

export function fitRunCommandId(type: FitRunCommand['type']) {
  const random = globalThis.crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2)
  return `${type.toLowerCase()}-${Date.now()}-${random}`
}

export async function acceptFitRunBatch(
  sessionId: string,
  runIds: string[],
  sourceRevision: string,
  codexCompleted = false,
) {
  const response = await nodeApi<{ result: FitBatchAcceptResult }>(
    inspectorAdminUrl(),
    `${fitRunsPath(sessionId)}/batch-accept`,
    {
      method: 'POST',
      body: JSON.stringify({ runIds, sourceRevision, codexCompleted }),
    },
    20 * 60_000,
  )
  return response.result
}
