import { create } from 'zustand'
import { api } from '../../api/client'
import type { Project, Channel, ChannelCategory, Message, ProjectMember, ProjectSpace, ProjectLanding, ProjectListResponse, ChannelMessagesResponse, SendMessageResponse } from './types'
import { DEFAULT_RUNTIME_ROUTE } from './runtimeRoutes'
import type { RuntimeRoute } from './runtimeRoutes'

interface ChannelMessageCacheEntry {
  messages: Message[]
  loadedAt: number
}

interface ProjectState {
  projects: Project[]
  projectsLoaded: boolean
  activeProjectId: string
  space: ProjectSpace | null
  landing: ProjectLanding | null
  channels: Channel[]
  categories: ChannelCategory[]
  members: ProjectMember[]
  spaceLoading: boolean
  spaceError: string
  activeChannelId: string
  messages: Message[]
  messageCache: Record<string, ChannelMessageCacheEntry>
  messageRequestSeq: number
  messagesLoading: boolean
  sendingMessage: boolean
  pollTimer: ReturnType<typeof setInterval> | null
  projectHomeVersion: number

  loadProjects: () => Promise<void>
  selectProject: (id: string) => Promise<void>
  reloadProjectSpace: () => Promise<void>
  applyMemberPresence: (userId: string, isOnline: boolean) => void
  selectChannel: (id: string) => Promise<void>
  loadMessages: (projectId: string, channelId: string) => Promise<void>
  sendMessage: (
    content: string,
    agent?: string | null,
    runtimeRoute?: RuntimeRoute,
    conversationId?: string | null,
    conversationTitle?: string | null,
    localNodeId?: string | null,
    localWorkspacePath?: string | null,
    channelIdOverride?: string | null,
    directPcCli?: boolean,
  ) => Promise<SendMessageResponse | null>
  cancelTask: (taskId: string) => Promise<void>
  approveTool: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
  startPolling: () => void
  stopPolling: () => void
}

