import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { createFitRun, fitRunCommandId, getFitRun, listFitRuns, sendFitRunCommand } from './fitRunApi'
import {
  clearFitRunCodexSettlement,
  fitRunSettlementCommandId,
  listenForFitRunCodexSettled,
  readFitRunCodexSettlement,
} from './fitRunEvents'
import { fitRunPairKey, sameFitRunPair } from './fitRunIdentity'
import { useFitRunStore } from './fitRunStore'
import {
  ACTIVE_FIT_RUN_PHASES,
  TERMINAL_FIT_RUN_PHASES,
  type CreateFitRunInput,
  type FitRunCommand,
  type FitRunCommandInput,
  type FitRunDocument,
} from './types'

interface UseFitRunOptions {
  sessionId?: string
  input?: CreateFitRunInput
  onNotice?: (message: string) => void
}

export function useFitRun({ sessionId, input, onNotice }: UseFitRunOptions) {
  const [run, setRun] = useState<FitRunDocument | null>(null)
  const [busy, setBusy] = useState(false)
  const [settlementBusy, setSettlementBusy] = useState(false)
  const [settlementRevision, setSettlementRevision] = useState(0)
  const [error, setError] = useState('')
  const generationRef = useRef(0)
  const inputRef = useRef(input)
  inputRef.current = input
  const setActiveRun = useFitRunStore((state) => state.setRun)
  const inputKey = useMemo(() => fitRunPairKey(input), [input])

  const updateRun = useCallback((next: FitRunDocument | null) => {
    setRun(next)
    setActiveRun(next)
  }, [setActiveRun])

  useEffect(() => {
    const generation = ++generationRef.current
    updateRun(null)
    setError('')
    const restoreInput = inputRef.current
    if (!sessionId || !restoreInput) {
      setBusy(false)
      return
    }
    setBusy(true)
    void listFitRuns(sessionId).then(async (runs) => {
      const matching = runs.find((candidate) => (
        !TERMINAL_FIT_RUN_PHASES.has(candidate.phase)
        && sameFitRunPair(candidate, restoreInput)
      ))
      if (!matching || generation !== generationRef.current) return
      const restored = matching.sessionId === sessionId ? matching : (await sendFitRunCommand(
        sessionId,
        matching.runId,
        {
          type: 'REBIND_SESSION',
          commandId: fitRunCommandId('REBIND_SESSION'),
          newSessionId: sessionId,
          newRuntimeNodeId: restoreInput.pair.runtimeNodeId,
          newCurrentRect: restoreInput.pair.currentRect,
        },
      )).run
      if (generation === generationRef.current) {
        updateRun(restored)
        onNotice?.('已恢复上次未完成的设计稿拟合任务')
      }
    }).catch((restoreError) => {
      if (generation === generationRef.current) {
        setError(messageOf(restoreError, '无法恢复拟合任务'))
      }
    }).finally(() => {
      if (generation === generationRef.current) setBusy(false)
    })
  }, [inputKey, onNotice, sessionId, updateRun])

  const refresh = useCallback(async () => {
    if (!sessionId || !run?.runId) return null
    const generation = generationRef.current
    try {
      const next = await getFitRun(sessionId, run.runId)
      if (generation === generationRef.current) updateRun(next)
      return next
    } catch (refreshError) {
      if (generation === generationRef.current) {
        setError(messageOf(refreshError, '无法刷新拟合任务'))
      }
      return null
    }
  }, [run?.runId, sessionId, updateRun])

  useEffect(() => {
    if (!run || !ACTIVE_FIT_RUN_PHASES.has(run.phase)) return
    const timer = window.setInterval(() => { void refresh() }, 1_500)
    return () => window.clearInterval(timer)
  }, [refresh, run])

  const start = useCallback(async () => {
    if (!sessionId || !input) throw new Error('请先完成设计稿区域和 Runtime Node 配对')
    setBusy(true)
    setError('')
    try {
      const next = await createFitRun(sessionId, { ...input, autoStart: true })
      updateRun(next)
      onNotice?.('FitRun 已启动：先由本地求解器拟合，出现结构平台期后再交给 Codex')
      return next
    } catch (startError) {
      const message = messageOf(startError, '无法启动拟合任务')
      setError(message)
      onNotice?.(message)
      throw startError
    } finally {
      setBusy(false)
    }
  }, [input, onNotice, sessionId, updateRun])

  const command = useCallback(async (
    value: FitRunCommandInput,
  ) => {
    if (!sessionId || !run) throw new Error('FitRun 尚未创建')
    setBusy(true)
    setError('')
    try {
      const payload = {
        ...value,
        commandId: value.commandId || fitRunCommandId(value.type),
      } as FitRunCommand
      const response = await sendFitRunCommand(sessionId, run.runId, payload)
      updateRun(response.run)
      return response.run
    } catch (commandError) {
      const message = messageOf(commandError, 'FitRun 操作失败')
      setError(message)
      onNotice?.(message)
      throw commandError
    } finally {
      setBusy(false)
    }
  }, [onNotice, run, sessionId, updateRun])

  useEffect(() => listenForFitRunCodexSettled(() => {
    setSettlementRevision((current) => current + 1)
  }), [])

  const settlementRunId = run?.runId
  const settlementPhase = run?.phase
  const settlementHandoffId = run?.handoff?.handoffId
  const settlementTaskId = run?.handoff?.taskId
  useEffect(() => {
    if (!settlementRunId || !settlementHandoffId || !settlementTaskId) return undefined
    const settlement = readFitRunCodexSettlement(settlementTaskId)
    if (!settlement) return undefined
    if (settlementPhase !== 'CODEX_RUNNING') {
      if (settlementPhase !== 'PAUSED') clearFitRunCodexSettlement(settlementTaskId)
      return undefined
    }

    let disposed = false
    let retryTimer: number | undefined
    let attempts = 0
    const submit = async () => {
      setSettlementBusy(true)
      const payload: FitRunCommand = settlement.succeeded ? {
        type: 'CODEX_COMPLETED',
        commandId: fitRunSettlementCommandId(settlementRunId, settlementHandoffId, settlement),
        handoffId: settlementHandoffId,
        taskId: settlementTaskId,
        sourceRevisionAfter: '',
      } : {
        type: 'CODEX_FAILED',
        commandId: fitRunSettlementCommandId(settlementRunId, settlementHandoffId, settlement),
        handoffId: settlementHandoffId,
        error: 'Codex 任务未成功完成',
      }
      try {
        const response = await sendFitRunCommand(sessionId!, settlementRunId, payload)
        if (disposed) return
        clearFitRunCodexSettlement(settlementTaskId)
        setError('')
        updateRun(response.run)
      } catch (settlementError) {
        if (disposed) return
        attempts += 1
        const message = messageOf(settlementError, 'Codex 结果同步失败，正在自动重试')
        setError(message)
        if (attempts === 1) onNotice?.(message)
        retryTimer = window.setTimeout(() => { void submit() }, retryDelay(attempts))
      } finally {
        if (!disposed) setSettlementBusy(false)
      }
    }
    if (sessionId) void submit()
    return () => {
      disposed = true
      setSettlementBusy(false)
      if (retryTimer !== undefined) window.clearTimeout(retryTimer)
    }
  }, [
    onNotice,
    sessionId,
    settlementHandoffId,
    settlementPhase,
    settlementRevision,
    settlementRunId,
    settlementTaskId,
    updateRun,
  ])

  return {
    run,
    busy: busy || settlementBusy,
    error,
    canStart: Boolean(sessionId && input),
    start,
    refresh,
    command,
    clear: () => updateRun(null),
  }
}

function retryDelay(attempt: number) {
  return Math.min(1_000 * (2 ** Math.min(attempt - 1, 5)), 30_000)
}

function messageOf(error: unknown, fallback: string) {
  return error instanceof Error && error.message ? error.message : fallback
}
