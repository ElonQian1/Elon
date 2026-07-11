import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Check, GitBranch, MessageSquareText, Play, RefreshCw, TerminalSquare, Users, X } from 'lucide-react'
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
  clearLegacyUiTunerModuleMemory,
  readLegacyUiTunerModuleMemory,
  type UiTunerProjectSessionRecord,
  type UiTunerWorkspaceResponse,
} from './projectSessions'
import {
  createUiTunerContextArtifact,
  forkUiTunerConversation,
  importLegacyUiTunerWorkspace,
  loadUiTunerWorkspace,
  reviewUiTunerMemory,
} from './uiTunerWorkspaceApi'
import UiTunerConversationDrawer from './UiTunerConversationDrawer'
import type { UiTunerConversationMode } from './uiTunerConversation'
import {
  listenForFitRunCodexRequests,
  notifyFitRunCodexSettled,
  readFitRunCodexLaunch,
} from './fit-run/fitRunEvents'
import { useFitRunStore } from './fit-run/fitRunStore'
import { TERMINAL_FIT_RUN_PHASES } from './fit-run/types'
import panelStyles from './UiTunerPanels.module.css'

interface UiTunerProjectSessionPanelProps {
  pack: UiTunerCodexContextPack
  intent: string
  onMutationTaskStarted: (pack: UiTunerCodexContextPack) => Promise<void> | void
  onTaskSettled: () => void
}

const CODEX_ROUTE: RuntimeRoute = 'route_a'

interface FitRunTaskRef {
  runId: string
  handoffId: string
  taskId: string
}

interface SessionStartOptions {
  fitRun?: Pick<FitRunTaskRef, 'runId' | 'handoffId'>
}

