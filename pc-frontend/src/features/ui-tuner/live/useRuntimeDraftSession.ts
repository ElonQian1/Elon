import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type {
  LivePatchAck,
  LivePatchOperation,
  LiveUiFrame,
  LiveUiNode,
  LiveUiScope,
} from './liveUiApi'
import {
  EMPTY_RUNTIME_DRAFT_STATE,
  acknowledgeRuntimeDraft,
  applyRuntimeDraftOperations,
  confirmRuntimeDraftFrame,
  markRuntimeDraftSyncing,
  rejectRuntimeDraft,
  runtimeDraftStatus,
  type RuntimeDraftState,
} from './runtimeDraftModel'

interface QueuedRuntimeDraft {
  epoch: number
  key: string
  nodeIds: string[]
  revision: number
  operations: LivePatchOperation[]
  send: () => Promise<LivePatchAck>
}

interface UseRuntimeDraftSessionOptions {
  resetKey: string
  frame: LiveUiFrame | null
  nodes: LiveUiNode[]
  selectedNode: LiveUiNode | null
  applyRemote: (operation: LivePatchOperation, scope: LiveUiScope) => Promise<LivePatchAck>
  applyGestureRemote: (operations: LivePatchOperation[], gestureId: string) => Promise<LivePatchAck>
  onNotice: (message: string) => void
}

