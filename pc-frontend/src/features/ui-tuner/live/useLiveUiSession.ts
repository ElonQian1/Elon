import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { UiTunerDocument, UiTunerElement } from '../types'
import {
  applyLiveUiPatch,
  buildAndVerifyLiveUi,
  getLiveUiFrame,
  getLiveUiSession,
  getLiveUiTree,
  liveUiHistoryAction,
  openLiveUiPreview,
  startLiveUiSession,
  stopLiveUiSession,
  type LivePatchOperation,
  type LiveBuildVerifyResult,
  type LivePreviewRequest,
  type LiveMcpDescriptor,
  type LiveUiNode,
  type LiveUiFrame,
  type LiveUiScope,
  type LiveUiSession,
} from './liveUiApi'
import {
  bindLiveUiIr,
  runLiveVisualSolver,
  uploadLiveTargetDesign,
  type LiveTargetDesign,
  type LiveUiIrDocument,
  type PixelRect,
  type VisualSolverResult,
} from './liveUiIrApi'
import {
  commitLiveSource,
  getLiveSourceCommitPlan,
  type LiveSourceCommitPlan,
  type LiveSourceCommitResult,
} from './liveUiCommitApi'
import { matchLiveNode, mergeEffectiveValues, messageOf } from './liveUiSessionHelpers'

export type LiveUiConnectionState = 'idle' | 'connecting' | 'connected' | 'attach_only' | 'error'

interface UseLiveUiSessionOptions {
  deviceId?: string
  packageName?: string
  projectRoot?: string
  document: UiTunerDocument
  selected: UiTunerElement | null
  onNotice: (message: string) => void
}

