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
  reconnectLiveUiSession,
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
  uploadLiveTargetDesign,
  type LiveTargetDesign,
  type LiveUiIrDocument,
} from './liveUiIrApi'
import {
  commitLiveSource,
  getLiveSourceCommitPlan,
  type LiveSourceCommitPlan,
  type LiveSourceCommitResult,
} from './liveUiCommitApi'
import { matchLiveNode, mergeEffectiveValues, messageOf } from './liveUiSessionHelpers'
import { buildVerificationNotice } from './verification/notice'

export type LiveUiConnectionState = 'idle' | 'connecting' | 'connected' | 'attach_only' | 'error'

interface UseLiveUiSessionOptions {
  deviceId?: string
  packageName?: string
  projectRoot?: string
  debugApplicationIdSuffix?: string
  document: UiTunerDocument
  selected: UiTunerElement | null
  onNotice: (message: string) => void
}

export function useLiveUiSession({
  deviceId,
  packageName,
  projectRoot,
  debugApplicationIdSuffix,
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
  const [liveFrame, setLiveFrame] = useState<LiveUiFrame | null>(null)
  const [buildVerifyResult, setBuildVerifyResult] = useState<LiveBuildVerifyResult | null>(null)
  const [gestureActive, setGestureActiveState] = useState(false)
  const [previewRequest, setPreviewRequest] = useState<LivePreviewRequest | null>(null)
  const sessionRef = useRef<LiveUiSession | null>(null)
  const treeRevisionRef = useRef<number | null>(null)
  const stopPromiseRef = useRef<Promise<void>>(Promise.resolve())
  const startPromiseRef = useRef<Promise<void>>(Promise.resolve())
  const generationRef = useRef(0)
  const targetDesignRef = useRef<LiveTargetDesign | null>(null)
  const targetSignatureRef = useRef('')
  const gestureActiveRef = useRef(false)
  const reconnectPromiseRef = useRef<Promise<void> | null>(null)
  const reconnectAttemptRef = useRef(0)
  const lastPreviewRef = useRef<LivePreviewRequest | null>(null)
  const gestureFrameBusyRef = useRef(false)
  const gestureFramePendingRef = useRef<string | null>(null)

  useEffect(() => {
    // A Preview belongs to the selected APK/device, not to a particular
    // source-root-backed Live session. Preserve it when the user corrects or
    // switches the source directory so Build Verify can reopen the same scene.
    lastPreviewRef.current = null
  }, [deviceId, packageName])

  const refresh = useCallback(async (sessionId?: string) => {
    const id = sessionId ?? sessionRef.current?.id
    if (!id) return
    const [nextSession, tree] = await Promise.all([
      getLiveUiSession(id),
      getLiveUiTree(id),
    ])
    const treeChanged = treeRevisionRef.current !== nextSession.treeRevision
    sessionRef.current = nextSession
    treeRevisionRef.current = nextSession.treeRevision
    setSession(nextSession)
    if (treeChanged) setNodes(tree.nodes)
    if (nextSession.connected) {
      setState('connected')
      setError('')
    } else {
      // A Debug APK reinstall or process restart tears down the runtime socket.
      // Do not keep presenting stale nodes as a usable LIVE connection: expose
      // the existing reconnect action so the user can bootstrap the installed
      // package again without rebuilding it.
      setState('attach_only')
      setError('Debug Runtime 已断开，请重新连接后继续实时修改。')
    }
  }, [])

  useEffect(() => {
    const cleanDevice = deviceId?.trim()
    const cleanPackage = packageName?.trim()
    const generation = ++generationRef.current
    let timer: number | undefined
    let disposed = false

    const queueStop = (current: LiveUiSession) => {
      const next = stopPromiseRef.current
        .catch(() => undefined)
        .then(() => stopLiveUiSession(current.id).catch(() => undefined))
      stopPromiseRef.current = next
      return next
    }
    const stopCurrent = async () => {
      const current = sessionRef.current
      sessionRef.current = null
      if (current) await queueStop(current)
    }

    if (!cleanDevice || !cleanPackage) {
      void stopCurrent()
      setSession(null)
      treeRevisionRef.current = null
      setNodes([])
      setCommitPlan(null)
      setCommitResult(null)
      setMcp(null)
      setUiIr(null)
      setTargetDesign(null)
      setLiveFrame(null)
      setBuildVerifyResult(null)
      setPreviewRequest(null)
      setState('idle')
      setError('')
      return () => undefined
    }

    const startTransaction = startPromiseRef.current
      .catch(() => undefined)
      .then(async () => {
      // React effect cleanup cannot be awaited. Serialize STOP before the
      // next START so a stale session cannot remove the new adb reverse rule
      // or stop the newly connected Android Runtime.
      await stopPromiseRef.current.catch(() => undefined)
      await stopCurrent()
      if (disposed || generation !== generationRef.current) return
      setSession(null)
      treeRevisionRef.current = null
      setNodes([])
      setCommitPlan(null)
      setCommitResult(null)
      setMcp(null)
      setUiIr(null)
      setTargetDesign(null)
      setLiveFrame(null)
      setBuildVerifyResult(null)
      setPreviewRequest(null)
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
          await queueStop(created)
          return
        }
        sessionRef.current = created
        setSession(created)
        setMcp(started.mcp)
        return created
      } catch (startError) {
        if (disposed || generation !== generationRef.current) return null
        const message = messageOf(startError, '无法启动 Live Runtime')
        setState('attach_only')
        setError(`${message}；已保留截图/XML调试模式。`)
        return null
      }
    })
    startPromiseRef.current = startTransaction.then(() => undefined, () => undefined)

    void (async () => {
      const created = await startTransaction
      if (!created || disposed || generation !== generationRef.current) return
      try {
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
      if (current) void queueStop(current)
    }
  }, [deviceId, packageName, projectRoot, refresh])

  const refreshFrame = useCallback(async (sessionId?: string) => {
    const id = sessionId ?? sessionRef.current?.id
    if (!id) return
    const frame = await getLiveUiFrame(id)
    setLiveFrame(frame)
  }, [])

  const requestGestureFrame = useCallback((sessionId: string) => {
    gestureFramePendingRef.current = sessionId
    if (gestureFrameBusyRef.current) return
    gestureFrameBusyRef.current = true
    void (async () => {
      try {
        while (gestureFramePendingRef.current) {
          const pendingSessionId = gestureFramePendingRef.current
          gestureFramePendingRef.current = null
          await refreshFrame(pendingSessionId).catch(() => undefined)
        }
      } finally {
        gestureFrameBusyRef.current = false
      }
    })()
  }, [refreshFrame])

  const reconnect = useCallback(async () => {
    const current = sessionRef.current
    if (!current) return
    if (reconnectPromiseRef.current) return reconnectPromiseRef.current
    const task = (async () => {
      setState('connecting')
      setError('真实画面已冻结，正在恢复 adb reverse 与 Android Runtime…')
      try {
        await reconnectLiveUiSession(current.id)
        for (let attempt = 0; attempt < 16; attempt += 1) {
          await new Promise((resolve) => window.setTimeout(resolve, 250))
          await refresh(current.id)
          if (sessionRef.current?.connected) {
            reconnectAttemptRef.current = 0
            await refreshFrame(current.id).catch(() => undefined)
            return
          }
        }
        throw new Error('Android Runtime 重连超时')
      } catch (reconnectError) {
        reconnectAttemptRef.current += 1
        setState('attach_only')
        setError(messageOf(reconnectError, 'Android Runtime 自动重连失败'))
      }
    })()
    reconnectPromiseRef.current = task
    try {
      await task
    } finally {
      reconnectPromiseRef.current = null
    }
  }, [refresh, refreshFrame])

  useEffect(() => {
    if (state !== 'attach_only' || !session?.id) return
    const delay = Math.min(8_000, 500 * (2 ** reconnectAttemptRef.current))
    const timer = window.setTimeout(() => { void reconnect() }, delay)
    return () => window.clearTimeout(timer)
  }, [reconnect, session?.id, state])

  useEffect(() => {
    if (state !== 'connected' || !session?.id) {
      return
    }
    let disposed = false
    let timer: number | undefined
    const pollRuntime = async () => {
      if (disposed) return
      try {
        await Promise.all([refresh(session.id), refreshFrame(session.id)])
      } catch {
        // A transient tree/frame read must not tear down the current live session.
      }
      if (!disposed) timer = window.setTimeout(
        () => { void pollRuntime() },
        gestureActiveRef.current ? 160 : 1_500,
      )
    }
    void pollRuntime()
    return () => {
      disposed = true
      if (timer !== undefined) window.clearTimeout(timer)
    }
  }, [refresh, refreshFrame, session?.id, state])

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

  const applyOperations = useCallback(async (
    operations: LivePatchOperation[],
    scope: LiveUiScope,
    gestureId?: string,
  ) => {
    const currentSession = sessionRef.current
    const target = matchLiveNode(selected, nodes)
    if (!currentSession?.connected || !target) {
      throw new Error('当前选中元素尚未绑定 Live Node')
    }
    if (!gestureId) setBusy(true)
    try {
      const ack = await applyLiveUiPatch({
        sessionId: currentSession.id,
        target,
        scope,
        operations,
        gestureId,
      })
      if (ack.status !== 'APPLIED') throw new Error(ack.error || '真机拒绝了修改')
      setNodes((current) => current.map((node) => (
        node.runtimeNodeId === target.runtimeNodeId
          ? mergeEffectiveValues(node, ack.effectiveValues ?? {})
          : node
      )))
      setCommitPlan(null)
      setCommitResult(null)
      if (gestureId) {
        window.setTimeout(() => requestGestureFrame(currentSession.id), 16)
      }
      if (!gestureId) {
        onNotice(`LIVE PREVIEW：${operations.map((item) => item.property).join('、')} 已在真机生效，源码尚未写入`)
        window.setTimeout(() => {
          void Promise.all([refresh(), refreshFrame(currentSession.id)]).catch(() => undefined)
        }, 180)
      }
      return ack
    } catch (applyError) {
      const message = messageOf(applyError, '真机实时修改失败')
      setError(message)
      onNotice(message)
      throw applyError
    } finally {
      if (!gestureId) setBusy(false)
    }
  }, [nodes, onNotice, refresh, refreshFrame, requestGestureFrame, selected])

  const apply = useCallback((operation: LivePatchOperation, scope: LiveUiScope) => (
    applyOperations([operation], scope)
  ), [applyOperations])

  const applyGesture = useCallback((operations: LivePatchOperation[], gestureId: string) => (
    applyOperations(operations, 'INSTANCE', gestureId)
  ), [applyOperations])

  const setGestureActive = useCallback((active: boolean) => {
    gestureActiveRef.current = active
    setGestureActiveState(active)
    if (!active) {
      const current = sessionRef.current
      if (current?.connected) {
        window.setTimeout(() => {
          void Promise.all([refresh(current.id), refreshFrame(current.id)]).catch(() => undefined)
        }, 80)
      }
    }
  }, [refresh, refreshFrame])

  const historyAction = useCallback(async (action: 'undo' | 'redo') => {
    const current = sessionRef.current
    if (!current?.connected) return
    const target = matchLiveNode(selected, nodes)
    setBusy(true)
    try {
      const ack = await liveUiHistoryAction(current.id, action)
      await new Promise((resolve) => window.setTimeout(resolve, 120))
      await Promise.all([refresh(current.id), refreshFrame(current.id)])
      if (target && ack.effectiveValues) {
        setNodes((currentNodes) => currentNodes.map((node) => (
          node.runtimeNodeId === target.runtimeNodeId
            ? mergeEffectiveValues(node, ack.effectiveValues ?? {})
            : node
        )))
      }
      setCommitPlan(null)
      setCommitResult(null)
      onNotice(action === 'undo' ? '已撤销一条真机实时修改' : '已重做一条真机实时修改')
    } finally {
      setBusy(false)
    }
  }, [nodes, onNotice, refresh, refreshFrame, selected])

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

  const openPreview = useCallback(async (request: LivePreviewRequest) => {
    const current = sessionRef.current
    if (!current?.connected) throw new Error('Live Runtime 尚未连接')
    setBusy(true)
    try {
      await openLiveUiPreview(current.id, request)
      lastPreviewRef.current = request
      setPreviewRequest(request)
      onNotice(`Preview 已切换：${request.screenId} · ${request.scenario} · ${request.theme}`)
      window.setTimeout(() => {
        void Promise.all([refresh(current.id), refreshFrame(current.id)]).catch(() => undefined)
      }, 350)
    } catch (previewError) {
      const message = messageOf(previewError, '无法打开 Preview Host')
      setError(message)
      onNotice(message)
      throw previewError
    } finally {
      setBusy(false)
    }
  }, [onNotice, refresh, refreshFrame])

  const buildVerify = useCallback(async (preview?: LivePreviewRequest) => {
    const current = sessionRef.current
    if (!current) throw new Error('Live UI 会话不存在')
    setBusy(true)
    setBuildVerifyResult(null)
    try {
      const result = await buildAndVerifyLiveUi(
        current.id,
        preview ?? lastPreviewRef.current ?? undefined,
        debugApplicationIdSuffix,
      )
      if (preview) {
        lastPreviewRef.current = preview
        setPreviewRequest(preview)
      }
      setBuildVerifyResult(result)
      await Promise.all([refresh(current.id), refreshFrame(current.id)])
      onNotice(buildVerificationNotice(result))
      return result
    } catch (verifyError) {
      const message = messageOf(verifyError, '构建安装验收失败')
      setError(message)
      onNotice(message)
      throw verifyError
    } finally {
      setBusy(false)
    }
  }, [debugApplicationIdSuffix, onNotice, refresh, refreshFrame])

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
    liveFrame,
    buildVerifyResult,
    gestureActive,
    previewRequest,
    apply,
    applyGesture,
    setGestureActive,
    undo: () => historyAction('undo'),
    redo: () => historyAction('redo'),
    reconnect: () => {
      reconnectAttemptRef.current = 0
      void reconnect()
    },
    refresh,
    previewCommit,
    commit,
    syncContext,
    openPreview,
    buildVerify,
  }
}