export function useRuntimeDraftSession({
  resetKey,
  frame,
  nodes,
  selectedNode,
  applyRemote,
  applyGestureRemote,
  onNotice,
}: UseRuntimeDraftSessionOptions) {
  const [state, setState] = useState<RuntimeDraftState>(EMPTY_RUNTIME_DRAFT_STATE)
  const stateRef = useRef(state)
  const pendingRef = useRef(new Map<string, QueuedRuntimeDraft>())
  const flushingRef = useRef(false)
  const resetKeyRef = useRef(resetKey)
  const epochRef = useRef(0)
  const historyRef = useRef<{ past: RuntimeDraftState[]; future: RuntimeDraftState[] }>({
    past: [],
    future: [],
  })
  const gestureHistoryRef = useRef(new Set<string>())
  const [, setHistoryRevision] = useState(0)

  const replaceState = useCallback((update: (current: RuntimeDraftState) => RuntimeDraftState) => {
    const next = update(stateRef.current)
    stateRef.current = next
    setState(next)
    return next
  }, [])

  const reset = useCallback(() => {
    epochRef.current += 1
    pendingRef.current.clear()
    historyRef.current = { past: [], future: [] }
    gestureHistoryRef.current.clear()
    stateRef.current = EMPTY_RUNTIME_DRAFT_STATE
    setState(EMPTY_RUNTIME_DRAFT_STATE)
    setHistoryRevision((current) => current + 1)
  }, [])

  useEffect(() => {
    if (resetKeyRef.current === resetKey) return
    resetKeyRef.current = resetKey
    reset()
  }, [reset, resetKey])

  useEffect(() => {
    replaceState((current) => confirmRuntimeDraftFrame(current, frame))
  }, [frame, replaceState])

  const flush = useCallback(async () => {
    if (flushingRef.current) return
    flushingRef.current = true
    try {
      while (pendingRef.current.size > 0) {
        const next = pendingRef.current.values().next().value as QueuedRuntimeDraft | undefined
        if (!next) break
        pendingRef.current.delete(next.key)
        if (next.epoch !== epochRef.current) continue
        replaceState((current) => next.nodeIds.reduce(
          (drafts, nodeId) => markRuntimeDraftSyncing(drafts, nodeId, next.revision),
          current,
        ))
        try {
          const ack = await next.send()
          if (next.epoch !== epochRef.current) continue
          replaceState((current) => next.nodeIds.reduce(
            (drafts, nodeId) => acknowledgeRuntimeDraft(drafts, nodeId, next.revision, ack),
            current,
          ))
        } catch (error) {
          if (next.epoch !== epochRef.current) continue
          const message = error instanceof Error ? error.message : 'Android 后台同步失败'
          replaceState((current) => next.nodeIds.reduce(
            (drafts, nodeId) => rejectRuntimeDraft(drafts, nodeId, next.revision, message),
            current,
          ))
          onNotice(`PC 本地草稿仍然保留；${message}`)
        }
      }
    } finally {
      flushingRef.current = false
      if (pendingRef.current.size > 0) void flush()
    }
  }, [onNotice, replaceState])

  const preview = useCallback((
    operations: LivePatchOperation[],
    scope: LiveUiScope,
  ) => {
    if (!selectedNode) throw new Error('当前选中元素尚未绑定 Live Node')
    const targets = scope === 'DEFINITION'
      ? nodes.filter((node) => node.definitionId === selectedNode.definitionId)
      : [selectedNode]
    const next = replaceState((current) => targets.reduce(
      (drafts, node) => applyRuntimeDraftOperations(drafts, node, operations, frame),
      current,
    ))
    return {
      nodeIds: targets.map((node) => node.runtimeNodeId),
      revision: next.revision,
    }
  }, [frame, nodes, replaceState, selectedNode])

  const queue = useCallback((draft: QueuedRuntimeDraft) => {
    pendingRef.current.set(draft.key, draft)
    void flush()
  }, [flush])

  const rememberHistory = useCallback((gestureId?: string) => {
    if (gestureId && gestureHistoryRef.current.has(gestureId)) return
    if (gestureId) gestureHistoryRef.current.add(gestureId)
    historyRef.current.past.push(stateRef.current)
    historyRef.current.future = []
    setHistoryRevision((current) => current + 1)
  }, [])

  const apply = useCallback((operation: LivePatchOperation, scope: LiveUiScope) => {
    const target = selectedNode
    if (!target) return Promise.reject(new Error('当前选中元素尚未绑定 Live Node'))
    rememberHistory()
    const local = preview([operation], scope)
    queue({
      epoch: epochRef.current,
      key: `${target.runtimeNodeId}:${scope}:${operation.property}`,
      ...local,
      operations: [operation],
      send: () => applyRemote(operation, scope),
    })
    return Promise.resolve({ queued: true, revision: local.revision })
  }, [applyRemote, preview, queue, rememberHistory, selectedNode])

  const applyGesture = useCallback((operations: LivePatchOperation[], gestureId: string) => {
    const target = selectedNode
    if (!target) return Promise.reject(new Error('当前选中元素尚未绑定 Live Node'))
    rememberHistory(gestureId)
    const local = preview(operations, 'INSTANCE')
    queue({
      epoch: epochRef.current,
      key: `${target.runtimeNodeId}:gesture:${gestureId}`,
      ...local,
      operations,
      send: () => applyGestureRemote(operations, gestureId),
    })
    return Promise.resolve({ queued: true, revision: local.revision })
  }, [applyGestureRemote, preview, queue, rememberHistory, selectedNode])

  const restoreHistory = useCallback((direction: 'undo' | 'redo') => {
    const source = direction === 'undo' ? historyRef.current.past : historyRef.current.future
    const destination = direction === 'undo' ? historyRef.current.future : historyRef.current.past
    const snapshot = source.pop()
    if (!snapshot) return false
    epochRef.current += 1
    pendingRef.current.clear()
    destination.push(stateRef.current)
    stateRef.current = snapshot
    setState(snapshot)
    setHistoryRevision((current) => current + 1)
    return true
  }, [])

  const undoLocal = useCallback(() => restoreHistory('undo'), [restoreHistory])
  const redoLocal = useCallback(() => restoreHistory('redo'), [restoreHistory])

  return useMemo(() => ({
    state,
    status: runtimeDraftStatus(state),
    active: Object.keys(state.nodes).length > 0,
    apply,
    applyGesture,
    reset,
    undoLocal,
    redoLocal,
    canUndo: historyRef.current.past.length > 0,
    canRedo: historyRef.current.future.length > 0,
  }), [apply, applyGesture, redoLocal, reset, state, undoLocal])
}
