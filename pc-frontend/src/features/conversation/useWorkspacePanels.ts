import { useCallback, useEffect, useMemo, useState } from 'react'
import type { CSSProperties, PointerEvent as ReactPointerEvent } from 'react'

export type PanelSide = 'channel' | 'member'

interface WorkspacePanelState {
  channelWidth: number
  memberWidth: number
  channelCollapsed: boolean
  memberCollapsed: boolean
}

interface StoredWorkspacePanelState extends Partial<WorkspacePanelState> {
  version?: number
}

const STORAGE_KEY = 'elon.pc.workspacePanels.v2'
const COMPACT_RIGHT_PANEL_BREAKPOINT = 1440
const COLLAPSED_CHANNEL_WIDTH = 44

const CHANNEL_MIN = 220
const CHANNEL_MAX = 360
const CHANNEL_DEFAULT = 272
const MEMBER_MIN = 260
const MEMBER_MAX = 420
const MEMBER_DEFAULT = 300

export function useWorkspacePanels() {
  const [state, setState] = useState<WorkspacePanelState>(initialWorkspacePanelState)
  const [resizingSide, setResizingSide] = useState<PanelSide | null>(null)

  useEffect(() => {
    writeWorkspacePanelState(state)
  }, [state])

  const layoutStyle = useMemo(() => ({
    '--conversation-channel-width': `${state.channelWidth}px`,
    '--conversation-member-width': `${state.memberWidth}px`,
    '--conversation-channel-column': state.channelCollapsed ? `${COLLAPSED_CHANNEL_WIDTH}px` : `${state.channelWidth}px`,
    '--conversation-member-column': state.memberCollapsed ? '0px' : `minmax(${MEMBER_MIN}px, ${state.memberWidth}px)`,
  }) as CSSProperties, [state])

  const toggleChannelPanel = useCallback(() => {
    setState((current) => ({
      ...current,
      channelCollapsed: !current.channelCollapsed,
    }))
  }, [])

  const toggleMemberPanel = useCallback(() => {
    setState((current) => ({
      ...current,
      memberCollapsed: !current.memberCollapsed,
    }))
  }, [])

  const resetPanelWidth = useCallback((side: PanelSide) => {
    setState((current) => side === 'channel'
      ? { ...current, channelWidth: CHANNEL_DEFAULT, channelCollapsed: false }
      : { ...current, memberWidth: MEMBER_DEFAULT, memberCollapsed: false })
  }, [])

  const adjustPanelWidth = useCallback((side: PanelSide, delta: number) => {
    setState((current) => side === 'channel'
      ? { ...current, channelWidth: clamp(current.channelWidth + delta, CHANNEL_MIN, CHANNEL_MAX), channelCollapsed: false }
      : { ...current, memberWidth: clamp(current.memberWidth + delta, MEMBER_MIN, MEMBER_MAX), memberCollapsed: false })
  }, [])

  const startPanelResize = useCallback((side: PanelSide, event: ReactPointerEvent<HTMLElement>) => {
    event.preventDefault()
    const startX = event.clientX
    const startWidth = side === 'channel' ? state.channelWidth : state.memberWidth
    const originalCursor = document.body.style.cursor
    const originalUserSelect = document.body.style.userSelect

    setResizingSide(side)
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'

    function onPointerMove(moveEvent: PointerEvent) {
      const movement = moveEvent.clientX - startX
      const nextWidth = side === 'channel' ? startWidth + movement : startWidth - movement
      setState((current) => side === 'channel'
        ? {
            ...current,
            channelWidth: clamp(nextWidth, CHANNEL_MIN, CHANNEL_MAX),
            channelCollapsed: false,
          }
        : {
            ...current,
            memberWidth: clamp(nextWidth, MEMBER_MIN, MEMBER_MAX),
            memberCollapsed: false,
          })
    }

    function stopResize() {
      setResizingSide(null)
      document.body.style.cursor = originalCursor
      document.body.style.userSelect = originalUserSelect
      window.removeEventListener('pointermove', onPointerMove)
      window.removeEventListener('pointerup', stopResize)
      window.removeEventListener('pointercancel', stopResize)
    }

    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', stopResize)
    window.addEventListener('pointercancel', stopResize)
  }, [state.channelWidth, state.memberWidth])

  return {
    channelWidth: state.channelWidth,
    memberWidth: state.memberWidth,
    channelCollapsed: state.channelCollapsed,
    memberCollapsed: state.memberCollapsed,
    resizingSide,
    layoutStyle,
    toggleChannelPanel,
    toggleMemberPanel,
    resetPanelWidth,
    adjustPanelWidth,
    startPanelResize,
  }
}

export type WorkspacePanels = ReturnType<typeof useWorkspacePanels>

function initialWorkspacePanelState(): WorkspacePanelState {
  const stored = readWorkspacePanelState()
  const compactRightPanel = typeof window !== 'undefined'
    ? window.innerWidth <= COMPACT_RIGHT_PANEL_BREAKPOINT
    : false

  return {
    channelWidth: clamp(Number(stored?.channelWidth ?? CHANNEL_DEFAULT), CHANNEL_MIN, CHANNEL_MAX),
    memberWidth: clamp(Number(stored?.memberWidth ?? MEMBER_DEFAULT), MEMBER_MIN, MEMBER_MAX),
    channelCollapsed: stored?.channelCollapsed === true,
    memberCollapsed: typeof stored?.memberCollapsed === 'boolean'
      ? stored.memberCollapsed
      : compactRightPanel,
  }
}

function readWorkspacePanelState(): StoredWorkspacePanelState | null {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as StoredWorkspacePanelState
    return parsed && typeof parsed === 'object' ? parsed : null
  } catch {
    return null
  }
}

function writeWorkspacePanelState(state: WorkspacePanelState) {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, ...state }))
  } catch {
    // Local layout preference is best-effort only.
  }
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min
  return Math.min(max, Math.max(min, Math.round(value)))
}
