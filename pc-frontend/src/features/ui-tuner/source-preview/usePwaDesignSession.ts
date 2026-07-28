import { useCallback, useEffect, useMemo, useRef, useState, type MutableRefObject } from 'react'
import { getAuthToken } from '../../../api/client'
import {
  listenForFitRunCodexSettled,
  readFitRunCodexLaunchByRun,
  readFitRunCodexSettlement,
  requestFitRunCodexTracking,
  requestCodexForFitRun,
  type FitRunCodexSettledDetail,
} from '../fit-run/fitRunEvents'
import { buildPwaDesignContextPack } from './pwaDesignContext'
import { planPwaDesignWriteback } from './pwaDesignWriteback'
import {
  executeCrossPlatformDeterministicWriteback,
  mergedPlatformFiles,
} from './crossPlatformWritebackOrchestrator'
import {
  type CrossPlatformWritebackReceipt,
  type PlatformReceiptUpdate,
} from './crossPlatformWritebackReceipt'
import { matchPwaSourceNode, pwaSourceBinding } from './pwaNodeMapping'
import {
  beginPwaDraftRestore,
  consumePwaDraftAppliedAck,
  pwaDraftRestoreLabel,
  type PwaDraftAppliedAck,
  type PwaDraftRestoreState,
} from './pwaDraftRestoreAck'
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
import { resolvePwaStyleBinding } from './sourcePreviewApi'
import {
  sourceSavedEvidenceFromAiReceipt,
  sourceSavedEvidenceFromDraft,
  type PwaBridgeVerificationSnapshot,
  type PwaVerificationState,
} from './pwaVerificationModel'
import {
  useCrossPlatformWritebackReceipt,
  type AndroidWritebackVerification,
} from './useCrossPlatformWritebackReceipt'
import { usePwaSourceVerification } from './usePwaSourceVerification'
import { mergePwaRouteState, type PwaRouteState } from './pwaRuntimeViewport'
import type { PwaBridgeHealth } from './pwaBridgeHealth'
import type { SourcePreviewNode } from './types'

export type { AndroidWritebackVerification } from './useCrossPlatformWritebackReceipt'
export type { PwaRouteState } from './pwaRuntimeViewport'

const BRIDGE_SOURCE = 'elon-pwa-design-bridge'
const PARENT_SOURCE = 'elon-pc-ui-tuner'
const PROTOCOL_VERSION = 1

export interface PwaSelection {
  identity: PwaElementIdentity
  rect: { left: number; top: number; width: number; height: number }
  originalStyle: PwaOriginalStyleSnapshot
  domContext: PwaDomContextNode[]
  sourceSelectors?: string[]
  sourceBinding?: PwaExplicitStyleBinding
}

interface UsePwaDesignSessionOptions {
  projectId: string
  workspaceIdentity: string
  sourceRevision: string
  root: SourcePreviewNode | null
  onSelect: (key: string) => void
  runtimeUrl: string
  androidVerification?: AndroidWritebackVerification
}

