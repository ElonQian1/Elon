import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type MutableRefObject,
  type PointerEvent as ReactPointerEvent,
  type SetStateAction,
} from 'react'
import { clamp, touch } from '../uiTunerGeometry'
import type { UiTunerDocument, UiTunerElement } from '../types'
import type { LivePatchOperation, LiveUiNode } from './liveUiApi'

export type CanvasGestureMode = 'move' | 'resize'

interface RuntimeGestureState {
  kind: 'runtime'
  id: string
  mode: CanvasGestureMode
  startX: number
  startY: number
  scale: number
  density: number
  gestureId: string
  baseX: number
  baseY: number
  baseWidth: number
  baseHeight: number
}

interface LocalGestureState {
  kind: 'local'
  id: string
  mode: CanvasGestureMode
  startX: number
  startY: number
  scale: number
  original: UiTunerElement
}

type GestureState = RuntimeGestureState | LocalGestureState

interface RuntimePatchBatch {
  gestureId: string
  operations: LivePatchOperation[]
}

interface UseRuntimeCanvasGestureOptions {
  documentRef: MutableRefObject<UiTunerDocument>
  setDocument: Dispatch<SetStateAction<UiTunerDocument>>
  setSelectedId: Dispatch<SetStateAction<string | null>>
  pushHistorySnapshot: (document: UiTunerDocument) => void
  selectedNode: LiveUiNode | null
  realRenderer: boolean
  runtimeConnected: boolean
  viewScale: number
  applyRuntimeGesture: (operations: LivePatchOperation[], gestureId: string) => Promise<unknown>
  setRuntimeGestureActive: (active: boolean) => void
  onNotice: (message: string) => void
}

const MIN_SIZE = 24

