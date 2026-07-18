import { useCallback, useEffect, useMemo, useRef, useState, type MutableRefObject } from 'react'
import { getAuthToken } from '../../../api/client'
import { listenForFitRunCodexSettled, requestCodexForFitRun } from '../fit-run/fitRunEvents'
import { buildPwaDesignContextPack } from './pwaDesignContext'
import {
  applyDeterministicAndroidWriteback,
  applyDeterministicPwaWriteback,
  planPwaDesignWriteback,
  recordDeterministicWriteback,
  type PwaCrossPlatformWritebackResult,
} from './pwaDesignWriteback'
import { matchPwaSourceNode, pwaSourceBinding } from './pwaNodeMapping'
import {
  normalizePwaRoute,
  removePwaDesignDraft,
  stringifyPwaDraftCliPackage,
  type PwaDesignDraft,
  type PwaDomContextNode,
  type PwaElementIdentity,
  type PwaExplicitStyleBinding,
  type PwaOriginalStyleSnapshot,
  type PwaRouteIdentity,
  type PwaStyleProperty,
  resolvedPwaAfterStyle,
  stablePwaIdentityKey,
} from './pwaDesignDraft'
import { PwaDesignSessionModel } from './pwaDesignSessionModel'
import { sourceSavedEvidenceFromDraft, type PwaBridgeVerificationSnapshot, type PwaVerificationState } from './pwaVerificationModel'
import { usePwaSourceVerification } from './usePwaSourceVerification'
import type { SourcePreviewNode } from './types'

const BRIDGE_SOURCE = 'elon-pwa-design-bridge'
const PARENT_SOURCE = 'elon-pc-ui-tuner'
const PROTOCOL_VERSION = 1

export interface PwaSelection {
  identity: PwaElementIdentity
  rect: { left: number; top: number; width: number; height: number }
  originalStyle: PwaOriginalStyleSnapshot
  domContext: PwaDomContextNode[]
  sourceBinding?: PwaExplicitStyleBinding
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
  syncState: PwaVerificationState
  reloadKey: number
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
  retryVerification: () => Promise<void>
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
  const [reloadKey, setReloadKey] = useState(0)
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

  const reloadSource = useCallback(() => {
    modeRef.current = 'interact'
    setModeState('interact')
    setReady(false)
    setSelection(null)
    setMappedNodeKey(null)
    setUnboundLabel('')
    setReloadKey((value) => value + 1)
  }, [])

  const restoreDraft = useCallback(() => {
    const current = model.draft
    if (current) syncDraft(current)
    setSaveLabel('真实验证未通过，临时草稿已恢复，可修改后重试')
  }, [model, syncDraft])

  const clearVerifiedDraft = useCallback(() => {
    const current = model.draft
    if (!current) return
    removePwaDesignDraft(current)
    const { draft: cleared } = model.restore(current.project, { ...current.route, viewport: current.viewport })
    setDraft(cleared)
    setSelection(null)
    setMappedNodeKey(null)
    setHistoryVersion((value) => value + 1)
    setSaveLabel('真实源码与构建已验证，临时草稿已清除')
  }, [model])

  const verification = usePwaSourceVerification({ post, reloadSource, restoreDraft, clearVerifiedDraft })

  const applyDraftState = useCallback((value: PwaDesignDraft, sync = true) => {
    setDraft(value)
    if (sync) syncDraft(value)
    if (Object.keys(value.elements).length) {
      setSaveLabel(`已自动保存 · r${value.revision}`)
    } else {
      setSaveLabel('本页暂无样式草稿')
    }
    setHistoryVersion((version) => version + 1)
    verification.markLive('草稿已变更，当前仅为临时实时预览')
  }, [syncDraft, verification.markLive])

  useEffect(() => () => model.dispose(), [model])

  useEffect(() => listenForFitRunCodexSettled((detail) => {
    if (!syncTaskIdRef.current || detail.taskId !== syncTaskIdRef.current) return
    if (detail.succeeded) {
      verification.markSourceSaved(undefined, 'AI 已完成未绑定/结构修改；需刷新源码绑定并重新取得 changed files 后验证')
      setSaveLabel('AI 写回任务已结束，但尚未通过真实构建与画面验证')
    } else {
      verification.fail('跨端 Codex 写回失败；草稿已保留，可修正后重试')
    }
    syncTaskIdRef.current = ''
  }), [verification.fail, verification.markSourceSaved])

