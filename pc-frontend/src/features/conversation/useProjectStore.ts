import { create } from 'zustand'
import { api } from '../../api/client'
import type { Project, Channel, ChannelCategory, Message, ProjectMember, ProjectSpace, ProjectListResponse, ChannelMessagesResponse } from './types'

interface ProjectState {
  projects: Project[]
  projectsLoaded: boolean
  activeProjectId: string
  space: ProjectSpace | null
  channels: Channel[]
  categories: ChannelCategory[]
  members: ProjectMember[]
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
  channels: [],
  categories: [],
  members: [],
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
    set({ activeProjectId: id, space: null, channels: [], categories: [], members: [], activeChannelId: '', messages: [] })
    try {
      const space = await api.get<ProjectSpace>(`/api/projects/${encodeURIComponent(id)}/space`)
      const channels = space.channels ?? []
      set({ space, channels, categories: space.categories ?? [], members: space.members ?? [] })
      if (channels.length > 0) {
        // 优先选 ai_development 频道，其次选第一个
        const preferred = channels.find((c) => c.kind === 'ai_development') ?? channels[0]
        await get().selectChannel(preferred.id)
      }
    } catch (err) {
      console.warn('Failed to load project space:', err)
    }
  },

  reloadProjectSpace: async () => {
    const { activeProjectId, activeChannelId } = get()
    if (!activeProjectId) return
    const space = await api.get<ProjectSpace>(`/api/projects/${encodeURIComponent(activeProjectId)}/space`)
    const channels = space.channels ?? []
    const nextActive = channels.some((c) => c.id === activeChannelId) ? activeChannelId : (channels[0]?.id ?? '')
    set({
      space,
      channels,
      categories: space.categories ?? [],
      members: space.members ?? [],
      activeChannelId: nextActive,
    })
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

  startPolling: () => {
    const { pollTimer } = get()
    if (pollTimer) return
    const timer = setInterval(() => {
      const { activeProjectId, activeChannelId, sendingMessage } = get()
      if (activeProjectId && activeChannelId && !sendingMessage) {
        get().loadMessages(activeProjectId, activeChannelId).catch(() => {})
      }
    }, 5000)
    set({ pollTimer: timer })
  },

  stopPolling: () => {
    const { pollTimer } = get()
    if (pollTimer) {
      clearInterval(pollTimer)
      set({ pollTimer: null })
    }
  },
}))