export function useLiveUiSession({
  deviceId,
  packageName,
  projectRoot,
  document,
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
  const [mcp, setMcp] = useState<LiveMcpDescriptor | null>(null)
  const [uiIr, setUiIr] = useState<LiveUiIrDocument | null>(null)
  const [targetDesign, setTargetDesign] = useState<LiveTargetDesign | null>(null)
  const [solverResult, setSolverResult] = useState<VisualSolverResult | null>(null)
  const [liveFrame, setLiveFrame] = useState<LiveUiFrame | null>(null)
  const [buildVerifyResult, setBuildVerifyResult] = useState<LiveBuildVerifyResult | null>(null)
  const [restartRevision, setRestartRevision] = useState(0)
  const sessionRef = useRef<LiveUiSession | null>(null)
  const generationRef = useRef(0)
  const targetDesignRef = useRef<LiveTargetDesign | null>(null)
  const targetSignatureRef = useRef('')

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
      setMcp(null)
      setUiIr(null)
      setTargetDesign(null)
      setSolverResult(null)
      setLiveFrame(null)
      setBuildVerifyResult(null)
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
      setMcp(null)
      setUiIr(null)
      setTargetDesign(null)
      setSolverResult(null)
      setLiveFrame(null)
      setBuildVerifyResult(null)
      targetDesignRef.current = null
      targetSignatureRef.current = ''
      setState('connecting')
      setError('')
      try {
        const started = await startLiveUiSession({
          deviceId: cleanDevice,
          packageName: cleanPackage,
          projectRoot,
        })
        const created = started.session
        if (disposed || generation !== generationRef.current) {
          await stopLiveUiSession(created.id).catch(() => undefined)
          return
        }
        sessionRef.current = created
        setSession(created)
        setMcp(started.mcp)
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

  useEffect(() => {
    if (state !== 'connected' || !session?.id) {
      setLiveFrame(null)
      return
    }
    let disposed = false
    let timer: number | undefined
    const pollFrame = async () => {
      if (disposed) return
      try {
        const frame = await getLiveUiFrame(session.id)
        if (!disposed) setLiveFrame(frame)
      } catch {
        // 树与 Patch 连接仍然可用时，单帧截图失败不应降级整个 Live 会话。
      }
      if (!disposed) timer = window.setTimeout(() => { void pollFrame() }, 900)
    }
    void pollFrame()
    return () => {
      disposed = true
      if (timer !== undefined) window.clearTimeout(timer)
    }
  }, [session?.id, state])

  const selectedNode = useMemo(
    () => matchLiveNode(selected, nodes),
    [nodes, selected],
  )

  const syncContext = useCallback(async () => {
    const current = sessionRef.current
    if (!current?.connected || !document.runtimeSnapshot) return null
    let target = targetDesignRef.current
    const image = document.canvas.targetDesign
    const signature = image
      ? [
          image.name,
          image.dataUrl.length,
          image.dataUrl.slice(-64),
          image.width,
          image.height,
          image.figmaUrl ?? '',
        ].join(':')
      : ''
    if (image && signature !== targetSignatureRef.current) {
      target = await uploadLiveTargetDesign(current.id, image)
      targetDesignRef.current = target
      targetSignatureRef.current = signature
      setTargetDesign(target)
    } else if (!image) {
      target = null
      targetDesignRef.current = null
      targetSignatureRef.current = ''
      setTargetDesign(null)
    }
    const next = await bindLiveUiIr({
      sessionId: current.id,
      document,
      selected,
      selectedRuntimeNodeId: selectedNode?.runtimeNodeId,
      targetDesign: target ?? undefined,
    })
    setUiIr(next)
    return next
  }, [document, selected, selectedNode?.runtimeNodeId])

  useEffect(() => {
    if (state !== 'connected' || !document.runtimeSnapshot) return
    const timer = window.setTimeout(() => {
      void syncContext().catch((syncError) => {
        setError(messageOf(syncError, '无法同步 UI IR'))
      })
    }, 360)
    return () => window.clearTimeout(timer)
  }, [
    document.canvas.targetDesign,
    document.runtimeSnapshot,
    selected?.id,
    selectedNode?.runtimeNodeId,
    state,
    syncContext,
  ])

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

  const solve = useCallback(async (
    targetRect: PixelRect,
    properties?: string[],
  ) => {
    const current = sessionRef.current
    const target = matchLiveNode(selected, nodes)
    if (!current?.connected || !target) throw new Error('当前元素尚未绑定 Live Node')
    if (!document.canvas.targetDesign) throw new Error('请先导入目标设计图')
    setBusy(true)
    setSolverResult(null)
    try {
      await syncContext()
      const result = await runLiveVisualSolver({
        sessionId: current.id,
        runtimeNodeId: target.runtimeNodeId,
        targetRect,
        properties,
      })
      setSolverResult(result)
      await refresh(current.id)
      onNotice(result.status === 'APPLIED'
        ? '本地视觉求解已应用：损失改善 ' + result.improvementPercent.toFixed(2) + '%，源码尚未写入'
        : '本地视觉求解未找到更优参数，真机保持不变')
      return result
    } catch (solveError) {
      const message = messageOf(solveError, '本地视觉求解失败')
      setError(message)
      onNotice(message)
      throw solveError
    } finally {
      setBusy(false)
    }
  }, [document.canvas.targetDesign, nodes, onNotice, refresh, selected, syncContext])

  const openPreview = useCallback(async (request: LivePreviewRequest) => {
    const current = sessionRef.current
    if (!current?.connected) throw new Error('Live Runtime 尚未连接')
    setBusy(true)
    try {
      await openLiveUiPreview(current.id, request)
      onNotice(`Preview 已切换：${request.screenId} · ${request.scenario} · ${request.theme}`)
      window.setTimeout(() => { void refresh(current.id).catch(() => undefined) }, 350)
    } catch (previewError) {
      const message = messageOf(previewError, '无法打开 Preview Host')
      setError(message)
      onNotice(message)
      throw previewError
    } finally {
      setBusy(false)
    }
  }, [onNotice, refresh])

  const buildVerify = useCallback(async (preview?: LivePreviewRequest) => {
    const current = sessionRef.current
    if (!current) throw new Error('Live UI 会话不存在')
    setBusy(true)
    setBuildVerifyResult(null)
    try {
      const result = await buildAndVerifyLiveUi(current.id, preview)
      setBuildVerifyResult(result)
      await refresh(current.id)
      onNotice(`BUILD VERIFIED：${result.runtimeBuildId ?? '新 Debug APK'} 已安装，临时 Patch 已清空`)
      return result
    } catch (verifyError) {
      const message = messageOf(verifyError, '构建安装验收失败')
      setError(message)
      onNotice(message)
      throw verifyError
    } finally {
      setBusy(false)
    }
  }, [onNotice, refresh])

  return {
    session,
    nodes,
    selectedNode,
    state,
    error,
    busy,
    commitPlan,
    commitResult,
    mcp,
    uiIr,
    targetDesign,
    solverResult,
    liveFrame,
    buildVerifyResult,
    apply,
    undo: () => historyAction('undo'),
    redo: () => historyAction('redo'),
    reconnect: () => setRestartRevision((value) => value + 1),
    refresh,
    previewCommit,
    commit,
    syncContext,
    solve,
    openPreview,
    buildVerify,
  }
}
