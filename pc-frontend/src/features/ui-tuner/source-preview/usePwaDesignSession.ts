import { useCallback, useEffect, useMemo, useRef, useState, type MutableRefObject } from 'react'
import { getAuthToken } from '../../../api/client'
import { listenForFitRunCodexSettled, requestCodexForFitRun } from '../fit-run/fitRunEvents'
import { buildPwaDesignContextPack } from './pwaDesignContext'
import {
  applyDeterministicAndroidWriteback,
  planPwaDesignWriteback,
  type PwaDeterministicWritebackResult,
} from './pwaDesignWriteback'
import { matchPwaSourceNode, pwaSourceBinding } from './pwaNodeMapping'
import {
  normalizePwaRoute,
  stringifyPwaDraftCliPackage,
  type PwaDesignDraft,
  type PwaDomContextNode,
  type PwaElementIdentity,
  type PwaOriginalStyleSnapshot,
  type PwaRouteIdentity,
  type PwaStyleProperty,
  resolvedPwaAfterStyle,
  stablePwaIdentityKey,
} from './pwaDesignDraft'
import { PwaDesignSessionModel } from './pwaDesignSessionModel'
import type { SourcePreviewNode } from './types'

const BRIDGE_SOURCE = 'elon-pwa-design-bridge'
const PARENT_SOURCE = 'elon-pc-ui-tuner'
const PROTOCOL_VERSION = 1

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
  syncState: PwaSyncState
  writebackPlan: ReturnType<typeof planPwaDesignWriteback>
  setMode: (mode: 'select' | 'interact') => void
  updateStyle: (property: PwaStyleProperty, value: string) => void
  resetCurrent: () => void
  clearPage: () => void
  undo: () => void
  redo: () => void
  saveNow: () => void
  syncNow: () => Promise<void>
  copyCliPackage: () => Promise<void>
  downloadCliPackage: () => void
  prepareReload: () => void
}

function bridgeElements(draft: PwaDesignDraft) {
  return Object.values(draft.elements).map((element) => ({
    selector: element.identity.selector,
    styleDiff: element.styleDiff,
  }))
}

function bridgeDraftKey(draft: PwaDesignDraft): string {
  return [
    draft.project.id,
    draft.route.path,
    draft.route.search,
    draft.route.hash,
    draft.route.screenKey || 'screen:unidentified',
    `${draft.viewport.width}x${draft.viewport.height}`,
  ].join('|')
}

