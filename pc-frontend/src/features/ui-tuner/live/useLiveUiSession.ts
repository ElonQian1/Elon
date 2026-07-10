import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { UiTunerElement } from '../types'
import {
  applyLiveUiPatch,
  getLiveUiSession,
  getLiveUiTree,
  liveUiHistoryAction,
  startLiveUiSession,
  stopLiveUiSession,
  type LivePatchOperation,
  type LiveUiNode,
  type LiveUiScope,
  type LiveUiSession,
} from './liveUiApi'
import {
  commitLiveSource,
  getLiveSourceCommitPlan,
  type LiveSourceCommitPlan,
  type LiveSourceCommitResult,
} from './liveUiCommitApi'

export type LiveUiConnectionState = 'idle' | 'connecting' | 'connected' | 'attach_only' | 'error'

interface UseLiveUiSessionOptions {
  deviceId?: string
  packageName?: string
  projectRoot?: string
  selected: UiTunerElement | null
  onNotice: (message: string) => void
}

export function useLiveUiSession({
  deviceId,
  packageName,
  projectRoot,
  selected,
  onNotice,
}: UseLiveUiSessionOptions) {
  const [session, setSession] = useState<LiveUiSession | null>(null)
  const [nodes, setNodes] = useState<LiveUiNode[]>([])
  const [state, setState] = useState<LiveUiConnectionState>('idle')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [commitPlan, setCommitPlan] = useState<LiveSourceCommitPlan | null>(null)
  const [commitResult, setCommitResult] = useState<LiveSourceCommitResult | null>(null)
  const [restartRevision, setRestartRevision] = useState(0)
  const sessionRef = useRef<LiveUiSession | null>(null)
  const generationRef = useRef(0)

  const refresh = useCallback(async (sessionId?: string) => {
    const id = sessionId ?? sessionRef.current?.id
    if (!id) return
    const [nextSession, tree] = await Promise.all([
      getLiveUiSession(id),
      getLiveUiTree(id),
    ])
    sessionRef.current = nextSession
    setSession(nextSession)
    setNodes(tree.nodes)
    if (nextSession.connected) {
      setState('connected')
      setError('')
    }
  }, [])

  useEffect(() => {
    const cleanDevice = deviceId?.trim()
    const cleanPackage = packageName?.trim()
    const generation = ++generationRef.current
    let timer: number | undefined
    let disposed = false

    const stopCurrent = async () => {
      const current = sessionRef.current
      sessionRef.current = null
      if (current) await stopLiveUiSession(current.id).catch(() => undefined)
    }

    if (!cleanDevice || !cleanPackage) {
      void stopCurrent()
      setSession(null)
      setNodes([])
      setCommitPlan(null)
      setCommitResult(null)
      setState('idle')
      setError('')
      return () => undefined
    }

    void (async () => {
      await stopCurrent()
      if (disposed || generation !== generationRef.current) return
      setSession(null)
      setNodes([])
      setCommitPlan(null)
      setCommitResult(null)
      setState('connecting')
      setError('')
      try {
        const created = await startLiveUiSession({
          deviceId: cleanDevice,
          packageName: cleanPackage,
          projectRoot,
        })
        if (disposed || generation !== generationRef.current) {
          await stopLiveUiSession(created.id).catch(() => undefined)
          return
        }
        sessionRef.current = created
        setSession(created)
        const startedAt = Date.now()
        const poll = async () => {
          if (disposed || generation !== generationRef.current) return
          try {
            await refresh(created.id)
            if (sessionRef.current?.connected) return
            if (Date.now() - startedAt > 8_000) {
              setState('attach_only')
              setError('当前 APK 未连接 Debug Runtime，已保留原有截图/XML 调试模式。')
              return
            }
          } catch (pollError) {
            if (Date.now() - startedAt > 8_000) {
              setState('attach_only')
              setError(messageOf(pollError, '当前 APK 不支持 Live Runtime'))
              return
            }
          }
          timer = window.setTimeout(() => { void poll() }, 650)
        }
        await poll()
      } catch (startError) {
        if (disposed || generation !== generationRef.current) return
        const message = messageOf(startError, '无法启动 Live Runtime')
        setState('attach_only')
        setError(`${message}；已保留截图/XML 调试模式。`)
      }
    })()

    return () => {
      disposed = true
      if (timer !== undefined) window.clearTimeout(timer)
      const current = sessionRef.current
      sessionRef.current = null
      if (current) void stopLiveUiSession(current.id).catch(() => undefined)
    }
  }, [deviceId, packageName, projectRoot, refresh, restartRevision])

  const selectedNode = useMemo(
    () => matchLiveNode(selected, nodes),
    [nodes, selected],
  )

  const apply = useCallback(async (
    operation: LivePatchOperation,
    scope: LiveUiScope,
  ) => {
    const currentSession = sessionRef.current
    const target = matchLiveNode(selected, nodes)
    if (!currentSession?.connected || !target) {
      throw new Error('当前选中元素尚未绑定 Live Node')
    }
    setBusy(true)
    try {
      const ack = await applyLiveUiPatch({
        sessionId: currentSession.id,
        target,
        scope,
        operation,
      })
      if (ack.status !== 'APPLIED') throw new Error(ack.error || '真机拒绝了修改')
      setNodes((current) => current.map((node) => (
        node.runtimeNodeId === target.runtimeNodeId
          ? mergeEffectiveValues(node, ack.effectiveValues ?? {})
          : node
      )))
      setCommitPlan(null)
      setCommitResult(null)
      onNotice(`LIVE PREVIEW：${operation.property} 已在真机生效，源码尚未写入`)
      window.setTimeout(() => { void refresh().catch(() => undefined) }, 180)
      return ack
    } catch (applyError) {
      const message = messageOf(applyError, '真机实时修改失败')
      setError(message)
      onNotice(message)
      throw applyError
    } finally {
      setBusy(false)
    }
  }, [nodes, onNotice, refresh, selected])

  const historyAction = useCallback(async (action: 'undo' | 'redo') => {
    const current = sessionRef.current
    if (!current?.connected) return
    setBusy(true)
    try {
      await liveUiHistoryAction(current.id, action)
      await new Promise((resolve) => window.setTimeout(resolve, 120))
      await refresh(current.id)
      setCommitPlan(null)
      setCommitResult(null)
      onNotice(action === 'undo' ? '已撤销一条真机实时修改' : '已重做一条真机实时修改')
    } finally {
      setBusy(false)
    }
  }, [onNotice, refresh])

  const previewCommit = useCallback(async () => {
    const current = sessionRef.current
    if (!current?.connected) throw new Error('Live Runtime 尚未连接')
    setBusy(true)
    try {
      const plan = await getLiveSourceCommitPlan(current.id)
      setCommitPlan(plan)
      return plan
    } catch (planError) {
      const message = messageOf(planError, '无法生成源码写回计划')
      setError(message)
      onNotice(message)
      throw planError
    } finally {
      setBusy(false)
    }
  }, [onNotice])

  const commit = useCallback(async (plan: LiveSourceCommitPlan) => {
    const current = sessionRef.current
    if (!current?.connected) throw new Error('Live Runtime 尚未连接')
    setBusy(true)
    try {
      const result = await commitLiveSource(current.id, plan.sourceRevision)
      setCommitResult(result)
      setCommitPlan(null)
      onNotice(`SOURCE SAVED：已写入 ${result.changedFiles.length} 个源码文件，等待构建验证`)
      return result
    } catch (commitError) {
      const message = messageOf(commitError, '源码写回失败')
      setError(message)
      onNotice(message)
      throw commitError
    } finally {
      setBusy(false)
    }
  }, [onNotice])

  return {
    session,
    nodes,
    selectedNode,
    state,
    error,
    busy,
    commitPlan,
    commitResult,
    apply,
    undo: () => historyAction('undo'),
    redo: () => historyAction('redo'),
    reconnect: () => setRestartRevision((value) => value + 1),
    refresh,
    previewCommit,
    commit,
  }
}

