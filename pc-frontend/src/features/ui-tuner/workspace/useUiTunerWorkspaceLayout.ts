import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

const STORAGE_PREFIX = 'elon.uiTuner.canvasLayout.v1'
const DEFAULT_SPLIT_RATIO = 35

interface CanvasLayoutState {
  designPaneOpen: boolean
  splitRatio: number
  leftPanelOpen: boolean
  rightPanelOpen: boolean
  focusMode: boolean
}

function clampSplitRatio(value: number) {
  return Math.round(Math.min(Math.max(value, 20), 80))
}

function loadState(storageKey: string): CanvasLayoutState {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(storageKey) ?? '') as Partial<CanvasLayoutState>
    return {
      designPaneOpen: parsed.designPaneOpen !== false,
      splitRatio: clampSplitRatio(Number(parsed.splitRatio) || DEFAULT_SPLIT_RATIO),
      leftPanelOpen: parsed.leftPanelOpen !== false,
      rightPanelOpen: parsed.rightPanelOpen !== false,
      focusMode: parsed.focusMode === true,
    }
  } catch {
    return {
      designPaneOpen: true,
      splitRatio: DEFAULT_SPLIT_RATIO,
      leftPanelOpen: true,
      rightPanelOpen: true,
      focusMode: false,
    }
  }
}

export function useUiTunerWorkspaceLayout(storageScope: string, hasTarget: boolean) {
  const storageKey = useMemo(() => `${STORAGE_PREFIX}:${storageScope || 'default'}`, [storageScope])
  const [state, setState] = useState<CanvasLayoutState>(() => loadState(storageKey))
  const activeStorageKey = useRef(storageKey)
  const previousHasTarget = useRef(hasTarget)

  useEffect(() => {
    if (activeStorageKey.current === storageKey) return
    activeStorageKey.current = storageKey
    setState(loadState(storageKey))
  }, [storageKey])

  useEffect(() => {
    if (!previousHasTarget.current && hasTarget) {
      setState((current) => ({ ...current, designPaneOpen: true }))
    }
    previousHasTarget.current = hasTarget
  }, [hasTarget])

  useEffect(() => {
    window.localStorage.setItem(storageKey, JSON.stringify(state))
  }, [state, storageKey])

  const setSplitRatio = useCallback((splitRatio: number) => {
    setState((current) => ({ ...current, splitRatio: clampSplitRatio(splitRatio) }))
  }, [])

  const toggleDesignPane = useCallback(() => {
    if (!hasTarget) return
    setState((current) => ({ ...current, designPaneOpen: !current.designPaneOpen }))
  }, [hasTarget])

  const toggleLeftPanel = useCallback(() => {
    setState((current) => ({ ...current, leftPanelOpen: !current.leftPanelOpen, focusMode: false }))
  }, [])

  const toggleRightPanel = useCallback(() => {
    setState((current) => ({ ...current, rightPanelOpen: !current.rightPanelOpen, focusMode: false }))
  }, [])

  const toggleFocusMode = useCallback(() => {
    setState((current) => ({ ...current, focusMode: !current.focusMode }))
  }, [])

  return {
    designPaneOpen: hasTarget && state.designPaneOpen,
    splitRatio: state.splitRatio,
    leftPanelOpen: !state.focusMode && state.leftPanelOpen,
    rightPanelOpen: !state.focusMode && state.rightPanelOpen,
    focusMode: state.focusMode,
    setSplitRatio,
    toggleDesignPane,
    toggleLeftPanel,
    toggleRightPanel,
    toggleFocusMode,
  }
}