  const bridgeContextRef = useRef({ model, onSelect, post, project, root, syncDraft, verification })
  bridgeContextRef.current = { model, onSelect, post, project, root, syncDraft, verification }

  useEffect(() => {
    const receive = (event: MessageEvent) => {
      if (event.origin !== window.location.origin || event.source !== iframeRef.current?.contentWindow) return
      const message = event.data as {
        source?: string
        protocolVersion?: number
        type?: string
        payload?: Partial<PwaRouteState> & Partial<PwaBridgeVerificationSnapshot> & { node?: PwaSelection }
      }
      if (message.source !== BRIDGE_SOURCE || message.protocolVersion !== PROTOCOL_VERSION) return
      const context = bridgeContextRef.current
      if (message.type === 'ready') {
        setReady(true)
        const token = getAuthToken()
        if (token) context.post('set-session-auth', { token })
        context.post('set-mode', { mode: modeRef.current })
        if (!context.verification.onIframeReady() && context.model.draft) context.syncDraft(context.model.draft)
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
          context.verification.markLive()
          context.syncDraft(restored)
        }
        return
      }
      if (message.type === 'source-verification' && message.payload?.requestId) {
        context.verification.handleSnapshot(message.payload as PwaBridgeVerificationSnapshot)
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
        binding: pwaSourceBinding({ ...selection.identity, key: stableKey }, root, selection.sourceBinding),
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
    setMode('interact')
    verification.markLive(writebackPlan.requiresCodex
      ? '正在保存草稿；确定性绑定先写回，未绑定属性随后交给 AI…'
      : '正在保存草稿并执行确定性源码写回…')
    const androidResult = await applyDeterministicAndroidWriteback({
      draft: current,
      root,
      projectRoot: current.project.workspaceIdentity,
      sourceRevision: current.project.sourceRevision,
    })
    let deterministicResult: PwaCrossPlatformWritebackResult = {
      android: androidResult,
      pwa: { applied: 0, changedFiles: [], sourceRevisions: {}, completed: [] },
    }
    let latest = recordDeterministicWriteback(current, deterministicResult)
    if (androidResult.applied) {
      latest = { ...latest, project: { ...latest.project, sourceRevision: androidResult.sourceRevision } }
    }
    if (androidResult.error) {
      if (latest !== current) applyDraftState(model.replace(latest), false)
      verification.fail(`APK 确定性写回已停止：${androidResult.error}。源码冲突不会交给 AI 静默覆盖。`)
      return
    }
    const pwaResult = await applyDeterministicPwaWriteback({
      draft: latest,
      root,
      projectRoot: latest.project.workspaceIdentity,
    })
    deterministicResult = { android: androidResult, pwa: pwaResult }
    latest = recordDeterministicWriteback(latest, { ...deterministicResult, android: { ...androidResult, completed: [] } })
    if (latest !== current) applyDraftState(model.replace(latest), false)
    if (pwaResult.error) {
      verification.fail(`PWA 确定性写回已停止：${pwaResult.error}。请刷新源码绑定后重试。`)
      return
    }
    const plan = planPwaDesignWriteback(latest, root)
    const evidence = sourceSavedEvidenceFromDraft(latest, `pwa-source-${Date.now()}`) ?? undefined
    if (!plan.requiresCodex) {
      verification.markSourceSaved(evidence, `源码已保存：APK ${androidResult.applied} 个节点，PWA ${pwaResult.applied} 个绑定；尚未验证`)
      setSaveLabel('源码已保存，正在准备真实构建与画面验证')
      await verification.start(evidence)
      return
    }
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
        workspacePath: latest.project.workspaceIdentity,
        contextPack,
        reason: plan.codexReasons.join('；'),
      })
      syncTaskIdRef.current = taskId
      verification.markSourceSaved(undefined, androidResult.applied || pwaResult.applied
        ? `确定性部分已保存 ${androidResult.applied + pwaResult.applied} 个绑定；AI 只补未绑定属性或结构修改`
        : '已进入现有 AI 会话，只处理未绑定属性或结构修改', taskId)
    } catch (error) {
      verification.fail(error instanceof Error ? error.message : '跨端同步任务启动失败')
    }
  }, [applyDraftState, model, root, selection, setMode, verification.fail, verification.markLive, verification.markSourceSaved, verification.start])

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
    verification.markLive('已手动重载真实 PWA；保存的草稿会在页面连接后恢复')
    reloadSource()
  }, [reloadSource, verification.markLive])

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
    syncState: verification.state,
    reloadKey,
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
    retryVerification: verification.retry,
  }
}
