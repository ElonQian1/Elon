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
import { LOCAL_NODE_BASE_CHANGED_EVENT } from '../../api/runtime'
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
  resolveFitRunWorkspace,
} from './fit-run/fitRunEvents'
import { useFitRunStore } from './fit-run/fitRunStore'
import { TERMINAL_FIT_RUN_PHASES } from './fit-run/types'
import { loadAiTaskSettlement } from './fit-run/taskSettlement'
import { aiWritebackReceiptInstruction } from './source-preview/aiWritebackReceipt'
import panelStyles from './UiTunerPanels.module.css'

interface UiTunerProjectSessionPanelProps {
  pack: UiTunerCodexContextPack | null
  intent: string
  onMutationTaskStarted: (pack: UiTunerCodexContextPack) => Promise<void> | void
  onTaskSettled: () => void
  headless?: boolean
}

const CODEX_ROUTE: RuntimeRoute = 'route_a'

interface FitRunTaskRef {
  runId: string
  handoffId: string
  taskId: string
  kind?: 'FIT_RUN' | 'PWA_DRAFT'
}

interface SessionStartOptions {
  fitRun?: Pick<FitRunTaskRef, 'runId' | 'handoffId'>
  handoffKind?: 'FIT_RUN' | 'PWA_DRAFT'
  contextPack?: UiTunerCodexContextPack
  workspacePathOverride?: string
}

