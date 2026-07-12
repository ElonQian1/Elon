import { useCallback, useEffect, useRef, useState } from 'react'
import type { DesignDiffRegion } from '../comparison/autoPairApi'
import type { PixelRect } from '../comparison/types'
import type { useFitRun } from './useFitRun'
import type { CreateFitRunInput } from './types'

export type AutoFitQueuePhase = 'IDLE' | 'ACTIVATING' | 'RUNNING' | 'COMPLETED' | 'FAILED'

interface UseAutoFitQueueOptions {
  fitRun: ReturnType<typeof useFitRun>
  fitInput?: CreateFitRunInput
  activateRegion: (region: DesignDiffRegion) => boolean
  onNotice?: (message: string) => void
}

export function useAutoFitQueue({
  fitRun,
  fitInput,
  activateRegion,
  onNotice,
}: UseAutoFitQueueOptions) {
  const [regions, setRegions] = useState<DesignDiffRegion[]>([])
  const [currentIndex, setCurrentIndex] = useState(-1)
  const [phase, setPhase] = useState<AutoFitQueuePhase>('IDLE')
  const [error, setError] = useState('')
  const actionRef = useRef(false)
  const current = currentIndex >= 0 ? regions[currentIndex] : undefined

  const start = useCallback((source: DesignDiffRegion[]) => {
    const runnable = source.filter((region) => region.recommendedRuntimeNodeId && region.candidates.length > 0)
    if (runnable.length === 0) throw new Error('没有找到可自动拟合的真实 Android 节点')
    setRegions(runnable)
    setCurrentIndex(0)
    setError('')
    fitRun.clear()
    if (!activateRegion(runnable[0])) throw new Error('无法激活第一个拟合节点')
    setPhase('ACTIVATING')
    onNotice?.(`全页面拟合已启动，共 ${runnable.length} 个节点`)
  }, [activateRegion, fitRun, onNotice])

  const reset = useCallback(() => {
    setRegions([])
    setCurrentIndex(-1)
    setPhase('IDLE')
    setError('')
    actionRef.current = false
  }, [])

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
      actionRef.current = true
      void fitRun.command({ type: 'ACCEPT_BEST' }).catch((cause) => {
        setError(messageOf(cause, '自动写回和源码验收失败'))
        setPhase('FAILED')
      }).finally(() => { actionRef.current = false })
      return
    }
    if (fitRun.run.phase === 'ACCEPTED') {
      const nextIndex = currentIndex + 1
      if (nextIndex >= regions.length) {
        setPhase('COMPLETED')
        onNotice?.(`全页面拟合完成，${regions.length} 个节点已通过目标和源码门禁`)
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
  }, [
    activateRegion,
    current,
    currentIndex,
    fitInput,
    fitRun,
    onNotice,
    phase,
    regions,
  ])

  return {
    phase,
    error,
    regions,
    currentIndex,
    current,
    start,
    reset,
    active: ['ACTIVATING', 'RUNNING'].includes(phase),
  }
}

function sameRect(left: PixelRect, right: PixelRect) {
  return left.left === right.left && left.top === right.top
    && left.right === right.right && left.bottom === right.bottom
}

function messageOf(cause: unknown, fallback: string) {
  return cause instanceof Error && cause.message ? cause.message : fallback
}