function matchLiveNode(selected: UiTunerElement | null, nodes: LiveUiNode[]): LiveUiNode | null {
  if (!selected?.runtime) return null
  const resourceId = comparableResourceId(selected.runtime.resourceId)
  if (resourceId) {
    const exact = nodes.find((node) => comparableResourceId(node.resourceId) === resourceId)
    if (exact) return exact
  }
  const original = selected.runtime.originalBounds
  let best: { node: LiveUiNode; score: number } | null = null
  for (const node of nodes) {
    if (!node.geometry.visible) continue
    const score = overlapScore(original, node.geometry.boundsInDisplayPx)
    if (score > (best?.score ?? 0)) best = { node, score }
  }
  return best && best.score >= 0.45 ? best.node : null
}

function comparableResourceId(value?: string) {
  return value?.trim().replace(/^.*:id\//, '').replace(/^.*\/id\//, '') || ''
}

function overlapScore(
  left: NonNullable<UiTunerElement['runtime']>['originalBounds'],
  right: LiveUiNode['geometry']['boundsInDisplayPx'],
) {
  const intersectionWidth = Math.max(0, Math.min(left.right, right.right) - Math.max(left.left, right.left))
  const intersectionHeight = Math.max(0, Math.min(left.bottom, right.bottom) - Math.max(left.top, right.top))
  const intersection = intersectionWidth * intersectionHeight
  const leftArea = Math.max(1, left.width * left.height)
  const rightArea = Math.max(1, right.width * right.height)
  return intersection / Math.max(leftArea, rightArea)
}

function mergeEffectiveValues(
  node: LiveUiNode,
  values: Record<string, LiveUiNode['properties'][string]['effective']>,
): LiveUiNode {
  const properties = { ...node.properties }
  for (const [name, value] of Object.entries(values)) {
    if (!value || !properties[name]) continue
    properties[name] = { ...properties[name], effective: value }
  }
  return { ...node, properties }
}

function messageOf(error: unknown, fallback: string) {
  return error instanceof Error && error.message.trim() ? error.message : fallback
}
