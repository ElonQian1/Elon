import { create } from 'zustand'
import { api } from '../../api/client'
import type { Project, Channel, ChannelCategory, Message, ProjectMember, ProjectSpace, ProjectLanding, ProjectListResponse, ChannelMessagesResponse } from './types'

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
  messagesLoading: boolean
  sendingMessage: boolean
  pollTimer: ReturnType<typeof setInterval> | null

  loadProjects: () => Promise<void>
  selectProject: (id: string) => Promise<void>
  reloadProjectSpace: () => Promise<void>
  selectChannel: (id: string) => Promise<void>
  loadMessages: (projectId: string, channelId: string) => Promise<void>
  sendMessage: (content: string, agent?: string | null) => Promise<void>
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
  messagesLoading: false,
  sendingMessage: false,
  pollTimer: null,

  loadProjects: async () => {
    const data = await api.get<ProjectListResponse>('/api/me/projects')
    set({ projects: data.projects ?? [], projectsLoaded: true })
  },

  selectProject: async (id: string) => {
    if (get().activeProjectId === id) return
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
      // 进入项目后停留在项目首页（landing），由用户手动选择频道。
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

  selectChannel: async (id: string) => {
    const { activeProjectId } = get()
    if (!activeProjectId) return
    get().stopPolling()
    set({ activeChannelId: id, messages: [] })
    await get().loadMessages(activeProjectId, id)
    get().startPolling()
  },

  loadMessages: async (projectId: string, channelId: string) => {
    set({ messagesLoading: true })
    try {
      const data = await api.get<ChannelMessagesResponse>(
        `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/messages?limit=120`,
      )
      set({ messages: data.messages ?? [] })
    } catch (err) {
      console.warn('Failed to load messages:', err)
    } finally {
      set({ messagesLoading: false })
    }
  },

  sendMessage: async (content: string, agent?: string | null) => {
    const { activeProjectId, activeChannelId } = get()
    if (!activeProjectId || !activeChannelId || !content.trim()) return
    set({ sendingMessage: true })
    try {
      await api.post(
        `/api/projects/${encodeURIComponent(activeProjectId)}/channels/${encodeURIComponent(activeChannelId)}/ai-tasks`,
        { content, agent: agent ?? null },
      )
      // 立即刷新消息
      await get().loadMessages(activeProjectId, activeChannelId)
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
