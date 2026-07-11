import { useCallback, useEffect, useRef, useState } from 'react'
import {
  clearFitRunCodexLaunch,
  fitRunStartCommandId,
  persistFitRunCodexLaunch,
  readFitRunCodexLaunch,
  requestCodexForFitRun,
} from './fitRunEvents'
import type { FitRunCommandInput, FitRunDocument } from './types'

const AUTO_CODEX_STORAGE_KEY = 'elon.uiTuner.autoCodexFit'

interface FitRunCodexHandoffOptions {
  run: FitRunDocument | null
  command: (command: FitRunCommandInput) => Promise<FitRunDocument>
}

interface RetryState {
  handoffKey: string
  attempt: number
}

export function useFitRunCodexHandoff({ run, command }: FitRunCodexHandoffOptions) {
  const [autoCodex, setAutoCodexState] = useState(readAutoCodexPreference)
  const [launching, setLaunching] = useState(false)
  const [error, setError] = useState('')
  const [retryState, setRetryState] = useState<RetryState>({ handoffKey: '', attempt: 0 })
  const launchingRef = useRef(false)
  const latestRef = useRef({ run, command })
  latestRef.current = { run, command }

  const handoffKey = run?.handoff ? `${run.runId}:${run.handoff.handoffId}` : ''
  const awaitingHandoff = run?.phase === 'AWAITING_CODEX' && Boolean(handoffKey)
  const retryAttempt = retryState.handoffKey === handoffKey ? retryState.attempt : 0

  const launch = useCallback(async () => {
    const current = latestRef.current.run
    const handoff = current?.handoff
    if (!current || !handoff || current.phase !== 'AWAITING_CODEX' || launchingRef.current) return
    const key = `${current.runId}:${handoff.handoffId}`
    launchingRef.current = true
    setLaunching(true)
    setError('')
    try {
      let launchRecord = readFitRunCodexLaunch(current.runId, handoff.handoffId)
      if (!launchRecord) {
        const { taskId } = await requestCodexForFitRun({
          runId: current.runId,
          handoffId: handoff.handoffId,
          handoffPath: handoff.artifactPath,
          reason: handoff.reason,
        })
        launchRecord = {
          runId: current.runId,
          handoffId: handoff.handoffId,
          taskId,
          createdAt: new Date().toISOString(),
        }
        persistFitRunCodexLaunch(launchRecord)
      }
      await latestRef.current.command({
        type: 'CODEX_STARTED',
        commandId: fitRunStartCommandId(current.runId, handoff.handoffId, launchRecord.taskId),
        handoffId: handoff.handoffId,
        taskId: launchRecord.taskId,
      })
      clearFitRunCodexLaunch(current.runId, handoff.handoffId)
      setRetryState({ handoffKey: key, attempt: 0 })
    } catch (launchError) {
      setError(launchError instanceof Error ? launchError.message : 'Codex 自动接力失败')
      setRetryState((previous) => ({
        handoffKey: key,
        attempt: previous.handoffKey === key ? previous.attempt + 1 : 1,
      }))
    } finally {
      launchingRef.current = false
      setLaunching(false)
    }
  }, [])

  useEffect(() => {
    if (!autoCodex || !awaitingHandoff || launching) return undefined
    const timer = window.setTimeout(() => { void launch() }, retryDelay(retryAttempt))
    return () => window.clearTimeout(timer)
  }, [autoCodex, awaitingHandoff, handoffKey, launch, launching, retryAttempt])

  const setAutoCodex = useCallback((enabled: boolean) => {
    setAutoCodexState(enabled)
    try {
      window.localStorage.setItem(AUTO_CODEX_STORAGE_KEY, String(enabled))
    } catch {
      // Keep the current-session preference when browser storage is unavailable.
    }
  }, [])

  return { autoCodex, setAutoCodex, launching, error, launch, retryAttempt }
}

function retryDelay(attempt: number) {
  if (attempt <= 0) return 0
  return Math.min(1_000 * (2 ** Math.min(attempt - 1, 5)), 30_000)
}

function readAutoCodexPreference() {
  try {
    return window.localStorage.getItem(AUTO_CODEX_STORAGE_KEY) !== 'false'
  } catch {
    return true
  }
}
