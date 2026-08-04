import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  captureTauriHost,
  beginDesignWriteback,
  captureDesignSession,
  createDesignDraft,
  getDesignDraft,
  getDesignSurface,
  listDesignSessions,
  listDesignDrafts,
  listDesignTargets,
  loadDesignPixel,
  loadTauriNativePixel,
  openDesignSession,
  prepareTauriRuntime,
  stopTauriRuntime,
  undoDesignDraft,
  updateDesignDraft,
} from './designSessionApi'
import type {
  DesignPlatform,
  DesignDraft,
  DesignDraftSummary,
  DesignSessionIdentity,
  DesignSessionSummary,
  DesignSurface,
  DesignTarget,
  DesignViewport,
  SemanticUiNode,
  DesignWritebackReceipt,
} from './types'

const DEFAULT_VIEWPORT: Record<DesignPlatform, DesignViewport> = {
  web: { width: 1280, height: 800, deviceScaleFactor: 1 },
  pwa: { width: 390, height: 844, deviceScaleFactor: 1 },
  tauri: { width: 1280, height: 800, deviceScaleFactor: 1 },
  android: { width: 390, height: 844, deviceScaleFactor: 1 },
}

export function useHeadlessDesignSession(active: boolean, projectRoot: string) {
  const [targets, setTargets] = useState<DesignTarget[]>([])
  const [sessions, setSessions] = useState<DesignSessionSummary[]>([])
  const [platform, setPlatform] = useState<DesignPlatform>('web')
  const [route, setRoute] = useState('/')
  const [url, setUrl] = useState('')
  const [viewport, setViewport] = useState<DesignViewport>(DEFAULT_VIEWPORT.web)
  const [session, setSession] = useState<DesignSessionIdentity | null>(null)
  const [surface, setSurface] = useState<DesignSurface | null>(null)
  const [selectedNode, setSelectedNode] = useState<SemanticUiNode | null>(null)
  const [drafts, setDrafts] = useState<DesignDraftSummary[]>([])
  const [designDraft, setDesignDraft] = useState<DesignDraft | null>(null)
  const [writebackReceipt, setWritebackReceipt] = useState<DesignWritebackReceipt | null>(null)
  const [query, setQuery] = useState('')
  const [pixelUrl, setPixelUrl] = useState('')
  const [nativePixelUrl, setNativePixelUrl] = useState('')
  const [tauriRuntimeStatus, setTauriRuntimeStatus] = useState('')
  const [busy, setBusy] = useState(false)
  const [status, setStatus] = useState('')
  const [error, setError] = useState('')
  const pixelUrlRef = useRef('')
  const nativePixelUrlRef = useRef('')
  const selectedNodeRef = useRef<SemanticUiNode | null>(null)

  const installPixel = useCallback(async (designSessionId: string) => {
    const blob = await loadDesignPixel(projectRoot, designSessionId)
    const next = URL.createObjectURL(blob)
    if (pixelUrlRef.current) URL.revokeObjectURL(pixelUrlRef.current)
    pixelUrlRef.current = next
    setPixelUrl(next)
  }, [projectRoot])

  const clearPixel = useCallback(() => {
    if (pixelUrlRef.current) URL.revokeObjectURL(pixelUrlRef.current)
    pixelUrlRef.current = ''
    setPixelUrl('')
  }, [])

  const installNativePixel = useCallback(async (designSessionId: string) => {
    const blob = await loadTauriNativePixel(projectRoot, designSessionId)
    const next = URL.createObjectURL(blob)
    if (nativePixelUrlRef.current) URL.revokeObjectURL(nativePixelUrlRef.current)
    nativePixelUrlRef.current = next
    setNativePixelUrl(next)
  }, [projectRoot])

  const clearNativePixel = useCallback(() => {
    if (nativePixelUrlRef.current) URL.revokeObjectURL(nativePixelUrlRef.current)
    nativePixelUrlRef.current = ''
    setNativePixelUrl('')
  }, [])

  const refreshDrafts = useCallback(async (designSessionId: string, selector?: string) => {
    const listed = await listDesignDrafts(projectRoot, designSessionId)
    setDrafts(listed.drafts)
    const match = listed.drafts.find((item) => item.selector === selector)
    if (!match) {
      setDesignDraft(null)
      setWritebackReceipt(null)
      return null
    }
    const detail = await getDesignDraft(projectRoot, match.draftId)
    setDesignDraft(detail.draft)
    return detail.draft
  }, [projectRoot])

  const readSurface = useCallback(async (designSessionId: string, search = '') => {
    const next = await getDesignSurface({ projectRoot, designSessionId, query: search, limit: 80 })
    setSurface(next)
    const chosen = (
      next.nodes.find((node) => node.selector === selectedNodeRef.current?.selector)
      ?? next.nodes.find((node) => node.interactive)
      ?? next.nodes[0]
      ?? null
    )
    selectedNodeRef.current = chosen
    setSelectedNode(chosen)
    if (next.pixels?.path) await installPixel(designSessionId)
    else clearPixel()
    if (next.nativeHost?.artifact.path) await installNativePixel(designSessionId)
    else clearNativePixel()
    await refreshDrafts(designSessionId, chosen?.selector)
    return next
  }, [clearNativePixel, clearPixel, installNativePixel, installPixel, projectRoot, refreshDrafts])

  const load = useCallback(async (preferredDesignSessionId?: string) => {
    if (!projectRoot) return
    setBusy(true)
    setError('')
    try {
      const [targetResult, sessionResult] = await Promise.all([
        listDesignTargets(projectRoot),
        listDesignSessions(projectRoot),
      ])
      setTargets(targetResult.targets)
      setSessions(sessionResult.sessions)
      const preferred = sessionResult.sessions.find((item) => (
        item.designSessionId === preferredDesignSessionId
      )) ?? sessionResult.sessions[0]
      const preferredPlatform = preferred?.platform
        ?? targetResult.targets[0]?.platform
      if (preferredPlatform) {
        setPlatform(preferredPlatform)
        setViewport(preferred?.viewport ?? DEFAULT_VIEWPORT[preferredPlatform])
      }
      if (preferred) {
        setSession(preferred)
        setRoute(preferred.route)
        setUrl(preferred.url ?? '')
        await readSurface(preferred.designSessionId)
      } else {
        setSession(null)
        setSurface(null)
        clearPixel()
        clearNativePixel()
      }
      setStatus(sessionResult.invalidRecordCount
        ? `已忽略 ${sessionResult.invalidRecordCount} 个损坏的本地会话记录`
        : '后台设计数据面已连接')
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '后台设计数据面连接失败')
    } finally {
      setBusy(false)
    }
  }, [clearNativePixel, clearPixel, projectRoot, readSurface])

  useEffect(() => {
    if (active) void load()
  }, [active, load])

  useEffect(() => () => {
    if (pixelUrlRef.current) URL.revokeObjectURL(pixelUrlRef.current)
    if (nativePixelUrlRef.current) URL.revokeObjectURL(nativePixelUrlRef.current)
  }, [])

  const selectPlatform = useCallback((next: DesignPlatform) => {
    setPlatform(next)
    setViewport(DEFAULT_VIEWPORT[next])
    const recent = sessions.find((item) => item.platform === next)
    if (recent) {
      setSession(recent)
      setRoute(recent.route)
      setUrl(recent.url ?? '')
      void readSurface(recent.designSessionId).catch((reason) => {
        setError(reason instanceof Error ? reason.message : '设计会话读取失败')
      })
      return
    }
    setSession(null)
    setSurface(null)
    selectedNodeRef.current = null
    setSelectedNode(null)
    clearPixel()
    clearNativePixel()
    setTauriRuntimeStatus('')
  }, [clearNativePixel, clearPixel, readSurface, sessions])

  const selectSession = useCallback(async (next: DesignSessionSummary) => {
    setSession(next)
    setPlatform(next.platform)
    setRoute(next.route)
    setUrl(next.url ?? '')
    setViewport(next.viewport)
    setBusy(true)
    setError('')
    try {
      await readSurface(next.designSessionId, query)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计会话读取失败')
    } finally {
      setBusy(false)
    }
  }, [query, readSurface])

  const prepareIntentTarget = useCallback(async (nextPlatform: DesignPlatform, nextRoute: string) => {
    const normalizedRoute = normalizeRoute(nextRoute)
    const reusable = sessions.find((item) => item.platform === nextPlatform && item.route === normalizedRoute)
    setPlatform(nextPlatform)
    setViewport(reusable?.viewport ?? DEFAULT_VIEWPORT[nextPlatform])
    setRoute(normalizedRoute)
    setUrl(reusable?.url ?? '')
    if (reusable) {
      setSession(reusable)
      await readSurface(reusable.designSessionId, query)
      return
    }
    setSession(null)
    setSurface(null)
    selectedNodeRef.current = null
    setSelectedNode(null)
    setDesignDraft(null)
    setWritebackReceipt(null)
    clearPixel()
    clearNativePixel()
    setTauriRuntimeStatus('')
  }, [clearNativePixel, clearPixel, query, readSurface, sessions])

  const open = useCallback(async () => {
    if (!projectRoot) throw new Error('当前项目缺少本机工作区路径')
    const next = await openDesignSession({
      projectRoot,
      platform,
      route: normalizeRoute(route),
      url: platform === 'android' ? undefined : url.trim() || undefined,
      viewport,
    })
    setSession(next)
    setRoute(next.route)
    setUrl(next.url ?? '')
    setSurface(null)
    selectedNodeRef.current = null
    setSelectedNode(null)
    clearPixel()
    clearNativePixel()
    return next
  }, [clearNativePixel, clearPixel, platform, projectRoot, route, url, viewport])

  const capture = useCallback(async (steps: Array<Record<string, unknown>> = []) => {
    setBusy(true)
    setError('')
    try {
      const normalizedRoute = normalizeRoute(route)
      const activeSession = session
        && session.platform === platform
        && session.route === normalizedRoute
        && (platform === 'android' || (session.url ?? '') === url.trim())
        ? session
        : await open()
      const result = await captureDesignSession({
        projectRoot,
        designSessionId: activeSession.designSessionId,
        capture: steps.length ? { steps } : undefined,
      })
      if (!result.ok && result.status !== 'PREPARATION_REQUIRED') {
        throw new Error(result.diagnostic
          ? `${result.diagnostic.code}：${result.diagnostic.nextStep}`
          : result.message || '后台页面捕获失败')
      }
      await readSurface(activeSession.designSessionId, query)
      const listed = await listDesignSessions(projectRoot)
      setSessions(listed.sessions)
      setStatus(result.status === 'PREPARATION_REQUIRED'
        ? result.message || 'Android 需要连接 Live Runtime'
        : steps.length
          ? '已在隔离后台页面重放交互，并更新 UI 树与截图哈希'
          : '已更新 PNG、语义 UI 树与截图哈希')
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '后台页面捕获失败')
    } finally {
      setBusy(false)
    }
  }, [open, platform, projectRoot, query, readSurface, route, session, url])

  const interactSelected = useCallback(async () => {
    if (!selectedNode?.interactive) return
    await capture([{ action: 'click', selector: selectedNode.selector }])
  }, [capture, selectedNode])

  const prepareTauri = useCallback(async (restart = false) => {
    if (!session || platform !== 'tauri') return
    setBusy(true)
    setError('')
    try {
      const result = await prepareTauriRuntime({ projectRoot, designSessionId: session.designSessionId, restart })
      setTauriRuntimeStatus(result.status)
      setStatus(result.status === 'READY'
        ? `Tauri 原生窗口已就绪：${result.runtime?.window?.title || '已发现窗口'}`
        : result.status === 'STARTING'
          ? 'Tauri CLI 已在后台启动；请继续轮询窗口状态'
          : result.next || result.status)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Tauri Runtime 准备失败')
    } finally {
      setBusy(false)
    }
  }, [platform, projectRoot, session])

  const captureTauri = useCallback(async () => {
    if (!session || platform !== 'tauri') return
    setBusy(true)
    setError('')
    try {
      await captureTauriHost({ projectRoot, designSessionId: session.designSessionId })
      await readSurface(session.designSessionId, query)
      setTauriRuntimeStatus('CAPTURED')
      setStatus('已捕获 Tauri 原生窗口 PNG、边界、PID 与 SHA-256')
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Tauri 原生窗口捕获失败')
    } finally {
      setBusy(false)
    }
  }, [platform, projectRoot, query, readSurface, session])

  const stopTauri = useCallback(async () => {
    if (!session || platform !== 'tauri') return
    setBusy(true)
    setError('')
    try {
      const result = await stopTauriRuntime({ projectRoot, designSessionId: session.designSessionId })
      setTauriRuntimeStatus(result.status)
      setStatus(result.status === 'STOPPED' ? '已停止当前 designSession 启动的 Tauri 进程树' : '当前没有登记的 Tauri Runtime')
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Tauri Runtime 停止失败')
    } finally {
      setBusy(false)
    }
  }, [platform, projectRoot, session])

  const selectNode = useCallback((node: SemanticUiNode | null) => {
    selectedNodeRef.current = node
    setSelectedNode(node)
    if (!session) {
      setDesignDraft(null)
      return
    }
    void refreshDrafts(session.designSessionId, node?.selector).catch((reason) => {
      setError(reason instanceof Error ? reason.message : '设计草稿读取失败')
    })
  }, [refreshDrafts, session])

  const saveDraftPatch = useCallback(async (property: string, after: string) => {
    if (!session || !selectedNode) return
    const cleanProperty = property.trim()
    const cleanAfter = after.trim()
    if (!cleanProperty || !cleanAfter) return
    setBusy(true)
    setError('')
    try {
      const style = selectedNode.style as Record<string, string | undefined>
      const nextPatch = { property: cleanProperty, before: style[cleanProperty], after: cleanAfter }
      const nextOperation = { type: 'SET_STYLE' as const, property: cleanProperty, before: style[cleanProperty], after: cleanAfter }
      const result = designDraft && designDraft.selector === selectedNode.selector
        ? await updateDesignDraft({
            projectRoot,
            draftId: designDraft.draftId,
            expectedRevision: designDraft.revision,
            patches: [...designDraft.patches.filter((patch) => patch.property !== cleanProperty), nextPatch],
            operations: [
              ...(designDraft.operations ?? []).filter((operation) => (
                operation.type !== 'SET_STYLE' || operation.property !== cleanProperty
              )),
              nextOperation,
            ],
          })
        : await createDesignDraft({
            projectRoot,
            designSessionId: session.designSessionId,
            selector: selectedNode.selector,
            patches: [nextPatch],
            operations: [nextOperation],
            targetPlatforms: [platform],
          })
      setDesignDraft(result.draft)
      setWritebackReceipt(null)
      const listed = await listDesignDrafts(projectRoot, session.designSessionId)
      setDrafts(listed.drafts)
      setStatus(`设计草稿已保存为 r${result.draft.revision}；尚未冒充源码写回`)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计草稿保存失败')
    } finally {
      setBusy(false)
    }
  }, [designDraft, platform, projectRoot, selectedNode, session])

  const undoDraft = useCallback(async () => {
    if (!designDraft) return
    setBusy(true)
    setError('')
    try {
      const result = await undoDesignDraft({ projectRoot, draftId: designDraft.draftId, expectedRevision: designDraft.revision })
      setDesignDraft(result.draft)
      setWritebackReceipt(null)
      setStatus(`已撤销最近草稿修改；当前 revision r${result.draft.revision}`)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计草稿撤销失败')
    } finally {
      setBusy(false)
    }
  }, [designDraft, projectRoot])

  const beginDraftWriteback = useCallback(async (writebackPlanId: string) => {
    if (!designDraft) return
    setBusy(true)
    setError('')
    try {
      const result = await beginDesignWriteback({
        projectRoot,
        draftId: designDraft.draftId,
        expectedRevision: designDraft.revision,
        writebackPlanId,
      })
      setDesignDraft(result.draft)
      setWritebackReceipt(result.receipt)
      setStatus(`已固定 ${result.receipt.sourceRevisionBefore.slice(0, 12)} 的写回基线；等待 AI 修改源码和提交分平台证据`)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计写回开始失败')
    } finally {
      setBusy(false)
    }
  }, [designDraft, projectRoot])

  const search = useCallback(async () => {
    if (!session) return
    setBusy(true)
    setError('')
    try {
      await readSurface(session.designSessionId, query)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'UI 树查询失败')
    } finally {
      setBusy(false)
    }
  }, [query, readSurface, session])

  const target = useMemo(
    () => targets.find((item) => item.platform === platform) ?? null,
    [platform, targets],
  )

  return {
    targets, sessions, target, platform, route, url, viewport, session, surface,
    selectedNode, drafts, designDraft, writebackReceipt, query, pixelUrl, nativePixelUrl,
    tauriRuntimeStatus, busy, status, error,
    setRoute, setUrl, setViewport, setQuery, setSelectedNode: selectNode,
    selectPlatform, selectSession, prepareIntentTarget, capture, interactSelected, prepareTauri, captureTauri,
    stopTauri, saveDraftPatch, undoDraft, beginDraftWriteback, search, reload: load,
  }
}

function normalizeRoute(value: string) {
  const clean = value.trim()
  if (!clean) return '/'
  return clean.startsWith('/') ? clean : `/${clean}`
}
