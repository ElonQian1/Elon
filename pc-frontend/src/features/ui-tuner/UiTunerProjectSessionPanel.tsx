import { useEffect, useMemo, useState } from 'react'
import { GitBranch, Play, RefreshCw, TerminalSquare } from 'lucide-react'
import { useProjectStore } from '../conversation/useProjectStore'
import { channelAllowsAiStart, mergeProjectRecords } from '../conversation/conversationPageHelpers'
import { ensureLocalFullAccessGrant, type LocalNodeStatus } from '../conversation/localPcRuntime'
import type { RuntimeRoute } from '../conversation/runtimeRoutes'
import SidecarTerminalPanel from '../dev/SidecarTerminalPanel'
import { useModelStore } from '../models/useModelStore'
import { selectedAgentForRuntimeRoute } from '../models/routeModelPolicy'
import { localJson } from '../doctor/localApi'
import { useAuthStore } from '../../store/auth'
import { clean, safeNodeAdminUrl } from '../../lib/utils'
import type { UiTunerCodexContextPack } from './contextPack'
import {
  buildUiTunerProjectTaskContent,
  createUiTunerProjectSession,
  readUiTunerModuleMemory,
  readUiTunerProjectSessions,
  rememberUiTunerIntent,
  saveUiTunerProjectSession,
  updateUiTunerProjectSession,
  writeUiTunerModuleMemory,
  type UiTunerProjectSessionRecord,
} from './projectSessions'
import panelStyles from './UiTunerPanels.module.css'

interface UiTunerProjectSessionPanelProps {
  pack: UiTunerCodexContextPack
  intent: string
}

const CODEX_ROUTE: RuntimeRoute = 'route_a'