export function useRuntimeCanvasGesture(options: UseRuntimeCanvasGestureOptions) {
  const [gesture, setGesture] = useState<GestureState | null>(null)
  const [runtimeGestureActive, setRuntimeGestureActiveState] = useState(false)
  const localSnapshotRef = useRef<UiTunerDocument | null>(null)
  const localMovedRef = useRef(false)
  const pendingRuntimeRef = useRef<RuntimePatchBatch | null>(null)
  const flushPromiseRef = useRef<Promise<void> | null>(null)
  const applyRuntimeRef = useRef(options.applyRuntimeGesture)
  const finishRuntimeRef = useRef(options.setRuntimeGestureActive)
  const noticeRef = useRef(options.onNotice)

  useEffect(() => { applyRuntimeRef.current = options.applyRuntimeGesture }, [options.applyRuntimeGesture])
  useEffect(() => { finishRuntimeRef.current = options.setRuntimeGestureActive }, [options.setRuntimeGestureActive])
  useEffect(() => { noticeRef.current = options.onNotice }, [options.onNotice])

  const flushRuntime = useCallback(async () => {
    if (flushPromiseRef.current) return flushPromiseRef.current
    const task = (async () => {
      while (pendingRuntimeRef.current) {
        const pending = pendingRuntimeRef.current
        pendingRuntimeRef.current = null
        await applyRuntimeRef.current(pending.operations, pending.gestureId)
      }
    })()
    flushPromiseRef.current = task
    try {
      await task
    } catch (error) {
      pendingRuntimeRef.current = null
      noticeRef.current(error instanceof Error ? error.message : 'Android 实时手势失败')
    } finally {
      flushPromiseRef.current = null
    }
  }, [])

  useEffect(() => {
    if (!gesture) return undefined

    const handlePointerMove = (event: PointerEvent) => {
      event.preventDefault()
      const dxPx = (event.clientX - gesture.startX) / gesture.scale
      const dyPx = (event.clientY - gesture.startY) / gesture.scale
      if (gesture.kind === 'local') {
        localMovedRef.current = true
        options.setDocument((current) => touch({
          ...current,
          elements: current.elements.map((element) => {
            if (element.id !== gesture.id) return element
            if (gesture.mode === 'move') {
              return {
                ...element,
                x: clamp(gesture.original.x + dxPx, 0, current.canvas.width - element.width),
                y: clamp(gesture.original.y + dyPx, 0, current.canvas.height - element.height),
              }
            }
            return {
              ...element,
              width: clamp(gesture.original.width + dxPx, MIN_SIZE, current.canvas.width - element.x),
              height: clamp(gesture.original.height + dyPx, MIN_SIZE, current.canvas.height - element.y),
            }
          }),
        }))
        return
      }

      const dxDp = dxPx / gesture.density
      const dyDp = dyPx / gesture.density
      const operations = gesture.mode === 'move'
        ? [
            operation('translationX', gesture.baseX + dxDp),
            operation('translationY', gesture.baseY + dyDp),
          ]
        : [
            operation('width', Math.max(1, gesture.baseWidth + dxDp)),
            operation('height', Math.max(1, gesture.baseHeight + dyDp)),
          ]
      pendingRuntimeRef.current = { gestureId: gesture.gestureId, operations }
      void flushRuntime()
    }

    const handlePointerUp = () => {
      const finishing = gesture
      setGesture(null)
      if (finishing.kind === 'local') {
        if (localSnapshotRef.current && localMovedRef.current) {
          options.pushHistorySnapshot(localSnapshotRef.current)
        }
        localSnapshotRef.current = null
        localMovedRef.current = false
        return
      }
      void (async () => {
        await flushRuntime()
        if (pendingRuntimeRef.current) await flushRuntime()
        finishRuntimeRef.current(false)
        setRuntimeGestureActiveState(false)
        noticeRef.current('真实 Android 手势已完成，可撤销或写回源码')
      })()
    }

    window.addEventListener('pointermove', handlePointerMove)
    window.addEventListener('pointerup', handlePointerUp)
    return () => {
      window.removeEventListener('pointermove', handlePointerMove)
      window.removeEventListener('pointerup', handlePointerUp)
    }
  }, [flushRuntime, gesture, options])

  const canMove = supportsMove(options.selectedNode)
  const canResize = supportsResize(options.selectedNode)

  const startGesture = useCallback((
    event: ReactPointerEvent<HTMLElement>,
    element: UiTunerElement,
    mode: CanvasGestureMode,
  ) => {
    if (event.button !== 0) return
    event.stopPropagation()
    event.preventDefault()
    options.setSelectedId(element.id)

    if (options.realRenderer) {
      const node = options.selectedNode
      if (!options.runtimeConnected || !node || element.runtime?.nodeId !== node.runtimeNodeId) {
        options.onNotice(options.runtimeConnected
          ? '已选中真实组件，请再次拖动组件或右下角手柄'
          : '真实画面已冻结，Runtime 重连后才能拖动')
        return
      }
      if ((mode === 'move' && !supportsMove(node)) || (mode === 'resize' && !supportsResize(node))) {
        options.onNotice(mode === 'move' ? '这个组件不支持实时移动' : '这个组件不支持实时缩放')
        return
      }
      const density = Math.max(0.1, node.geometry.density || 1)
      const next: RuntimeGestureState = {
        kind: 'runtime',
        id: element.id,
        mode,
        startX: event.clientX,
        startY: event.clientY,
        scale: options.viewScale,
        density,
        gestureId: `canvas-${Date.now()}-${node.runtimeNodeId}`,
        baseX: numericProperty(node, 'translationX', 0),
        baseY: numericProperty(node, 'translationY', 0),
        baseWidth: numericProperty(node, 'width', element.width / density),
        baseHeight: numericProperty(node, 'height', element.height / density),
      }
      pendingRuntimeRef.current = null
      options.setRuntimeGestureActive(true)
      setRuntimeGestureActiveState(true)
      setGesture(next)
      return
    }

    localSnapshotRef.current = options.documentRef.current
    localMovedRef.current = false
    setGesture({
      kind: 'local',
      id: element.id,
      mode,
      startX: event.clientX,
      startY: event.clientY,
      scale: options.viewScale,
      original: element,
    })
  }, [options])

  return {
    startGesture,
    runtimeGestureActive,
    canMove: options.realRenderer && options.runtimeConnected && canMove,
    canResize: options.realRenderer && options.runtimeConnected && canResize,
  }
}

function operation(property: string, value: number): LivePatchOperation {
  return { property, value: { type: 'dp', value: Math.round(value * 100) / 100 } }
}

function numericProperty(node: LiveUiNode, property: string, fallback: number): number {
  const value = node.properties[property]?.effective?.value
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function supportsMove(node: LiveUiNode | null): boolean {
  return Boolean(
    node
    && node.capabilities.visualTranslatePreview
    && node.properties.translationX?.changeLevel === 'LIVE'
    && node.properties.translationY?.changeLevel === 'LIVE',
  )
}

function supportsResize(node: LiveUiNode | null): boolean {
  return Boolean(
    node
    && (node.capabilities.resizeWidth || node.capabilities.resizeHeight)
    && node.properties.width?.changeLevel === 'LIVE'
    && node.properties.height?.changeLevel === 'LIVE',
  )
}
