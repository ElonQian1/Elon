import { useCallback, useEffect, useMemo, useRef, useState, type MutableRefObject } from 'react'
import { getAuthToken } from '../../../api/client'
import { matchPwaSourceNode, pwaSourceBinding } from './pwaNodeMapping'
import {
  createPwaDesignDraft,
  normalizePwaRoute,
  readPwaDesignDraft,
  removePwaDesignDraft,
  savePwaDesignDraft,
  type PwaDesignDraft,
  type PwaDomContextNode,
  type PwaElementIdentity,
  type PwaOriginalStyleSnapshot,
  type PwaRouteIdentity,
  type PwaStyleProperty,
  resolvedPwaAfterStyle,
  stablePwaIdentityKey,
} from './pwaDesignDraft'
import type { SourcePreviewNode } from './types'

const BRIDGE_SOURCE = 'elon-pwa-design-bridge'
const PARENT_SOURCE = 'elon-pc-ui-tuner'
const PROTOCOL_VERSION = 1
const HISTORY_LIMIT = 60
const TRANSACTION_IDLE_MS = 450

export interface PwaSelection {
  identity: PwaElementIdentity
  rect: { left: number; top: number; width: number; height: number }
  originalStyle: PwaOriginalStyleSnapshot
  domContext: PwaDomContextNode[]
}

export interface PwaRouteState extends PwaRouteIdentity {
  href: string
  title: string
  scroll?: { x: number; y: number }
}

interface UsePwaDesignSessionOptions {
  projectId: string
  workspaceIdentity: string
  sourceRevision: string
  root: SourcePreviewNode | null
  onSelect: (key: string) => void
}

export interface PwaDesignSession {
  iframeRef: MutableRefObject<HTMLIFrameElement | null>
  ready: boolean
  mode: 'select' | 'interact'
  selection: PwaSelection | null
  route: PwaRouteState | null
  draft: PwaDesignDraft | null
  mappedNodeKey: string | null
  unboundLabel: string
  canUndo: boolean
  canRedo: boolean
  saveLabel: string
  setMode: (mode: 'select' | 'interact') => void
  updateStyle: (property: PwaStyleProperty, value: string) => void
  resetCurrent: () => void
  clearPage: () => void
  undo: () => void
  redo: () => void
  saveNow: () => void
  prepareReload: () => void
}

function touchDraft(draft: PwaDesignDraft, elements: PwaDesignDraft['elements']): PwaDesignDraft {
  return {
    ...draft,
    elements,
    revision: draft.revision + 1,
    updatedAt: new Date().toISOString(),
  }
}

function bridgeElements(draft: PwaDesignDraft) {
  return Object.values(draft.elements).map((element) => ({
    selector: element.identity.selector,
    styleDiff: element.styleDiff,
  }))
}

function draftEntry(draft: PwaDesignDraft, identity: PwaElementIdentity) {
  const stableKey = stablePwaIdentityKey(identity)
  const direct = draft.elements[stableKey]
  if (direct) return { key: stableKey, element: direct }
  const legacy = Object.entries(draft.elements).find(([, element]) => element.identity.selector === identity.selector)
  return legacy ? { key: legacy[0], element: legacy[1] } : null
}

function routeKey(route: PwaRouteIdentity): string {
  const normalized = normalizePwaRoute(route)
  return `${normalized.path}${normalized.search}${normalized.hash}@${normalized.viewport.width}x${normalized.viewport.height}`
}