export function UiTunerProjectSessionPanel({
  pack,
  intent,
  onMutationTaskStarted,
  onTaskSettled,
  headless = false,
}: UiTunerProjectSessionPanelProps) {
  const [nodeAdminUrl, setNodeAdminUrl] = useState(uiTunerNodeAdminUrl)
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
  const headlessSettledTaskRef = useRef('')
  const activeFitRun = useFitRunStore((state) => state.run)
  const startSessionRef = useRef<(
    mode: UiTunerConversationMode,
    overrideIntent?: string,
    options?: SessionStartOptions,
  ) => Promise<UiTunerProjectSessionRecord | null>>(async () => null)

  useEffect(() => {
    const syncNodeAdminUrl = () => setNodeAdminUrl(uiTunerNodeAdminUrl())
    window.addEventListener(LOCAL_NODE_BASE_CHANGED_EVENT, syncNodeAdminUrl)
    return () => window.removeEventListener(LOCAL_NODE_BASE_CHANGED_EVENT, syncNodeAdminUrl)
  }, [])

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

  useEffect(() => {
    const taskId = clean(trackedFitRunTask?.taskId)
    const projectId = clean(activeProject?.id)
    const channelId = clean(aiChannel?.id)
    if (!headless || !taskId || !projectId || !channelId || headlessSettledTaskRef.current === taskId) return
    let canceled = false
    let timer = 0
    const poll = async () => {
      try {
        const settlement = await loadAiTaskSettlement(projectId, channelId, taskId)
        if (canceled) return
        if (settlement) {
          headlessSettledTaskRef.current = taskId
          notifyFitRunCodexSettled(settlement)
          setFitRunTask((current) => current?.taskId === taskId ? null : current)
          return
        }
      } catch {
        // The server task may not be queryable for a short time immediately after launch.
      }
      if (!canceled) timer = window.setTimeout(poll, 4_000)
    }
    void poll()
    return () => {
      canceled = true
      if (timer) window.clearTimeout(timer)
    }
  }, [activeProject?.id, aiChannel?.id, headless, trackedFitRunTask?.taskId])

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
      const activePack = options?.contextPack ?? pack
      if (!activePack) {
        setStatus('当前 UI 草稿缺少 AI Context Artifact，无法启动源码接力')
        return null
      }
      const executionWorkspace = resolveFitRunWorkspace({
        workspacePath: options?.workspacePathOverride,
        contextPack: activePack,
      }, workspacePath)
      if (!localNodeReady || !executionWorkspace.workspacePath) {
        setStatus(!executionWorkspace.workspacePath ? '当前 UI 草稿缺少本机源码目录' : localNodeStatusText)
        return null
      }
      const session = mode === 'fork'
        ? await forkUiTunerConversation({
            projectId: activeProject.id,
            conversationId: source.conversationId,
            title: `微调画布 · ${activePack.selectedElement?.name || activePack.screen.canvasName}`.slice(0, 80),
            selectedElementName: activePack.selectedElement?.name,
          })
        : source
      await selectChannel(aiChannel.id)
      await ensureLocalFullAccessGrant({
        adminUrl: nodeAdminUrl,
        projectId: activeProject.id,
        projectName: activeProject.name,
        workspacePath: executionWorkspace.workspacePath,
        runtimePermission: executionWorkspace.isOverride ? 'full_access' : activeProject.runtime_permission,
        useLocalRouteA: true,
      })
      const artifact = await createUiTunerContextArtifact({
        projectId: activeProject.id,
        conversationId: session.conversationId,
        userIntent: taskIntent,
        pack: activePack,
      })
      const agent = selectedAgentForRuntimeRoute(selectedAgent, modelOptions, CODEX_ROUTE)
      const response = await sendMessage(
        taskIntent,
        agent || null,
        CODEX_ROUTE,
        session.conversationId,
        session.title,
        localNodeId,
        executionWorkspace.workspacePath,
        aiChannel.id,
        true,
        undefined,
        {
          moduleKey: 'ui-tuner',
          contextArtifactId: artifact.id,
          transientWorkspace: executionWorkspace.isOverride,
        },
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
        setFitRunTask({ ...options.fitRun, taskId, kind: options.handoffKind })
        setStatus(options.handoffKind === 'PWA_DRAFT'
          ? '跨端草稿已进入持续项目 Codex CLI 会话'
          : 'FitRun 已交给持续项目 Codex CLI 会话；构建验收由 FitRun 统一调度')
      } else {
        setConversationOpen(true)
        setVerificationTaskId(taskId)
        await onMutationTaskStarted(activePack)
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
      : request.handoffKind === 'PWA_DRAFT'
        ? '本轮低 Token 跨端草稿已随现有 Context Artifact 提交'
        : '先通过 yilong-ui-live MCP 读取当前 FitRun handoff'
    const taskIntent = request.handoffKind === 'PWA_DRAFT' ? [
      `继续跨端 PWA 设计草稿 ${request.runId}。`,
      artifact,
      `路由原因：${request.reason}`,
      '这是源码写回任务：在已授权工作区修改并保存必要文件。',
      '为节省 Token，第一步只读取 Context Artifact 的 pwaDesign.compactHandoff：按 elements[].changedProperties 执行样式写回，优先打开 sourceFilesToInspect 中的候选文件。',
      '只有 compactHandoff 证据不足时，才展开 artifact 中的局部 DOM、compactSourceBundle、相关裁剪与视觉差异；不要默认读取整仓库或整棵 DOM。',
      '跳过 deterministicResult 已完成的写回；补充 PWA 来源绑定、结构调整或复杂 TSX/Kotlin。',
      'PWA Runtime DOM 仅是临时证据，最终必须写回源码并同时说明 PWA/APK 两端结果。',
      aiWritebackReceiptInstruction(),
    ].join('\n') : [
      `继续设计稿自动拟合任务 ${request.runId}。`, artifact,
      `平台期原因：${request.reason}`,
      '这是源码修改任务：在已授权工作区修改并保存必要文件。',
      '为节省 Token，优先使用 handoff 指向的目标裁剪、当前裁剪、节点子树和局部源码，缺少证据时再按需读取。',
      '优先修正布局结构或 Source Binding；不要用临时 translation 冒充可写回布局。',
      '修改并保存源码；不要自行构建、安装、截图或启动第二个验收流程，完成后由 FitRun 统一构建和评分。',
    ].join('\n')
    fitRunStartingRef.current = true
    setFitRunStarting(true)
    void startSessionRef.current('continue', taskIntent, {
      fitRun: { runId: request.runId, handoffId: request.handoffId },
      handoffKind: request.handoffKind,
      contextPack: request.contextPack,
      workspacePathOverride: request.workspacePath,
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

  if (headless || !pack) return null

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
  return safeNodeAdminUrl()
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