export function UiTunerProjectSessionPanel({ pack, intent }: UiTunerProjectSessionPanelProps) {
  const nodeAdminUrl = uiTunerNodeAdminUrl()
  const user = useAuthStore((state) => state.user)
  const selectedAgent = useModelStore((state) => state.selectedAgent)
  const modelOptions = useModelStore((state) => state.options)
  const {
    projects,
    projectsLoaded,
    activeProjectId,
    space,
    channels,
    messages,
    sendingMessage,
    loadProjects,
    selectProject,
    selectChannel,
    sendMessage,
  } = useProjectStore()
  const [localNode, setLocalNode] = useState<LocalNodeStatus | null>(null)
  const [localNodeError, setLocalNodeError] = useState('')
  const [status, setStatus] = useState('')
  const [sessions, setSessions] = useState<UiTunerProjectSessionRecord[]>(() => readUiTunerProjectSessions())
  const [activeSessionId, setActiveSessionId] = useState(() => sessions[0]?.id ?? '')
  const [memory, setMemory] = useState(() => readUiTunerModuleMemory())

  useEffect(() => {
    if (!projectsLoaded) loadProjects().catch(() => {})
  }, [loadProjects, projectsLoaded])

  useEffect(() => {
    let canceled = false
    async function loadLocalNode() {
      try {
        const data = await localJson<LocalNodeStatus>(nodeAdminUrl, '/api/status')
        if (!canceled) {
          setLocalNode(data)
          setLocalNodeError('')
        }
      } catch (error) {
        if (!canceled) {
          setLocalNode(null)
          setLocalNodeError((error as { message?: string }).message ?? '本机节点不可用')
        }
      }
    }
    void loadLocalNode()
    const timer = window.setInterval(loadLocalNode, 10_000)
    return () => {
      canceled = true
      window.clearInterval(timer)
    }
  }, [nodeAdminUrl])

  const listedProject = projects.find((project) => project.id === activeProjectId)
  const activeProject = useMemo(
    () => mergeProjectRecords(listedProject, space?.project),
    [listedProject, space?.project],
  )
  const aiChannel = channels.find((channel) => channel.kind === 'ai_development')
  const visibleSessions = useMemo(
    () => sessions.filter((session) => !activeProjectId || session.projectId === activeProjectId),
    [activeProjectId, sessions],
  )
  const selectedSession = visibleSessions.find((session) => session.id === activeSessionId)
    ?? visibleSessions[0]
    ?? null
  const workspacePath = clean(activeProject?.workspace_path ?? activeProject?.storage_worktree_path)
  const localNodeId = clean(localNode?.agent_id)
  const localNodeReady = !!localNodeId
    && !!user?.id
    && clean(localNode?.owner_user_id) === user.id
    && localNode?.connected !== false
    && localNode?.codex_cli?.available !== false
  const canStart = !!activeProject?.id
    && !!aiChannel?.id
    && channelAllowsAiStart(aiChannel)
    && localNodeReady
    && !!workspacePath
    && !sendingMessage
  const sessionMessages = useMemo(() => {
    const conversationId = selectedSession?.conversationId
    if (!conversationId) return []
    return messages
      .filter((message) => clean(message.conversation_id ?? message.conversationId) === conversationId)
      .slice(-4)
  }, [messages, selectedSession?.conversationId])

  async function startSession(mode: 'continue' | 'fork') {
    if (!activeProject?.id || !aiChannel?.id) {
      setStatus('请先选择一个有 AI 开发频道的自项目')
      return
    }
    if (!localNodeReady || !workspacePath) {
      setStatus(localNodeError || '本机 Codex CLI 或项目工作区未就绪')
      return
    }
    if (!channelAllowsAiStart(aiChannel)) {
      setStatus('当前角色不能在这个频道发起 AI 开发')
      return
    }
    const baseSession = selectedSession && mode === 'continue' ? selectedSession : null
    const session = baseSession ?? createUiTunerProjectSession({
      projectId: activeProject.id,
      channelId: aiChannel.id,
      elementName: pack.selectedElement?.name ?? pack.screen.canvasName,
      source: selectedSession,
      memory,
    })
    setStatus(mode === 'fork' ? '正在从最新记忆分叉 Codex 会话…' : '正在发送到项目 Codex 会话…')
    try {
      await selectChannel(aiChannel.id)
      await ensureLocalFullAccessGrant({
        adminUrl: nodeAdminUrl,
        projectId: activeProject.id,
        projectName: activeProject.name,
        workspacePath,
        runtimePermission: activeProject.runtime_permission,
        useLocalRouteA: true,
      })
      const nextMemory = rememberUiTunerIntent(memory, intent, pack.selectedElement?.name ?? '')
      const content = buildUiTunerProjectTaskContent({ pack, intent, memory: nextMemory, session, mode })
      const agent = selectedAgentForRuntimeRoute(selectedAgent, modelOptions, CODEX_ROUTE)
      const response = await sendMessage(
        content,
        agent || null,
        CODEX_ROUTE,
        session.conversationId,
        session.title,
        localNodeId,
        workspacePath,
        aiChannel.id,
        true,
      )
      const saved = updateUiTunerProjectSession(session, {
        taskId: clean(response?.task_id ?? response?.message?.task_id ?? response?.message?.taskId),
        status: 'running',
      })
      saveUiTunerProjectSession(saved)
      writeUiTunerModuleMemory(nextMemory)
      setMemory(nextMemory)
      setSessions(readUiTunerProjectSessions())
      setActiveSessionId(saved.id)
      setStatus('已进入项目 Codex CLI 会话')
    } catch (error) {
      setStatus((error as { message?: string }).message ?? '项目 Codex 会话启动失败')
    }
  }

  return (
    <div className={panelStyles.projectSessionPanel}>
      <div className={panelStyles.projectSessionHeader}>
        <span>项目会话</span>
        <button type="button" title="刷新项目" onClick={() => void loadProjects()}>
          <RefreshCw size={13} aria-hidden="true" />
        </button>
      </div>

      <select
        value={activeProjectId}
        onChange={(event) => { void selectProject(event.currentTarget.value) }}
      >
        <option value="">选择自项目</option>
        {projects.map((project) => (
          <option key={project.id} value={project.id}>{project.display_name || project.name}</option>
        ))}
      </select>

      <div className={panelStyles.sessionFacts}>
        <span>{activeProject ? `项目：${activeProject.display_name || activeProject.name}` : '未选择项目'}</span>
        <span>{aiChannel ? `频道：${aiChannel.name}` : '缺少 AI 开发频道'}</span>
        <span>{localNodeReady ? '本机 Codex CLI 就绪' : localNodeError || '本机 Codex CLI 未就绪'}</span>
      </div>

      <div className={panelStyles.sessionMemory}>
        <strong>模块记忆</strong>
        <small>{memory.stableSummary}</small>
      </div>

      {visibleSessions.length > 0 && (
        <select value={selectedSession?.id ?? ''} onChange={(event) => setActiveSessionId(event.currentTarget.value)}>
          {visibleSessions.map((session) => (
            <option key={session.id} value={session.id}>{session.title}</option>
          ))}
        </select>
      )}

      <div className={panelStyles.codexActions}>
        <button type="button" disabled={!canStart} onClick={() => void startSession('continue')}>
          <Play size={14} aria-hidden="true" />
          继续会话
        </button>
        <button type="button" disabled={!canStart} onClick={() => void startSession('fork')}>
          <GitBranch size={14} aria-hidden="true" />
          分叉会话
        </button>
      </div>

      {status && <p className={panelStyles.codexCopyState}>{status}</p>}

      {sessionMessages.length > 0 && (
        <div className={panelStyles.sessionMessages}>
          {sessionMessages.map((message) => (
            <small key={message.id}>{clean(message.content ?? message.text).slice(0, 160)}</small>
          ))}
        </div>
      )}

      {selectedSession?.taskId && localNodeReady && (
        <div className={panelStyles.sidecarWrap}>
          <div className={panelStyles.sidecarTitle}>
            <TerminalSquare size={13} aria-hidden="true" />
            <span>Codex sidecar</span>
          </div>
          <SidecarTerminalPanel
            adminUrl={nodeAdminUrl}
            session={{
              task_id: selectedSession.taskId,
              cli_name: 'Codex CLI',
              state: selectedSession.status || 'running',
              transport: 'pty',
              capabilities: { terminal_attach: true, terminal_input: true, terminal_resize: true },
            }}
          />
        </div>
      )}
    </div>
  )
}

function uiTunerNodeAdminUrl() {
  const fromQuery = new URLSearchParams(location.search).get('node_admin')
  return safeNodeAdminUrl(fromQuery || 'http://127.0.0.1:7799')
}