export interface PwaDesignSession {
  iframeRef: MutableRefObject<HTMLIFrameElement | null>
  ready: boolean
  mode: 'select' | 'interact'
  selection: PwaSelection | null
  route: PwaRouteState | null
  bridgeHealth: PwaBridgeHealth | null
  draft: PwaDesignDraft | null
  mappedNodeKey: string | null
  unboundLabel: string
  canUndo: boolean
  canRedo: boolean
  saveLabel: string
  syncState: PwaVerificationState
  writebackReceipt: CrossPlatformWritebackReceipt | null
  reloadKey: number
  writebackPlan: ReturnType<typeof planPwaDesignWriteback>
  setMode: (mode: 'select' | 'interact') => void
  updateStyle: (property: PwaStyleProperty, value: string) => void
  updateStyles: (label: string, styles: Partial<Record<PwaStyleProperty, string>>) => void
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
    identity: { ...element.identity, key: stablePwaIdentityKey(element.identity) },
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
  runtimeUrl,
  androidVerification,
}: UsePwaDesignSessionOptions): PwaDesignSession {
  const iframeRef = useRef<HTMLIFrameElement>(null)
  const [ready, setReady] = useState(false)
  const [modeState, setModeState] = useState<'select' | 'interact'>('interact')
  const [selection, setSelection] = useState<PwaSelection | null>(null)
  const [route, setRoute] = useState<PwaRouteState | null>(null)
  const [bridgeHealth, setBridgeHealth] = useState<PwaBridgeHealth | null>(null)
  const [draft, setDraft] = useState<PwaDesignDraft | null>(null)
  const [mappedNodeKey, setMappedNodeKey] = useState<string | null>(null)
  const [unboundLabel, setUnboundLabel] = useState('')
  const [historyVersion, setHistoryVersion] = useState(0)
  const [saveLabel, setSaveLabel] = useState('等待进入真实页面')
  const [reloadKey, setReloadKey] = useState(0)
  const sourceSelectorKey = selection?.sourceSelectors?.join('\n') ?? ''
  const routeRef = useRef<PwaRouteState | null>(null)
  const modeRef = useRef(modeState)
  const syncTaskIdRef = useRef('')
  const draftRestoreRef = useRef<PwaDraftRestoreState | null>(null)
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
    const requestedCount = value ? Object.keys(value.elements).length : 0
    if (value && requestedCount) {
      const pending = beginPwaDraftRestore(bridgeDraftKey(value), value.revision, requestedCount)
      draftRestoreRef.current = pending
      setSaveLabel(pwaDraftRestoreLabel(pending))
    } else {
      draftRestoreRef.current = null
    }
    post('apply-draft', value ? {
      draftKey: bridgeDraftKey(value),
      revision: value.revision,
      elements: bridgeElements(value),
    } : { draftKey: '', revision: 0, elements: [] })
  }, [post])

  const reloadSource = useCallback(() => {
    draftRestoreRef.current = null
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
    if (current) {
      setSaveLabel('真实验证未通过，正在恢复临时草稿')
      syncDraft(current)
    }
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

  const verification = usePwaSourceVerification({ post, reloadSource, restoreDraft, runtimeUrl })
  const receiptFlow = useCrossPlatformWritebackReceipt({
    androidVerification,
    pwaState: verification.state,
    onComplete: clearVerifiedDraft,
    onError: verification.fail,
    onStatus: setSaveLabel,
  })
  const writebackReceipt = receiptFlow.receipt
  const writebackReceiptRef = receiptFlow.receiptRef
  const rememberReceipt = receiptFlow.remember
  const updateReceipt = receiptFlow.update
  const verifyAndroidReceipt = receiptFlow.verifyAndroid

  const handleCodexSettlement = useCallback((detail: FitRunCodexSettledDetail) => {
    if (!syncTaskIdRef.current || detail.taskId !== syncTaskIdRef.current) return
    if (detail.succeeded) {
      const currentReceipt = writebackReceiptRef.current
      const currentDraft = model.draft
      const aiEvidence = detail.receipt && currentDraft
        ? sourceSavedEvidenceFromAiReceipt(currentDraft, detail.receipt, `pwa-ai-source-${Date.now()}`)
        : null
      if (!detail.receipt) {
        verification.fail('AI 任务已结束但缺少机器回执；没有 changedFiles、sourceHash/sourceRevision 与分端结果，不能显示源码已保存')
        setSaveLabel('AI 缺少机器回执；草稿与现有分端状态已保留')
      } else if (!currentReceipt) {
        if (aiEvidence) {
          verification.markSourceSaved(aiEvidence, 'AI 已写回 PWA 源码；正在用真实源码重载验证，不再停留在 AI 写作状态', detail.taskId)
          void verification.start(aiEvidence)
          setSaveLabel('AI 已返回 PWA 机器回执；正在执行真实 PWA 构建与画面验证')
        } else {
          verification.fail('AI 回执没有可验证的 PWA changedFiles/sourceRevision 或当前草稿缺少目标样式；草稿已保留')
          setSaveLabel('AI 回执不能转换为 PWA 验证证据；请检查回执或重试')
        }
      } else if (detail.receipt.sourceRevisionBefore !== currentReceipt.sourceRevision) {
        verification.fail('AI 回执的 sourceRevisionBefore 与确定性写回 checkpoint 不一致；已拒绝陈旧回执')
        setSaveLabel('AI 回执 revision 已过期；请刷新源码后重试')
      } else {
        const updates: Partial<Record<'pwa' | 'apk', PlatformReceiptUpdate>> = {}
        for (const platform of detail.receipt.targetPlatforms) {
          const result = detail.receipt.platformResults[platform]
          if (!result) continue
          const previous = currentReceipt.platformResults[platform]
          updates[platform] = {
            status: result.status,
            method: previous.changedFiles.length ? 'MIXED' : 'CODEX',
            changedFiles: mergedPlatformFiles(currentReceipt, platform, result.changedFiles),
            sourceRevisions: {
              ...previous.sourceRevisions,
              ...Object.fromEntries(result.changedFiles.map((file) => [file, result.sourceRevision])),
            },
            expectedSourceRevisionBefore: detail.receipt.sourceRevisionBefore,
            aiTaskId: detail.taskId,
            error: result.error,
          }
        }
        void updateReceipt(updates).then((receipt) => {
          const pwaStatus = receipt.platformResults.pwa.status
          if (aiEvidence && ['SAVED', 'BUILD_VERIFYING'].includes(pwaStatus)) {
            verification.markSourceSaved(aiEvidence, 'AI 机器回执已复核；正在执行 PWA 真实构建与画面验证', detail.taskId)
            void updateReceipt({
              pwa: {
                status: 'BUILD_VERIFYING',
                method: receipt.platformResults.pwa.method,
                changedFiles: receipt.platformResults.pwa.changedFiles,
                sourceRevisions: receipt.platformResults.pwa.sourceRevisions,
              },
            }).catch((error) => {
              verification.fail(error instanceof Error ? error.message : 'PWA AI 回执进入构建验证失败')
            })
            void verification.start(aiEvidence)
          } else {
            verification.markSourceSaved(undefined, 'AI 机器回执已复核；请刷新局部绑定后执行分端构建验证')
          }
          if (receipt.platformResults.apk.status === 'SAVED') void verifyAndroidReceipt(receipt)
          setSaveLabel(receipt.status === 'PARTIAL'
            ? 'AI 已部分写回；单端失败已保留在回执'
            : 'AI 源码回执已保存；等待 PWA/APK 独立构建证据')
        }).catch((error) => {
          verification.fail(error instanceof Error ? error.message : 'AI 机器回执复核失败')
        })
      }
    } else {
      verification.fail(`跨端 Codex 写回失败：${detail.error || '任务未完成'}；草稿已保留，可直接重试`)
    }
    syncTaskIdRef.current = ''
  }, [model, updateReceipt, verification.fail, verification.markSourceSaved, verification.start, verifyAndroidReceipt, writebackReceiptRef])

  const applyDraftState = useCallback((value: PwaDesignDraft, sync = true) => {
    setDraft(value)
    if (sync) syncDraft(value)
    if (!sync && Object.keys(value.elements).length) {
      setSaveLabel(`已自动保存 · r${value.revision}`)
    } else if (!Object.keys(value.elements).length) {
      setSaveLabel('本页暂无样式草稿')
    }
    setHistoryVersion((version) => version + 1)
    verification.markLive('草稿已变更，当前仅为临时实时预览')
  }, [syncDraft, verification.markLive])

  useEffect(() => () => model.dispose(), [model])

  useEffect(() => listenForFitRunCodexSettled(handleCodexSettlement), [handleCodexSettlement])

  useEffect(() => {
    if (!draft || syncTaskIdRef.current || verification.state.phase !== 'LIVE_PREVIEW') return
    const runId = `pwa:${draft.project.id}:${draft.revision}`
    const launch = readFitRunCodexLaunchByRun(runId, 'PWA_DRAFT')
    if (!launch?.taskId) return
    syncTaskIdRef.current = launch.taskId
    requestFitRunCodexTracking({
      runId,
      handoffId: launch.handoffId,
      taskId: launch.taskId,
      handoffKind: 'PWA_DRAFT',
    })
    const settlement = readFitRunCodexSettlement(launch.taskId)
    if (settlement) {
      handleCodexSettlement(settlement)
      return
    }
    verification.markAiWriting(launch.taskId, '已恢复上次 PWA 草稿 AI 写回任务；等待 Codex CLI 机器回执后自动验证')
    setSaveLabel('已恢复 PWA 草稿 AI 写回任务；等待后台结算')
  }, [draft, handleCodexSettlement, verification.markAiWriting, verification.state.phase])

  useEffect(() => {
    if (!selection || selection.sourceBinding || !sourceSelectorKey || !workspaceIdentity) return
    const identityKey = stablePwaIdentityKey(selection.identity)
    let cancelled = false
    setSaveLabel('正在本机解析真实 PWA 样式源码…')
    void resolvePwaStyleBinding({
      projectRoot: workspaceIdentity,
      selectors: selection.sourceSelectors ?? [],
    }).then((result) => {
      if (cancelled) return
      if (!result.binding) {
        setSaveLabel(result.detail || '没有唯一源码规则，保存时由 AI 按需建立绑定')
        return
      }
      setSelection((current) => current && stablePwaIdentityKey(current.identity) === identityKey
        ? { ...current, sourceBinding: result.binding }
        : current)
      const currentDraft = model.draft
      if (currentDraft) {
        const found = draftEntry(currentDraft, selection.identity)
        if (found) {
          const elements = {
            ...currentDraft.elements,
            [found.key]: {
              ...found.element,
              binding: pwaSourceBinding(found.element.identity, root, result.binding),
              updatedAt: new Date().toISOString(),
            },
          }
          const next = model.replace({ ...currentDraft, elements, updatedAt: new Date().toISOString() })
          setDraft(next)
        }
      }
      setSaveLabel(`已自动绑定 PWA 样式源码 · ${result.binding.sourceFile}`)
    }).catch((error) => {
      if (!cancelled) setSaveLabel(`本地源码解析暂不可用：${error instanceof Error ? error.message : '稍后由 AI 补充绑定'}`)
    })
    return () => { cancelled = true }
  }, [model, root, selection?.identity.key, selection?.sourceBinding, sourceSelectorKey, workspaceIdentity])

  const bridgeContextRef = useRef({
    model,
    onSelect,
    post,
    project,
    resetReceipt: receiptFlow.reset,
    root,
    syncDraft,
    verification,
  })
  bridgeContextRef.current = {
    model,
    onSelect,
    post,
    project,
    resetReceipt: receiptFlow.reset,
    root,
    syncDraft,
    verification,
  }

  useEffect(() => {
    const receive = (event: MessageEvent) => {
      if (event.origin !== window.location.origin || event.source !== iframeRef.current?.contentWindow) return
      const message = event.data as {
        source?: string
        protocolVersion?: number
        type?: string
        payload?: Partial<PwaRouteState> & Partial<PwaBridgeHealth> & Partial<PwaBridgeVerificationSnapshot> & Partial<PwaDraftAppliedAck> & { node?: PwaSelection; mode?: string; message?: string }
      }
      if (message.source !== BRIDGE_SOURCE || message.protocolVersion !== PROTOCOL_VERSION) return
      const context = bridgeContextRef.current
      if (message.type === 'ready') {
        setReady(true)
        const token = getAuthToken()
        if (token) context.post('set-session-auth', { token })
        context.post('set-mode', { mode: modeRef.current })
        context.post('health-check', { reason: 'parent-ready' })
        if (!context.verification.onIframeReady() && context.model.draft) context.syncDraft(context.model.draft)
        return
      }
      if (message.type === 'route-changed' && message.payload?.path && message.payload.viewport) {
        const nextRoute = mergePwaRouteState(message.payload as PwaRouteState)
        const changed = !routeRef.current || routeKey(routeRef.current) !== routeKey(nextRoute)
        routeRef.current = nextRoute
        setRoute(nextRoute)
        if (changed) {
          context.resetReceipt()
          const { draft: restored, restored: didRestore } = context.model.restore(context.project, nextRoute)
          setDraft(restored)
          setHistoryVersion((value) => value + 1)
          setSelection(null)
          setMappedNodeKey(null)
          setUnboundLabel('')
          context.verification.markLive()
          if (didRestore && Object.keys(restored.elements).length) {
            setSaveLabel(`已找到本页草稿 · r${restored.revision}，正在恢复`)
            context.syncDraft(restored)
          } else {
            draftRestoreRef.current = null
            setSaveLabel('本页暂无样式草稿')
            context.syncDraft(restored)
          }
        }
        return
      }
      if (message.type === 'draft-applied' && message.payload) {
        const current = draftRestoreRef.current
        if (!current) return
        const next = consumePwaDraftAppliedAck(current, message.payload as Partial<PwaDraftAppliedAck>)
        if (next === current) return
        draftRestoreRef.current = next
        setSaveLabel(pwaDraftRestoreLabel(next))
        return
      }
      if (message.type === 'mode-changed' && message.payload?.mode) {
        const nextMode = message.payload.mode === 'select' ? 'select' : 'interact'
        modeRef.current = nextMode
        setModeState(nextMode)
        return
      }
      if (message.type === 'bridge-notice' && message.payload?.message) {
        setSaveLabel(String(message.payload.message))
        return
      }
      if (message.type === 'health' && message.payload) {
        setBridgeHealth(message.payload as PwaBridgeHealth)
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
    post('health-check', { reason: `parent-mode-${nextMode}` })
  }, [post])

  useEffect(() => {
    const cancelSelection = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || modeRef.current !== 'select') return
      modeRef.current = 'interact'
      setModeState('interact')
      post('set-mode', { mode: 'interact' })
      post('health-check', { reason: 'parent-escape' })
      setSaveLabel('已退出选择组件模式；页面恢复正常操作')
    }
    window.addEventListener('keydown', cancelSelection)
    return () => window.removeEventListener('keydown', cancelSelection)
  }, [post])

  const updateStyles = useCallback((label: string, styles: Partial<Record<PwaStyleProperty, string>>) => {
    const current = model.draft
    if (!current || !selection) return
    const stableKey = stablePwaIdentityKey(selection.identity)
    const found = draftEntry(current, selection.identity)
    const existing = found?.element
    const originalStyle = existing?.originalStyle ?? selection.originalStyle
    const styleDiff = { ...(existing?.styleDiff ?? {}) }
    for (const [property, input] of Object.entries(styles) as [PwaStyleProperty, string | undefined][]) {
      const originalValue = originalStyle.authored[property] || originalStyle.computed[property] || ''
      const value = String(input ?? '').trim()
      if (!value || value === originalValue) delete styleDiff[property]
      else styleDiff[property] = value
    }
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
    const next = model.update(`${stableKey}:${label}`, () => elements)
    if (next) applyDraftState(next)
  }, [applyDraftState, model, root, selection])

  const updateStyle = useCallback((property: PwaStyleProperty, input: string) => {
    updateStyles(property, { [property]: input })
  }, [updateStyles])

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
    const restoring = draftRestoreRef.current
    setSaveLabel(restoring && restoring.phase !== 'complete'
      ? pwaDraftRestoreLabel(restoring)
      : Object.keys(current.elements).length ? `草稿已保存 · r${current.revision}` : '本页暂无样式草稿')
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
    let deterministic
    try {
      deterministic = await executeCrossPlatformDeterministicWriteback({
        draft: current,
        root,
        onReceipt: rememberReceipt,
      })
    } catch (error) {
      verification.fail(error instanceof Error ? error.message : '无法建立机器回执，已停止源码写回')
      return
    }
    const { draft: latest, result: deterministicResult, receipt, plan } = deterministic
    rememberReceipt(receipt)
    if (latest !== current) applyDraftState(model.replace(latest), false)
    const evidence = sourceSavedEvidenceFromDraft(latest, `pwa-source-${Date.now()}`) ?? undefined
    const pwaSaved = receipt.platformResults.pwa.status === 'SAVED'
    const apkSaved = receipt.platformResults.apk.status === 'SAVED'
    if (deterministic.conflict) {
      setSaveLabel(`双端写回出现部分失败：PWA ${receipt.platformResults.pwa.status} · APK ${receipt.platformResults.apk.status}`)
      if (pwaSaved && evidence) {
        await updateReceipt({
          pwa: {
            status: 'BUILD_VERIFYING',
            method: receipt.platformResults.pwa.method,
            changedFiles: receipt.platformResults.pwa.changedFiles,
            sourceRevisions: receipt.platformResults.pwa.sourceRevisions,
          },
        })
        verification.markSourceSaved(evidence, 'PWA 已保存；另一端失败不阻断本端真实重载验证')
        await verification.start(evidence)
      } else {
        verification.fail('双端写回存在源码冲突；成功端状态已保留，失败端不会交给 AI 静默覆盖')
      }
      if (apkSaved) void verifyAndroidReceipt(receipt)
      return
    }
    if (!plan.requiresCodex) {
      if (pwaSaved && evidence) {
        await updateReceipt({
          pwa: {
            status: 'BUILD_VERIFYING',
            method: receipt.platformResults.pwa.method,
            changedFiles: receipt.platformResults.pwa.changedFiles,
            sourceRevisions: receipt.platformResults.pwa.sourceRevisions,
          },
        })
        verification.markSourceSaved(
          evidence,
          `源码已保存：APK ${deterministicResult.android.applied} 个节点，PWA ${deterministicResult.pwa.applied} 个绑定；正在分端验证`,
        )
        await verification.start(evidence)
      } else {
        verification.fail('PWA 写回缺少 changedFiles/sourceRevision，不能开始真实重载验证')
      }
      if (apkSaved) void verifyAndroidReceipt(receipt)
      setSaveLabel('双端源码回执已保存；PWA 与 APK 正在独立构建验证')
      return
    }
    const contextPack = buildPwaDesignContextPack({
      draft: latest,
      root,
      selection,
      plan,
      deterministicResult,
      runtimeCapture: verification.state.runtimeCapture,
      writebackReceipt: receipt,
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
      verification.markAiWriting(taskId, deterministicResult.android.applied || deterministicResult.pwa.applied
        ? `确定性部分已保存 ${deterministicResult.android.applied + deterministicResult.pwa.applied} 个绑定；AI 正在补未绑定属性或结构修改`
        : 'AI 正在处理未绑定属性或结构修改；草稿仍保留，当前不算源码已保存')
    } catch (error) {
      verification.fail(error instanceof Error ? error.message : '跨端同步任务启动失败')
    }
  }, [
    applyDraftState,
    model,
    rememberReceipt,
    root,
    selection,
    setMode,
    updateReceipt,
    verification.fail,
    verification.markAiWriting,
    verification.markLive,
    verification.markSourceSaved,
    verification.start,
    verifyAndroidReceipt,
    writebackPlan.requiresCodex,
  ])

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
    bridgeHealth,
    draft,
    mappedNodeKey,
    unboundLabel,
    canUndo: model.canUndo && historyVersion >= 0,
    canRedo: model.canRedo && historyVersion >= 0,
    saveLabel,
    syncState: verification.state,
    writebackReceipt,
    reloadKey,
    writebackPlan,
    setMode,
    updateStyle,
    updateStyles,
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