export function usePwaDesignSession({
  projectId,
  workspaceIdentity,
  sourceRevision,
  root,
  onSelect,
}: UsePwaDesignSessionOptions): PwaDesignSession {
  const iframeRef = useRef<HTMLIFrameElement>(null)
  const [ready, setReady] = useState(false)
  const [modeState, setModeState] = useState<'select' | 'interact'>('interact')
  const [selection, setSelection] = useState<PwaSelection | null>(null)
  const [route, setRoute] = useState<PwaRouteState | null>(null)
  const [draft, setDraft] = useState<PwaDesignDraft | null>(null)
  const [mappedNodeKey, setMappedNodeKey] = useState<string | null>(null)
  const [unboundLabel, setUnboundLabel] = useState('')
  const [historyVersion, setHistoryVersion] = useState(0)
  const [saveLabel, setSaveLabel] = useState('等待进入真实页面')
  const draftRef = useRef<PwaDesignDraft | null>(null)
  const routeRef = useRef<PwaRouteState | null>(null)
  const modeRef = useRef(modeState)
  const pastRef = useRef<PwaDesignDraft[]>([])
  const futureRef = useRef<PwaDesignDraft[]>([])
  const transactionRef = useRef<{ key: string; timer: number } | null>(null)
  const project = useMemo(() => ({
    id: projectId || workspaceIdentity || 'unknown-project',
    workspaceIdentity: workspaceIdentity || projectId || 'unknown-workspace',
    sourceRevision,
  }), [projectId, sourceRevision, workspaceIdentity])

  const post = useCallback((type: string, payload: unknown) => {
    iframeRef.current?.contentWindow?.postMessage({
      source: PARENT_SOURCE,
      protocolVersion: PROTOCOL_VERSION,
      type,
      payload,
    }, window.location.origin)
  }, [])

  const syncDraft = useCallback((value: PwaDesignDraft | null) => {
    post('apply-draft', { elements: value ? bridgeElements(value) : [] })
  }, [post])

  const persist = useCallback((value: PwaDesignDraft) => {
    if (Object.keys(value.elements).length) {
      savePwaDesignDraft(value)
      setSaveLabel(`已自动保存 · r${value.revision}`)
    } else {
      removePwaDesignDraft(value)
      setSaveLabel('本页暂无样式草稿')
    }
  }, [])

  const applyDraftState = useCallback((value: PwaDesignDraft, sync = true) => {
    draftRef.current = value
    setDraft(value)
    persist(value)
    if (sync) syncDraft(value)
  }, [persist, syncDraft])

  const closeTransaction = useCallback(() => {
    const transaction = transactionRef.current
    if (transaction) window.clearTimeout(transaction.timer)
    transactionRef.current = null
  }, [])

  const beginTransaction = useCallback((key: string) => {
    const current = draftRef.current
    if (!current) return
    const active = transactionRef.current
    if (!active || active.key !== key) {
      closeTransaction()
      pastRef.current = [...pastRef.current, current].slice(-HISTORY_LIMIT)
      futureRef.current = []
      setHistoryVersion((value) => value + 1)
    } else {
      window.clearTimeout(active.timer)
    }
    transactionRef.current = {
      key,
      timer: window.setTimeout(() => { transactionRef.current = null }, TRANSACTION_IDLE_MS),
    }
  }, [closeTransaction])

  useEffect(() => () => closeTransaction(), [closeTransaction])

  useEffect(() => {
    const receive = (event: MessageEvent) => {
      if (event.origin !== window.location.origin || event.source !== iframeRef.current?.contentWindow) return
      const message = event.data as {
        source?: string
        protocolVersion?: number
        type?: string
        payload?: Partial<PwaRouteState> & { node?: PwaSelection }
      }
      if (message.source !== BRIDGE_SOURCE || message.protocolVersion !== PROTOCOL_VERSION) return
      if (message.type === 'ready') {
        setReady(true)
        const token = getAuthToken()
        if (token) post('set-session-auth', { token })
        post('set-mode', { mode: modeRef.current })
        if (draftRef.current) syncDraft(draftRef.current)
      }
      if (message.type === 'route-changed' && message.payload?.path && message.payload.viewport) {
        const normalized = normalizePwaRoute(message.payload as PwaRouteState)
        const nextRoute: PwaRouteState = { ...(message.payload as PwaRouteState), ...normalized }
        const changed = !routeRef.current || routeKey(routeRef.current) !== routeKey(nextRoute)
        routeRef.current = nextRoute
        setRoute(nextRoute)
        if (changed) {
          closeTransaction()
          const restored = readPwaDesignDraft(project, nextRoute) ?? createPwaDesignDraft(project, nextRoute)
          draftRef.current = restored
          setDraft(restored)
          pastRef.current = []
          futureRef.current = []
          setHistoryVersion((value) => value + 1)
          setSelection(null)
          setMappedNodeKey(null)
          setUnboundLabel('')
          setSaveLabel(Object.keys(restored.elements).length ? `已恢复本页草稿 · r${restored.revision}` : '本页暂无样式草稿')
          syncDraft(restored)
        }
      }
      if (message.type === 'selection' && message.payload?.node) {
        const nextSelection = message.payload.node
        setSelection(nextSelection)
        if (root) {
          const match = matchPwaSourceNode(root, nextSelection.identity)
          if (match) {
            setUnboundLabel('')
            setMappedNodeKey(match.key)
            onSelect(match.key)
            return
          }
        }
        setMappedNodeKey(null)
        setUnboundLabel(nextSelection.identity.ariaLabel || nextSelection.identity.text || nextSelection.identity.id || nextSelection.identity.tag)
      }
    }
    window.addEventListener('message', receive)
    return () => window.removeEventListener('message', receive)
  }, [closeTransaction, onSelect, post, project, root, syncDraft])

  const setMode = useCallback((nextMode: 'select' | 'interact') => {
    modeRef.current = nextMode
    setModeState(nextMode)
    post('set-mode', { mode: nextMode })
  }, [post])

  const updateStyle = useCallback((property: PwaStyleProperty, input: string) => {
    const current = draftRef.current
    if (!current || !selection) return
    const stableKey = stablePwaIdentityKey(selection.identity)
    beginTransaction(`${stableKey}:${property}`)
    const found = draftEntry(current, selection.identity)
    const existing = found?.element
    const originalStyle = existing?.originalStyle ?? selection.originalStyle
    const originalValue = originalStyle.authored[property] || originalStyle.computed[property] || ''
    const styleDiff = { ...(existing?.styleDiff ?? {}) }
    const value = input.trim()
    if (!value || value === originalValue) delete styleDiff[property]
    else styleDiff[property] = value
    const elements = { ...current.elements }
    if (Object.keys(styleDiff).length) {
      const revision = (existing?.revision ?? 0) + 1
      const now = new Date().toISOString()
      if (found && found.key !== stableKey) delete elements[found.key]
      elements[stableKey] = {
        identity: { ...selection.identity, key: stableKey },
        originalStyle,
        afterStyle: resolvedPwaAfterStyle(originalStyle, styleDiff),
        styleDiff,
        binding: pwaSourceBinding({ ...selection.identity, key: stableKey }, root),
        scope: existing?.scope ?? 'instance',
        domContext: selection.domContext ?? [],
        visualReferences: existing?.visualReferences ?? {},
        revision,
        createdAt: existing?.createdAt ?? now,
        updatedAt: now,
      }
    } else {
      if (found) delete elements[found.key]
    }
    applyDraftState(touchDraft(current, elements))
  }, [applyDraftState, beginTransaction, root, selection])

  const resetCurrent = useCallback(() => {
    const current = draftRef.current
    if (!current || !selection) return
    const found = draftEntry(current, selection.identity)
    if (!found) return
    beginTransaction(`${found.key}:reset`)
    const elements = { ...current.elements }
    delete elements[found.key]
    applyDraftState(touchDraft(current, elements))
  }, [applyDraftState, beginTransaction, selection])

  const clearPage = useCallback(() => {
    const current = draftRef.current
    if (!current || !Object.keys(current.elements).length) return
    beginTransaction('page:clear')
    applyDraftState(touchDraft(current, {}))
  }, [applyDraftState, beginTransaction])

  const undo = useCallback(() => {
    closeTransaction()
    const previous = pastRef.current.pop()
    const current = draftRef.current
    if (!previous || !current) return
    futureRef.current = [...futureRef.current, current].slice(-HISTORY_LIMIT)
    setHistoryVersion((value) => value + 1)
    applyDraftState(previous)
  }, [applyDraftState, closeTransaction])

  const redo = useCallback(() => {
    closeTransaction()
    const next = futureRef.current.pop()
    const current = draftRef.current
    if (!next || !current) return
    pastRef.current = [...pastRef.current, current].slice(-HISTORY_LIMIT)
    setHistoryVersion((value) => value + 1)
    applyDraftState(next)
  }, [applyDraftState, closeTransaction])

  const saveNow = useCallback(() => {
    const current = draftRef.current
    if (!current) return
    persist(current)
    setSaveLabel(Object.keys(current.elements).length ? `草稿已保存 · r${current.revision}` : '本页暂无样式草稿')
  }, [persist])

  const prepareReload = useCallback(() => {
    setReady(false)
    setSelection(null)
    setMappedNodeKey(null)
    setUnboundLabel('')
  }, [])

  return {
    iframeRef,
    ready,
    mode: modeState,
    selection,
    route,
    draft,
    mappedNodeKey,
    unboundLabel,
    canUndo: pastRef.current.length > 0 && historyVersion >= 0,
    canRedo: futureRef.current.length > 0 && historyVersion >= 0,
    saveLabel,
    setMode,
    updateStyle,
    resetCurrent,
    clearPage,
    undo,
    redo,
    saveNow,
    prepareReload,
  }
}