export interface PwaSyncState {
  phase: 'idle' | 'starting' | 'running' | 'completed' | 'failed'
  message: string
  taskId?: string
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
  return `${normalized.path}${normalized.search}${normalized.hash}#${normalized.screenKey || 'screen:unidentified'}@${normalized.viewport.width}x${normalized.viewport.height}`
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
  const [syncState, setSyncState] = useState<PwaSyncState>({ phase: 'idle', message: '等待草稿' })
  const routeRef = useRef<PwaRouteState | null>(null)
  const modeRef = useRef(modeState)
  const syncTaskIdRef = useRef('')
  const modelRef = useRef<PwaDesignSessionModel | null>(null)
  if (!modelRef.current) modelRef.current = new PwaDesignSessionModel()
  const model = modelRef.current
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
    post('apply-draft', value ? {
      draftKey: bridgeDraftKey(value),
      revision: value.revision,
      elements: bridgeElements(value),
    } : { draftKey: '', revision: 0, elements: [] })
  }, [post])

  const applyDraftState = useCallback((value: PwaDesignDraft, sync = true) => {
    setDraft(value)
    if (sync) syncDraft(value)
    if (Object.keys(value.elements).length) {
      setSaveLabel(`已自动保存 · r${value.revision}`)
    } else {
      setSaveLabel('本页暂无样式草稿')
    }
    setHistoryVersion((version) => version + 1)
  }, [syncDraft])

  useEffect(() => () => model.dispose(), [model])

  useEffect(() => listenForFitRunCodexSettled((detail) => {
    if (!syncTaskIdRef.current || detail.taskId !== syncTaskIdRef.current) return
    setSyncState(detail.succeeded
      ? { phase: 'completed', taskId: detail.taskId, message: 'PWA 与 APK 同步任务已完成' }
      : { phase: 'failed', taskId: detail.taskId, message: '跨端 Codex 写回失败，可保留草稿后重试' })
    if (detail.succeeded) setSaveLabel('跨端草稿已写回源码')
    syncTaskIdRef.current = ''
  }), [])

  const bridgeContextRef = useRef({ model, onSelect, post, project, root, syncDraft })
  bridgeContextRef.current = { model, onSelect, post, project, root, syncDraft }

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
      const context = bridgeContextRef.current
      if (message.type === 'ready') {
        setReady(true)
        const token = getAuthToken()
        if (token) context.post('set-session-auth', { token })
        context.post('set-mode', { mode: modeRef.current })
        if (context.model.draft) context.syncDraft(context.model.draft)
        return
      }
      if (message.type === 'route-changed' && message.payload?.path && message.payload.viewport) {
        const normalized = normalizePwaRoute(message.payload as PwaRouteState)
        const nextRoute: PwaRouteState = { ...(message.payload as PwaRouteState), ...normalized }
        const changed = !routeRef.current || routeKey(routeRef.current) !== routeKey(nextRoute)
        routeRef.current = nextRoute
        setRoute(nextRoute)
        if (changed) {
          const { draft: restored, restored: didRestore } = context.model.restore(context.project, nextRoute)
          setDraft(restored)
          setHistoryVersion((value) => value + 1)
          setSelection(null)
          setMappedNodeKey(null)
          setUnboundLabel('')
          setSaveLabel(didRestore && Object.keys(restored.elements).length ? `已恢复本页草稿 · r${restored.revision}` : '本页暂无样式草稿')
          context.syncDraft(restored)
        }
        return
      }
      if (message.type === 'selection' && message.payload?.node) {
        const nextSelection = message.payload.node
        setSelection(nextSelection)
        if (context.root) {
          const match = matchPwaSourceNode(context.root, nextSelection.identity)
          if (match) {
            setUnboundLabel('')
            setMappedNodeKey(match.key)
            context.onSelect(match.key)
            return
          }
        }
        setMappedNodeKey(null)
        setUnboundLabel(nextSelection.identity.ariaLabel || nextSelection.identity.text || nextSelection.identity.id || nextSelection.identity.tag)
      }
    }
    window.addEventListener('message', receive)
    return () => window.removeEventListener('message', receive)
  }, [])

  const setMode = useCallback((nextMode: 'select' | 'interact') => {
    modeRef.current = nextMode
    setModeState(nextMode)
    post('set-mode', { mode: nextMode })
  }, [post])

  const updateStyle = useCallback((property: PwaStyleProperty, input: string) => {
    const current = model.draft
    if (!current || !selection) return
    const stableKey = stablePwaIdentityKey(selection.identity)
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
    const next = model.update(`${stableKey}:${property}`, () => elements)
    if (next) applyDraftState(next)
  }, [applyDraftState, model, root, selection])

  const resetCurrent = useCallback(() => {
    const current = model.draft
    if (!current || !selection) return
    const found = draftEntry(current, selection.identity)
    if (!found) return
    const elements = { ...current.elements }
    delete elements[found.key]
    const next = model.update(`${found.key}:reset`, () => elements)
    if (next) applyDraftState(next)
  }, [applyDraftState, model, selection])

  const clearPage = useCallback(() => {
    const current = model.draft
    if (!current || !Object.keys(current.elements).length) return
    const next = model.update('page:clear', () => ({}))
    if (next) applyDraftState(next)
  }, [applyDraftState, model])

  const undo = useCallback(() => {
    const previous = model.undo()
    if (previous) applyDraftState(previous)
  }, [applyDraftState, model])

  const redo = useCallback(() => {
    const next = model.redo()
    if (next) applyDraftState(next)
  }, [applyDraftState, model])

  const saveNow = useCallback(() => {
    const current = model.save()
    if (!current) return
    setSaveLabel(Object.keys(current.elements).length ? `草稿已保存 · r${current.revision}` : '本页暂无样式草稿')
  }, [model])

  const writebackPlan = useMemo(() => planPwaDesignWriteback(draft, root), [draft, root])

  const syncNow = useCallback(async () => {
    const current = model.draft
    if (!current || !Object.keys(current.elements).length) return
    model.save()
    setSyncState({ phase: 'starting', message: '正在保存草稿并执行确定性写回…' })
    let deterministicResult: PwaDeterministicWritebackResult = {
      applied: 0,
      sourceRevision: current.project.sourceRevision,
      changedFiles: [] as string[],
    }
    try {
      deterministicResult = await applyDeterministicAndroidWriteback({
        draft: current,
        root,
        projectRoot: current.project.workspaceIdentity,
        sourceRevision: current.project.sourceRevision,
      })
    } catch (error) {
      deterministicResult.error = error instanceof Error ? error.message : '确定性 Android 写回失败'
    }
    const latest = deterministicResult.applied ? {
      ...current,
      project: { ...current.project, sourceRevision: deterministicResult.sourceRevision },
      updatedAt: new Date().toISOString(),
    } : current
    if (latest !== current) applyDraftState(model.replace(latest), false)
    const plan = planPwaDesignWriteback(latest, root)
    if (deterministicResult.error) plan.codexReasons.push(`确定性写回需 Codex 接管：${deterministicResult.error}`)
    const contextPack = buildPwaDesignContextPack({
      draft: latest,
      root,
      selection,
      plan,
      deterministicResult,
    })
    try {
      const handoffId = `pwa_${Date.now()}`
      const { taskId } = await requestCodexForFitRun({
        runId: `pwa:${latest.project.id}:${latest.revision}`,
        handoffId,
        handoffKind: 'PWA_DRAFT',
        contextPack,
        reason: plan.codexReasons.join('；'),
      })
      syncTaskIdRef.current = taskId
      setSyncState({
        phase: 'running', taskId,
        message: deterministicResult.applied
          ? `Android 已确定性写回 ${deterministicResult.applied} 个节点，Codex 正在补齐 PWA/复杂修改`
          : '已进入现有 Codex 会话，正在建立来源绑定并写回双端源码',
      })
    } catch (error) {
      setSyncState({ phase: 'failed', message: error instanceof Error ? error.message : '跨端同步任务启动失败' })
    }
  }, [applyDraftState, model, root, selection])

  const copyCliPackage = useCallback(async () => {
    const current = model.draft
    if (!current) return
    try {
      await navigator.clipboard.writeText(stringifyPwaDraftCliPackage(current))
      setSaveLabel('CLI 包已复制 · 不含整仓库或 Base64 截图')
    } catch {
      setSaveLabel('浏览器禁止复制，请改用下载 CLI 包')
    }
  }, [model])

  const downloadCliPackage = useCallback(() => {
    const current = model.draft
    if (!current) return
    const blob = new Blob([stringifyPwaDraftCliPackage(current)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = `pwa-design-${current.project.id || 'project'}-r${current.revision}.json`
    anchor.click()
    URL.revokeObjectURL(url)
    setSaveLabel('CLI 包已下载')
  }, [model])

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
    canUndo: model.canUndo && historyVersion >= 0,
    canRedo: model.canRedo && historyVersion >= 0,
    saveLabel,
    syncState,
    writebackPlan,
    setMode,
    updateStyle,
    resetCurrent,
    clearPage,
    undo,
    redo,
    saveNow,
    syncNow,
    copyCliPackage,
    downloadCliPackage,
    prepareReload,
  }
}
