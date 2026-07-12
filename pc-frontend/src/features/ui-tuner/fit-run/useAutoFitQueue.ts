import { useCallback, useEffect, useRef, useState } from 'react'
import type { DesignDiffRegion } from '../comparison/autoPairApi'
import type { PixelRect } from '../comparison/types'
import { acceptFitRunBatch } from './fitRunApi'
import {
  clearFitRunCodexSettlement,
  listenForFitRunCodexSettled,
  requestCodexForFitRun,
} from './fitRunEvents'
import type { useFitRun } from './useFitRun'
import type { CreateFitRunInput, FitBatchAcceptResult } from './types'
import { getFitLearningSummary, type FitLearningSummary } from './fitLearningApi'

export type AutoFitQueuePhase =
  | 'IDLE' | 'ACTIVATING' | 'RUNNING' | 'READY_TO_COMMIT'
  | 'COMMITTING' | 'CODEX_RUNNING' | 'COMPLETED' | 'FAILED'

interface UseAutoFitQueueOptions {
  sessionId?: string
  fitRun: ReturnType<typeof useFitRun>
  fitInput?: CreateFitRunInput
  activateRegion: (region: DesignDiffRegion) => boolean
  onNotice?: (message: string) => void
}

export function useAutoFitQueue({
  sessionId,
  fitRun,
  fitInput,
  activateRegion,
  onNotice,
}: UseAutoFitQueueOptions) {
  const [regions, setRegions] = useState<DesignDiffRegion[]>([])
  const [currentIndex, setCurrentIndex] = useState(-1)
  const [phase, setPhase] = useState<AutoFitQueuePhase>('IDLE')
  const [error, setError] = useState('')
  const [stagedRunIds, setStagedRunIds] = useState<string[]>([])
  const [batchResult, setBatchResult] = useState<FitBatchAcceptResult | null>(null)
  const [codexTaskId, setCodexTaskId] = useState('')
  const [sourceRevision, setSourceRevision] = useState('')
  const [learningSummary, setLearningSummary] = useState<FitLearningSummary | null>(null)
  const actionRef = useRef(false)
  const stagedRef = useRef<string[]>([])
  const current = currentIndex >= 0 ? regions[currentIndex] : undefined

  const refreshLearning = useCallback(async () => {
    if (!sessionId) {
      setLearningSummary(null)
      return null
    }
    try {
      const summary = await getFitLearningSummary(sessionId)
      setLearningSummary(summary)
      return summary
    } catch {
      return null
    }
  }, [sessionId])

  useEffect(() => { void refreshLearning() }, [refreshLearning])

  const start = useCallback((source: DesignDiffRegion[]) => {
    const runnable = source.filter((region) => region.recommendedRuntimeNodeId && region.candidates.length > 0)
    if (runnable.length === 0) throw new Error('没有找到可自动拟合的真实 Android 节点')
    stagedRef.current = []
    setStagedRunIds([])
    setBatchResult(null)
    setCodexTaskId('')
    setSourceRevision('')
    setRegions(runnable)
    setCurrentIndex(0)
    setError('')
    fitRun.clear()
    if (!activateRegion(runnable[0])) throw new Error('无法激活第一个拟合节点')
    setPhase('ACTIVATING')
    onNotice?.(`全页面拟合已启动，共 ${runnable.length} 个节点；完成后只构建一次`)
  }, [activateRegion, fitRun, onNotice])

  const reset = useCallback(() => {
    stagedRef.current = []
    setRegions([])
    setStagedRunIds([])
    setBatchResult(null)
    setCodexTaskId('')
    setSourceRevision('')
    setCurrentIndex(-1)
    setPhase('IDLE')
    setError('')
    actionRef.current = false
  }, [])

  const executeBatch = useCallback(async (codexCompleted = false) => {
    if (!sessionId || stagedRef.current.length === 0 || !sourceRevision) {
      throw new Error('批量拟合缺少 Live Session、源码版本或候选任务')
    }
    actionRef.current = true
    setPhase('COMMITTING')
    setError('')
    try {
      const result = await acceptFitRunBatch(
        sessionId,
        stagedRef.current,
        sourceRevision,
        codexCompleted,
      )
      setBatchResult(result)
      if (result.status === 'CODEX_REQUIRED') {
        if (!result.codexArtifactPath) throw new Error('批量 Codex 接力缺少 Artifact')
        setPhase('CODEX_RUNNING')
        const launch = await requestCodexForFitRun({
          runId: `batch:${stagedRef.current[0]}`,
          handoffId: `batch:${Date.now()}`,
          handoffPath: result.codexArtifactPath,
          reason: `一次性处理 ${stagedRef.current.length} 个节点的非确定性源码绑定`,
        })
        setCodexTaskId(launch.taskId)
        onNotice?.('已把最小批量 Artifact 交给 Codex；完成后会自动统一构建验收')
        return result
      }
      if (result.status !== 'BUILD_VERIFIED') {
        throw new Error(result.build?.message || '批量源码结果未通过双门禁')
      }
      setPhase('COMPLETED')
      void refreshLearning()
      onNotice?.(`全页面拟合完成：${stagedRef.current.length} 个节点只构建一次并通过双门禁`)
      return result
    } catch (cause) {
      setError(messageOf(cause, '批量写回和源码验收失败'))
      setPhase('FAILED')
      throw cause
    } finally {
      actionRef.current = false
    }
  }, [onNotice, refreshLearning, sessionId, sourceRevision])

  useEffect(() => listenForFitRunCodexSettled((settlement) => {
    if (!codexTaskId || settlement.taskId !== codexTaskId) return
    clearFitRunCodexSettlement(settlement.taskId)
    if (!settlement.succeeded) {
      setError('批量 Codex 源码任务未成功完成')
      setPhase('FAILED')
      return
    }
    void executeBatch(true)
  }), [codexTaskId, executeBatch])

  useEffect(() => {
    if (!current || actionRef.current) return
    if (phase === 'ACTIVATING') {
      if (!fitInput || !sameRect(fitInput.pair.targetRect, current.targetRect) || !fitRun.canStart) return
      actionRef.current = true
      void fitRun.start().then(() => {
        setPhase('RUNNING')
      }).catch((cause) => {
        setError(messageOf(cause, '自动拟合启动失败'))
        setPhase('FAILED')
      }).finally(() => { actionRef.current = false })
      return
    }
    if (phase !== 'RUNNING' || !fitRun.run) return
    if (fitRun.run.phase === 'CANDIDATE_READY') {
      if (stagedRef.current.includes(fitRun.run.runId)) return
      stagedRef.current = [...stagedRef.current, fitRun.run.runId]
      setStagedRunIds(stagedRef.current)
      setSourceRevision(fitRun.run.sourceRevision ?? '')
      const nextIndex = currentIndex + 1
      if (nextIndex >= regions.length) {
        setPhase('READY_TO_COMMIT')
        onNotice?.(`已得到 ${regions.length} 个 Live 最佳结果，等待统一写回和一次构建`)
        return
      }
      const next = regions[nextIndex]
      fitRun.clear()
      setCurrentIndex(nextIndex)
      if (!activateRegion(next)) {
        setError('下一个 Runtime 节点已失效，请刷新真机节点后重试')
        setPhase('FAILED')
        return
      }
      setPhase('ACTIVATING')
      return
    }
    if (['PLATEAU', 'FAILED', 'CANCELLED'].includes(fitRun.run.phase)) {
      setError(`节点 ${currentIndex + 1} 未完成：${fitRun.run.stopReason ?? fitRun.run.phase}`)
      setPhase('FAILED')
    }
  }, [activateRegion, current, currentIndex, fitInput, fitRun, onNotice, phase, regions])

  return {
    phase,
    error,
    regions,
    currentIndex,
    current,
    stagedRunIds,
    batchResult,
    learningSummary,
    start,
    reset,
    commit: () => executeBatch(false),
    active: ['ACTIVATING', 'RUNNING', 'COMMITTING', 'CODEX_RUNNING'].includes(phase),
  }
}

function sameRect(left: PixelRect, right: PixelRect) {
  return left.left === right.left && left.top === right.top
    && left.right === right.right && left.bottom === right.bottom
}

function messageOf(cause: unknown, fallback: string) {
  return cause instanceof Error && cause.message ? cause.message : fallback
}