export function UiTunerProjectSessionPanel({
  pack,
  intent,
  onMutationTaskStarted,
  onTaskSettled,
}: UiTunerProjectSessionPanelProps) {
  const nodeAdminUrl = uiTunerNodeAdminUrl()
  const user = useAuthStore((state) => state.user)
  const selectedAgent = useModelStore((state) => state.selectedAgent)
  const modelOptions = useModelStore((state) => state.options)
  const {
    projects, projectsLoaded, activeProjectId, space, channels, messages, sendingMessage,
    loadProjects, selectProject, selectChannel, sendMessage,
  } = useProjectStore()
  const [localNode, setLocalNode] = useState<LocalNodeStatus | null>(null)
  const [localNodeError, setLocalNodeError] = useState('')
  const [status, setStatus] = useState('')
  const [workspaceState, setWorkspaceState] = useState<UiTunerWorkspaceResponse | null>(null)
  const [workspaceLoading, setWorkspaceLoading] = useState(false)
  const [activeSessionId, setActiveSessionId] = useState('')
  const [conversationOpen, setConversationOpen] = useState(false)
  const [verificationTaskId, setVerificationTaskId] = useState('')
  const [fitRunTask, setFitRunTask] = useState<FitRunTaskRef | null>(null)
  const [fitRunStarting, setFitRunStarting] = useState(false)
  const fitRunStartingRef = useRef(false)
  const activeFitRun = useFitRunStore((state) => state.run)
  const startSessionRef = useRef<(
    mode: UiTunerConversationMode,
    overrideIntent?: string,
    options?: SessionStartOptions,
  ) => Promise<UiTunerProjectSessionRecord | null>>(async () => null)

  useEffect(() => {
    if (!projectsLoaded) loadProjects().catch(() => {})
  }, [loadProjects, projectsLoaded])

  useEffect(() => {
    let canceled = false
    async function loadLocalNode() {
      try {
        const data = await localJson<LocalNodeStatus>(nodeAdminUrl, '/api/status')
        if (!canceled) { setLocalNode(data); setLocalNodeError('') }
      } catch (error) {
        if (!canceled) {
          setLocalNode(null)
          setLocalNodeError((error as { message?: string }).message ?? '本机节点不可用')
        }
      }
    }
    void loadLocalNode()
    const timer = window.setInterval(loadLocalNode, 10_000)
    return () => { canceled = true; window.clearInterval(timer) }
  }, [nodeAdminUrl])

  const listedProject = projects.find((project) => project.id === activeProjectId)
  const activeProject = useMemo(
    () => mergeProjectRecords(listedProject, space?.project),
    [listedProject, space?.project],
  )
  const aiChannel = channels.find((channel) => channel.kind === 'ai_development')

  const refreshWorkspace = useCallback(async (showSpinner = true) => {
    if (!activeProject?.id) {
      setWorkspaceState(null)
      setActiveSessionId('')
      return null
    }
    if (showSpinner) setWorkspaceLoading(true)
    try {
      let next = await loadUiTunerWorkspace(activeProject.id)
      const legacyMemory = next.workspace.memoryRevision === 1 && !next.workspace.lastCheckpointId
        ? readLegacyUiTunerModuleMemory(activeProject.id)
        : null
      if (legacyMemory) {
        next = await importLegacyUiTunerWorkspace(activeProject.id, legacyMemory)
        clearLegacyUiTunerModuleMemory()
      }
      setWorkspaceState(next)
      setActiveSessionId((current) => {
        if (next.sessions.some((session) => session.id === current)) return current
        const preferred = next.workspace.activeConversationId || next.workspace.canonicalConversationId
        return next.sessions.find((session) => session.conversationId === preferred)?.id
          ?? next.sessions[0]?.id
          ?? ''
      })
      return next
    } catch (error) {
      setStatus((error as { message?: string }).message ?? 'ui-tuner 服务端工作区加载失败')
      return null
    } finally {
      if (showSpinner) setWorkspaceLoading(false)
    }
  }, [activeProject?.id])

  useEffect(() => {
    setWorkspaceState(null)
    setActiveSessionId('')
    void refreshWorkspace()
  }, [refreshWorkspace])

  const visibleSessions = workspaceState?.sessions ?? []
  const selectedSession = visibleSessions.find((session) => session.id === activeSessionId)
    ?? visibleSessions.find((session) => session.conversationId === workspaceState?.workspace.activeConversationId)
    ?? visibleSessions[0]
    ?? null
  const acceptedMemories = workspaceState?.memories.filter((memory) => memory.status === 'accepted') ?? []
  const candidateMemories = workspaceState?.memories.filter((memory) => memory.status === 'candidate') ?? []
  const workspacePath = clean(activeProject?.workspace_path ?? activeProject?.storage_worktree_path)
  const localNodeId = clean(localNode?.agent_id)
  const localNodeReady = !!localNodeId
    && !!user?.id
    && clean(localNode?.owner_user_id) === user.id
    && localNode?.connected !== false
    && localNode?.codex_cli?.available !== false
  const localNodeStatusText = describeLocalNodeStatus(localNode, localNodeReady, localNodeError, user?.id)
  const canStartBase = !!activeProject?.id
    && !!aiChannel?.id
    && !!selectedSession
    && channelAllowsAiStart(aiChannel)
    && localNodeReady
    && !!workspacePath
    && !workspaceLoading
    && !sendingMessage
  const restoredFitRunTask = useMemo<FitRunTaskRef | null>(() => {
    const handoff = activeFitRun?.handoff
    if (!activeFitRun || !handoff) return null
    const taskId = handoff.taskId
      ?? readFitRunCodexLaunch(activeFitRun.runId, handoff.handoffId)?.taskId
    if (!taskId || !['AWAITING_CODEX', 'CODEX_RUNNING'].includes(activeFitRun.phase)) return null
    return { runId: activeFitRun.runId, handoffId: handoff.handoffId, taskId }
  }, [activeFitRun])
  const trackedFitRunTask = fitRunTask ?? restoredFitRunTask
  const fitRunPipelineActive = Boolean(activeFitRun && !TERMINAL_FIT_RUN_PHASES.has(activeFitRun.phase))
  const canStart = canStartBase && !fitRunPipelineActive && !trackedFitRunTask && !fitRunStarting
  const canStartFitRun = canStartBase && !trackedFitRunTask && !verificationTaskId && !fitRunStarting
  const sessionMessages = useMemo(() => {
    if (!selectedSession?.conversationId) return []
    return messages
      .filter((message) => clean(message.conversation_id ?? message.conversationId) === selectedSession.conversationId)
      .slice(-4)
  }, [messages, selectedSession?.conversationId])

  useEffect(() => {
    if (!selectedSession || !['running', 'recovering'].includes(selectedSession.status)) return
    const timer = window.setInterval(() => { void refreshWorkspace(false) }, 4_000)
    return () => window.clearInterval(timer)
  }, [refreshWorkspace, selectedSession])

  async function startSession(
    mode: UiTunerConversationMode,
    overrideIntent?: string,
    options?: SessionStartOptions,
  ): Promise<UiTunerProjectSessionRecord | null> {
    if (options?.fitRun) {
      if (!canStartFitRun) {
        setStatus('已有 Codex 任务正在启动或运行，请等待后再继续 FitRun')
        return null
      }
    } else if (fitRunPipelineActive || trackedFitRunTask || fitRunStartingRef.current) {
      setStatus('设计稿 FitRun 正在使用 Codex，完成前不能并发启动手工源码任务')
      return null
    }
    if (!activeProject?.id || !aiChannel?.id) {
      setStatus('请先选择一个有 AI 开发频道的自项目')
      return null
    }
    if (!localNodeReady || !workspacePath) {
      setStatus(!workspacePath ? '自项目缺少本机工作区路径' : localNodeStatusText)
      return null
    }
    if (!channelAllowsAiStart(aiChannel)) {
      setStatus('当前角色不能在这个频道发起 AI 开发')
      return null
    }
    const current = workspaceState ?? await refreshWorkspace()
    const source = selectedSession
      ?? current?.sessions.find((session) => session.conversationId === current.workspace.activeConversationId)
      ?? current?.sessions[0]
    if (!source) {
      setStatus('服务端 ui-tuner 主会话尚未就绪')
      return null
    }
    setStatus(mode === 'fork' ? '正在从最新稳定检查点分叉…' : '正在发送到持续项目会话…')
    try {
      const taskIntent = overrideIntent?.trim() || intent.trim() || '继续优化微调画布和 APK UI 标准闭环。'
      const session = mode === 'fork'
        ? await forkUiTunerConversation({
            projectId: activeProject.id,
            conversationId: source.conversationId,
            title: `微调画布 · ${pack.selectedElement?.name || pack.screen.canvasName}`.slice(0, 80),
            selectedElementName: pack.selectedElement?.name,
          })
        : source
      await selectChannel(aiChannel.id)
      await ensureLocalFullAccessGrant({
        adminUrl: nodeAdminUrl,
        projectId: activeProject.id,
        projectName: activeProject.name,
        workspacePath,
        runtimePermission: activeProject.runtime_permission,
        useLocalRouteA: true,
      })
      const artifact = await createUiTunerContextArtifact({
        projectId: activeProject.id,
        conversationId: session.conversationId,
        userIntent: taskIntent,
        pack,
      })
      const agent = selectedAgentForRuntimeRoute(selectedAgent, modelOptions, CODEX_ROUTE)
      const response = await sendMessage(
        taskIntent,
        agent || null,
        CODEX_ROUTE,
        session.conversationId,
        session.title,
        localNodeId,
        workspacePath,
        aiChannel.id,
        true,
        undefined,
        { moduleKey: 'ui-tuner', contextArtifactId: artifact.id },
      )
      const taskId = clean(response?.task_id ?? response?.message?.task_id ?? response?.message?.taskId)
      setActiveSessionId(session.id)
      setWorkspaceState((previous) => previous ? {
        ...previous,
        workspace: { ...previous.workspace, activeConversationId: session.conversationId },
        sessions: previous.sessions.some((item) => item.id === session.id)
          ? previous.sessions.map((item) => item.id === session.id ? { ...item, taskId, status: 'running' } : item)
          : [{ ...session, taskId, status: 'running' }, ...previous.sessions],
      } : previous)
      if (options?.fitRun) {
        setFitRunTask({ ...options.fitRun, taskId })
        setStatus('FitRun 已交给持续项目 Codex CLI 会话；构建验收由 FitRun 统一调度')
      } else {
        setConversationOpen(true)
        setVerificationTaskId(taskId)
        await onMutationTaskStarted(pack)
        setStatus('已进入持续项目 Codex CLI 会话')
      }
      window.setTimeout(() => { void refreshWorkspace(false) }, 600)
      return { ...session, taskId, status: 'running' }
    } catch (error) {
      setStatus((error as { message?: string }).message ?? '项目 Codex 会话启动失败')
      return null
    }
  }

  startSessionRef.current = startSession

  useEffect(() => listenForFitRunCodexRequests((request) => {
    if (!canStartFitRun || fitRunStartingRef.current) {
      request.reject(new Error('当前项目 Codex 会话或本机节点尚未就绪'))
      return
    }
    const artifact = request.handoffPath
      ? `先读取 FitRun handoff：${request.handoffPath}`
      : '先通过 yilong-ui-live MCP 读取当前 FitRun handoff'
    const taskIntent = [
      `继续设计稿自动拟合任务 ${request.runId}。`,
      artifact,
      `平台期原因：${request.reason}`,
      '只读取 handoff 指向的目标裁剪、当前裁剪、节点子树和局部源码。',
      '优先修正布局结构或 Source Binding；不要用临时 translation 冒充可写回布局。',
      '只修改并保存源码，不要自行构建、安装、截图或启动第二个验收流程；完成后由 FitRun 统一构建和评分。',
    ].join('\n')
    fitRunStartingRef.current = true
    setFitRunStarting(true)
    void startSessionRef.current('continue', taskIntent, {
      fitRun: { runId: request.runId, handoffId: request.handoffId },
    }).then((session) => {
      if (!session?.taskId) throw new Error('Codex 任务未返回 taskId')
      request.resolve({ taskId: session.taskId })
    }).catch((error) => {
      request.reject(error instanceof Error ? error : new Error('Codex FitRun 交接失败'))
    }).finally(() => {
      fitRunStartingRef.current = false
      setFitRunStarting(false)
    })
  }), [canStartFitRun])

  async function decideMemory(
    memoryId: string,
    decision: 'accepted' | 'rejected',
    scopeType: 'user' | 'project' = 'user',
  ) {
    if (!activeProject?.id) return
    try {
      await reviewUiTunerMemory({ projectId: activeProject.id, memoryId, decision, scopeType })
      await refreshWorkspace(false)
      setStatus(decision === 'accepted' ? '候选记忆已接受' : '候选记忆已忽略')
    } catch (error) {
      setStatus((error as { message?: string }).message ?? '记忆审核失败')
    }
  }

  return (
    <div className={panelStyles.projectSessionPanel}>
      <div className={panelStyles.projectSessionHeader}>
        <span>项目会话</span>
        <button type="button" title="刷新项目与模块会话" onClick={() => { void loadProjects(); void refreshWorkspace() }}>
          <RefreshCw size={13} aria-hidden="true" />
        </button>
      </div>

      <select value={activeProjectId} onChange={(event) => { void selectProject(event.currentTarget.value) }}>
        <option value="">选择自项目</option>
        {projects.map((project) => (
          <option key={project.id} value={project.id}>{project.display_name || project.name}</option>
        ))}
      </select>

      <div className={panelStyles.sessionFacts}>
        <span>{activeProject ? `项目：${activeProject.display_name || activeProject.name}` : '未选择项目'}</span>
        <span>{aiChannel ? `频道：${aiChannel.name}` : '缺少 AI 开发频道'}</span>
        <span>{localNodeStatusText}</span>
      </div>

      <div className={panelStyles.sessionMemory}>
        <strong>服务端模块记忆 · r{workspaceState?.workspace.memoryRevision ?? 0}</strong>
        <small>{workspaceState?.workspace.stableSummary || (workspaceLoading ? '正在加载模块记忆…' : '选择项目后加载')}</small>
        <span>{acceptedMemories.length} 条已接受 · {candidateMemories.length} 条待确认</span>
      </div>

      {candidateMemories.slice(0, 3).map((memory) => (
        <div className={panelStyles.memoryCandidate} key={memory.id}>
          <small>{memory.content}</small>
          <div>
            <button type="button" title="接受为个人模块记忆" onClick={() => void decideMemory(memory.id, 'accepted')}>
              <Check size={13} aria-hidden="true" />
            </button>
            <button type="button" title="接受为项目共享模块记忆" onClick={() => void decideMemory(memory.id, 'accepted', 'project')}>
              <Users size={13} aria-hidden="true" />
            </button>
            <button type="button" title="忽略候选记忆" onClick={() => void decideMemory(memory.id, 'rejected')}>
              <X size={13} aria-hidden="true" />
            </button>
          </div>
        </div>
      ))}

      {visibleSessions.length > 0 && (
        <select value={selectedSession?.id ?? ''} onChange={(event) => setActiveSessionId(event.currentTarget.value)}>
          {visibleSessions.map((session) => (
            <option key={session.id} value={session.id}>
              {session.isCanonical ? '主会话 · ' : '分叉 · '}{session.title}
            </option>
          ))}
        </select>
      )}

      <div className={panelStyles.codexActions}>
        <button type="button" disabled={!selectedSession} onClick={() => setConversationOpen(true)}>
          <MessageSquareText size={14} aria-hidden="true" />打开聊天
        </button>
        <button type="button" disabled={!canStart} onClick={() => void startSession('continue')}>
          <Play size={14} aria-hidden="true" />继续会话
        </button>
        <button type="button" disabled={!canStart} onClick={() => void startSession('fork')}>
          <GitBranch size={14} aria-hidden="true" />分叉会话
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

      <div className={panelStyles.sidecarWrap}>
        <div className={panelStyles.sidecarTitle}><TerminalSquare size={13} aria-hidden="true" /><span>Codex sidecar</span></div>
        {selectedSession?.taskId && localNodeReady ? (
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
        ) : (
          <small className={panelStyles.sidecarHint}>
            {selectedSession?.taskId ? localNodeStatusText : '启动继续/分叉会话后显示托管 Codex sidecar 终端'}
          </small>
        )}
      </div>

      <UiTunerConversationDrawer
        open={conversationOpen}
        onClose={() => setConversationOpen(false)}
        pack={pack}
        intent={intent}
        activeProject={activeProject}
        aiChannel={aiChannel}
        workspacePath={workspacePath}
        canStart={canStart}
        localNodeReady={localNodeReady}
        localNodeStatusText={localNodeStatusText}
        selectedSession={selectedSession}
        visibleSessions={visibleSessions}
        verificationTaskId={trackedFitRunTask?.taskId ?? verificationTaskId}
        status={status}
        onSelectSession={setActiveSessionId}
        onStartSession={startSession}
        onTaskSettled={(succeeded) => {
          if (trackedFitRunTask) {
            notifyFitRunCodexSettled({ taskId: trackedFitRunTask.taskId, succeeded })
            setFitRunTask(null)
            return
          }
          setVerificationTaskId('')
          if (succeeded) onTaskSettled()
        }}
      />
    </div>
  )
}

function uiTunerNodeAdminUrl() {
  const fromQuery = new URLSearchParams(location.search).get('node_admin')
  return safeNodeAdminUrl(fromQuery || 'http://127.0.0.1:7799')
}

function describeLocalNodeStatus(
  localNode: LocalNodeStatus | null,
  ready: boolean,
  error: string,
  userId?: string,
) {
  if (ready) return '本机 Codex CLI 就绪'
  if (error) return `本机节点不可用：${error}`
  if (!localNode) return '正在检测本机节点'
  if (!clean(localNode.agent_id)) return '本机节点未返回 agent_id'
  if (userId && clean(localNode.owner_user_id) && clean(localNode.owner_user_id) !== userId) return '本机节点属于其他账号'
  if (localNode.connected === false) {
    return localNode.codex_cli?.available === false ? '本机节点未连接云端，Codex CLI 未就绪' : 'Codex CLI 就绪，节点未连接云端'
  }
  if (localNode.codex_cli?.available === false) return '本机 Codex CLI 未就绪'
  return '本机 Codex CLI 未就绪'
}
