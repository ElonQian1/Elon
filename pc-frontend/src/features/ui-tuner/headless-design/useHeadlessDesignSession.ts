import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  captureDesignSession,
  getDesignSurface,
  listDesignSessions,
  listDesignTargets,
  loadDesignPixel,
  openDesignSession,
} from './designSessionApi'
import type {
  DesignPlatform,
  DesignSessionIdentity,
  DesignSessionSummary,
  DesignSurface,
  DesignTarget,
  DesignViewport,
  SemanticUiNode,
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
  const [query, setQuery] = useState('')
  const [pixelUrl, setPixelUrl] = useState('')
  const [busy, setBusy] = useState(false)
  const [status, setStatus] = useState('')
  const [error, setError] = useState('')
  const pixelUrlRef = useRef('')

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

  const readSurface = useCallback(async (designSessionId: string, search = '') => {
    const next = await getDesignSurface({ projectRoot, designSessionId, query: search, limit: 80 })
    setSurface(next)
    setSelectedNode((current) => (
      next.nodes.find((node) => node.selector === current?.selector)
      ?? next.nodes.find((node) => node.interactive)
      ?? next.nodes[0]
      ?? null
    ))
    if (next.pixels?.path) await installPixel(designSessionId)
    else clearPixel()
    return next
  }, [clearPixel, installPixel, projectRoot])

  const load = useCallback(async () => {
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
      const preferred = sessionResult.sessions[0]
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
      }
      setStatus(sessionResult.invalidRecordCount
        ? `已忽略 ${sessionResult.invalidRecordCount} 个损坏的本地会话记录`
        : '后台设计数据面已连接')
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '后台设计数据面连接失败')
    } finally {
      setBusy(false)
    }
  }, [clearPixel, projectRoot, readSurface])

  useEffect(() => {
    if (active) void load()
  }, [active, load])

  useEffect(() => () => {
    if (pixelUrlRef.current) URL.revokeObjectURL(pixelUrlRef.current)
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
    setSelectedNode(null)
    clearPixel()
  }, [clearPixel, readSurface, sessions])

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
    setSelectedNode(null)
    clearPixel()
    return next
  }, [clearPixel, platform, projectRoot, route, url, viewport])

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
    selectedNode, query, pixelUrl, busy, status, error,
    setRoute, setUrl, setViewport, setQuery, setSelectedNode,
    selectPlatform, selectSession, capture, interactSelected, search, reload: load,
  }
}

function normalizeRoute(value: string) {
  const clean = value.trim()
  if (!clean) return '/'
  return clean.startsWith('/') ? clean : `/${clean}`
}