export const useProjectStore = create<ProjectState>()((set, get) => ({
  projects: [],
  projectsLoaded: false,
  activeProjectId: '',
  space: null,
  landing: null,
  channels: [],
  categories: [],
  members: [],
  spaceLoading: false,
  spaceError: '',
  activeChannelId: '',
  messages: [],
  messageCache: {},
  messageRequestSeq: 0,
  messagesLoading: false,
  sendingMessage: false,
  pollTimer: null,
  projectHomeVersion: 0,

  loadProjects: async () => {
    const data = await api.get<ProjectListResponse>('/api/me/projects')
    set({ projects: data.projects ?? [], projectsLoaded: true })
  },

  selectProject: async (id: string) => {
    if (get().activeProjectId === id) {
      get().stopPolling()
      set((state) => ({
        activeChannelId: '',
        messages: [],
        projectHomeVersion: state.projectHomeVersion + 1,
      }))
      return
    }
    get().stopPolling()
    set({
      activeProjectId: id,
      space: null,
      landing: null,
      channels: [],
      categories: [],
      members: [],
      spaceLoading: !!id,
      spaceError: '',
      activeChannelId: '',
      messages: [],
      projectHomeVersion: get().projectHomeVersion + 1,
    })
    if (!id) {
      set({ spaceLoading: false })
      return
    }  // 空 id = 返回项目列表，不加载 space
    try {
      const space = await api.get<ProjectSpace>(`/api/projects/${encodeURIComponent(id)}/space`)
      const channels = space.channels ?? []
      set({
        space,
        landing: space.landing ?? null,
        channels,
        categories: space.categories ?? [],
        members: space.members ?? [],
        spaceLoading: false,
        spaceError: '',
      })
      // 不自动进入频道：让项目首页展示，用户手动选择频道
    } catch (err) {
      console.warn('Failed to load project space:', err)
      set({ spaceLoading: false, spaceError: (err as { message?: string }).message ?? '项目空间加载失败' })
    }
  },

  reloadProjectSpace: async () => {
    const { activeProjectId, activeChannelId } = get()
    if (!activeProjectId) return
    set({ spaceLoading: true, spaceError: '' })
    try {
      const space = await api.get<ProjectSpace>(`/api/projects/${encodeURIComponent(activeProjectId)}/space`)
      const channels = space.channels ?? []
      const nextActive = channels.some((c) => c.id === activeChannelId) ? activeChannelId : (channels[0]?.id ?? '')
      set({
        space,
        landing: space.landing ?? null,
        channels,
        categories: space.categories ?? [],
        members: space.members ?? [],
        activeChannelId: nextActive,
        spaceLoading: false,
        spaceError: '',
      })
    } catch (err) {
      console.warn('Failed to reload project space:', err)
      set({ spaceLoading: false, spaceError: (err as { message?: string }).message ?? '项目空间加载失败' })
    }
  },

  applyMemberPresence: (userId: string, isOnline: boolean) => {
    const targetId = userId.trim()
    if (!targetId) return
    const updateMember = (member: ProjectMember): ProjectMember => {
      if (member.user_id !== targetId) return member
      const currentStatus = String(member.presence_status ?? '').trim().toLowerCase()
      const nextStatus = isOnline
        ? (currentStatus && currentStatus !== 'offline' && currentStatus !== 'invisible' ? currentStatus : 'online')
        : 'offline'
      if (member.is_online === isOnline && member.presence_status === nextStatus) return member
      return { ...member, is_online: isOnline, presence_status: nextStatus }
    }
    set((state) => {
      if (!state.members.some((member) => member.user_id === targetId)) return {}
      const members = state.members.map(updateMember)
      const spaceMembers = state.space?.members?.map(updateMember)
      return {
        members,
        space: state.space && spaceMembers ? { ...state.space, members: spaceMembers } : state.space,
      }
    })
  },

  selectChannel: async (id: string) => {
    const { activeProjectId } = get()
    if (!activeProjectId) return
    get().stopPolling()
    const cached = get().messageCache[channelMessageCacheKey(activeProjectId, id)]
    set({
      activeChannelId: id,
      messages: cached?.messages ?? [],
      messagesLoading: !cached,
    })
    await get().loadMessages(activeProjectId, id)
    get().startPolling()
  },

  loadMessages: async (projectId: string, channelId: string) => {
    const startState = get()
    const isActiveRequest = startState.activeProjectId === projectId && startState.activeChannelId === channelId
    const requestSeq = isActiveRequest ? startState.messageRequestSeq + 1 : startState.messageRequestSeq
    const key = channelMessageCacheKey(projectId, channelId)
    if (isActiveRequest) {
      set({ messagesLoading: true, messageRequestSeq: requestSeq })
    }
    try {
      const data = await api.get<ChannelMessagesResponse>(
        `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/messages?limit=120`,
      )
      const nextMessages = data.messages ?? []
      set((state) => {
        const nextCache = {
          ...state.messageCache,
          [key]: { messages: nextMessages, loadedAt: Date.now() },
        }
        if (
          !isActiveRequest
          || state.messageRequestSeq !== requestSeq
          || state.activeProjectId !== projectId
          || state.activeChannelId !== channelId
        ) {
          return { messageCache: nextCache }
        }
        return {
          messages: nextMessages,
          messageCache: nextCache,
          messagesLoading: false,
        }
      })
    } catch (err) {
      console.warn('Failed to load messages:', err)
      const state = get()
      if (
        isActiveRequest
        && state.messageRequestSeq === requestSeq
        && state.activeProjectId === projectId
        && state.activeChannelId === channelId
      ) {
        set({ messagesLoading: false })
      }
    }
  },

  sendMessage: async (
    content: string,
    agent?: string | null,
    runtimeRoute: RuntimeRoute = DEFAULT_RUNTIME_ROUTE,
    conversationId?: string | null,
    conversationTitle?: string | null,
    localNodeId?: string | null,
    localWorkspacePath?: string | null,
    channelIdOverride?: string | null,
    directPcCli?: boolean,
  ) => {
    const { activeProjectId, activeChannelId } = get()
    const channelId = channelIdOverride || activeChannelId
    if (!activeProjectId || !channelId || !content.trim()) return null
    set({ sendingMessage: true })
    try {
      const response = await api.post<SendMessageResponse>(
        `/api/projects/${encodeURIComponent(activeProjectId)}/channels/${encodeURIComponent(channelId)}/ai-tasks`,
        {
          content,
          agent: agent ?? null,
          runtimeRoute,
          conversation_id: conversationId || undefined,
          conversation_title: conversationTitle || undefined,
          localNodeId: localNodeId || undefined,
          localWorkspacePath: localWorkspacePath || undefined,
          directPcCli: directPcCli || undefined,
        },
      )
      // 立即刷新消息
      await get().loadMessages(activeProjectId, channelId)
      return response
    } catch (err) {
      console.warn('Failed to send message:', err)
      throw err
    } finally {
      set({ sendingMessage: false })
    }
  },

  cancelTask: async (taskId: string) => {
    const { activeProjectId, activeChannelId } = get()
    if (!activeProjectId || !activeChannelId) return
    await api.post(
      `/api/projects/${encodeURIComponent(activeProjectId)}/channels/${encodeURIComponent(activeChannelId)}/ai-tasks/${encodeURIComponent(taskId)}/cancel`,
      {},
    )
    await get().loadMessages(activeProjectId, activeChannelId)
  },

  approveTool: async (taskId: string, approvalId: string, decision: 'approve' | 'deny') => {
    const { activeProjectId, activeChannelId } = get()
    if (!activeProjectId || !activeChannelId) return
    await api.post(
      `/api/projects/${encodeURIComponent(activeProjectId)}/channels/${encodeURIComponent(activeChannelId)}/ai-tasks/${encodeURIComponent(taskId)}/tool-approvals/${encodeURIComponent(approvalId)}/decision`,
      { decision },
    )
    await get().loadMessages(activeProjectId, activeChannelId)
  },

  // startPolling / stopPolling 保留接口，实际调度已移至 useChannelAutoRefresh
  // selectChannel 里仍调用它们以便后续扩展
  startPolling: () => { /* 调度由 useChannelAutoRefresh 自适应管理 */ },
  stopPolling: () => {
    const { pollTimer } = get()
    if (pollTimer) {
      clearInterval(pollTimer)
      set({ pollTimer: null })
    }
  },
}))

function channelMessageCacheKey(projectId: string, channelId: string): string {
  return `${projectId}::${channelId}`
}
