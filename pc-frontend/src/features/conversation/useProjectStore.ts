import { create } from 'zustand'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import type { Project, Channel, ChannelCategory, Message, ProjectMember, ProjectSpace, ProjectLanding, ProjectListResponse, ChannelMessagesResponse, SendMessageResponse, ProjectAttachmentRef } from './types'
import { DEFAULT_RUNTIME_ROUTE } from './runtimeRoutes'
import type { RuntimeRoute } from './runtimeRoutes'

interface ChannelMessageCacheEntry {
  messages: Message[]
  loadedAt: number
}

interface MemberPresencePatch {
  status?: string
  customStatus?: string | null
  custom_status?: string | null
  activity?: string | null
}

const CHANNEL_MESSAGE_CACHE_FRESH_MS = 10000
const CACHED_CHANNEL_REFRESH_DELAY_MS = 300
const PROJECT_SPACE_CACHE_PREFIX = 'elon.pc.projectSpace.v1'
const PROJECT_SPACE_CACHE_MAX_AGE_MS = 24 * 60 * 60 * 1000
const PROJECT_SELECTION_KEY = 'elon.pc.projectSelection.v1'

interface ProjectSpaceCacheEntry {
  projectId: string
  userId: string
  cachedAt: number
  space: ProjectSpace
}

interface ProjectSelectionEntry {
  userId: string
  projectId: string
  channelId: string
  updatedAt: number
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
  applyMemberPresence: (userId: string, isOnline: boolean, patch?: MemberPresencePatch) => void
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
    attachments?: ProjectAttachmentRef[],
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
    let projects: Project[]
    try {
      const data = await api.get<ProjectListResponse>('/api/me/projects')
      projects = data.projects ?? []
    } catch (err) {
      if ((err as { status?: number })?.status === 401) {
        set({
          projects: [],
          projectsLoaded: true,
          activeProjectId: '',
          space: null,
          landing: null,
          channels: [],
          categories: [],
          members: [],
          activeChannelId: '',
          messages: [],
        })
        return
      }
      throw err
    }

    set({ projects, projectsLoaded: true })

    const currentProjectId = get().activeProjectId
    const selection = readProjectSelection()
    if (!currentProjectId && selection?.projectId && projects.some((p) => p.id === selection.projectId)) {
      await get().selectProject(selection.projectId)
      const channelId = selection.channelId
      if (channelId && get().channels.some((c) => c.id === channelId)) {
        await get().selectChannel(channelId)
      }
    }
  },

  selectProject: async (id: string) => {
    if (get().activeProjectId === id) {
      get().stopPolling()
      writeProjectSelection(id, '')
      set((state) => ({
        activeChannelId: '',
        messages: [],
        projectHomeVersion: state.projectHomeVersion + 1,
      }))
      return
    }
    get().stopPolling()
    writeProjectSelection(id, '')
    const cachedSpace = id ? readProjectSpaceCache(id) : null
    const cachedChannels = cachedSpace?.channels ?? []
    set({
      activeProjectId: id,
      space: cachedSpace,
      landing: cachedSpace?.landing ?? null,
      channels: cachedChannels,
      categories: cachedSpace?.categories ?? [],
      members: cachedSpace?.members ?? [],
      spaceLoading: !!id,
      spaceError: '',
      activeChannelId: '',
      messages: [],
      projectHomeVersion: get().projectHomeVersion + 1,
    })
    if (!id) {
      clearProjectSelection()
      set({ spaceLoading: false })
      return
    }  // 空 id = 返回项目列表，不加载 space
    try {
      const space = await api.get<ProjectSpace>(`/api/projects/${encodeURIComponent(id)}/space`)
      const channels = space.channels ?? []
      writeProjectSpaceCache(id, space)
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
      writeProjectSpaceCache(activeProjectId, space)
      writeProjectSelection(activeProjectId, nextActive)
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

  applyMemberPresence: (userId: string, isOnline: boolean, patch: MemberPresencePatch = {}) => {
    const targetId = userId.trim()
    if (!targetId) return
    const updateMember = (member: ProjectMember): ProjectMember => {
      if (member.user_id !== targetId) return member
      const currentStatus = String(member.presence_status ?? '').trim().toLowerCase()
      const eventStatus = String(patch.status ?? '').trim().toLowerCase()
      const nextStatus = isOnline
        ? (eventStatus && eventStatus !== 'offline' && eventStatus !== 'invisible'
          ? eventStatus
          : currentStatus && currentStatus !== 'offline' && currentStatus !== 'invisible'
            ? currentStatus
            : 'online')
        : 'offline'
      const hasCustomStatus = Object.prototype.hasOwnProperty.call(patch, 'customStatus')
        || Object.prototype.hasOwnProperty.call(patch, 'custom_status')
      const hasActivity = Object.prototype.hasOwnProperty.call(patch, 'activity')
      const customStatus = hasCustomStatus ? (patch.customStatus ?? patch.custom_status ?? null) : member.custom_status
      const activity = hasActivity ? (patch.activity ?? null) : member.activity
      if (
        member.is_online === isOnline
        && member.presence_status === nextStatus
        && member.custom_status === customStatus
        && member.activity === activity
      ) return member
      return {
        ...member,
        is_online: isOnline,
        presence_status: nextStatus,
        custom_status: customStatus,
        activity,
      }
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
    writeProjectSelection(activeProjectId, id)
    set({
      activeChannelId: id,
      messages: cached?.messages ?? [],
      messagesLoading: !cached,
    })
    if (cached && Date.now() - cached.loadedAt < CHANNEL_MESSAGE_CACHE_FRESH_MS) {
      get().startPolling()
      return
    }
    if (cached) {
      window.setTimeout(() => {
        const state = get()
        if (state.activeProjectId !== activeProjectId || state.activeChannelId !== id) return
        state.loadMessages(activeProjectId, id).catch(() => {})
      }, CACHED_CHANNEL_REFRESH_DELAY_MS)
      get().startPolling()
      return
    }
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
    attachments?: ProjectAttachmentRef[],
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
          attachments: attachments?.length ? attachments : undefined,
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

function projectSpaceCacheKey(projectId: string): string {
  const userId = useAuthStore.getState().user?.id || 'anonymous'
  return `${PROJECT_SPACE_CACHE_PREFIX}:${userId}:${projectId}`
}

function readProjectSpaceCache(projectId: string): ProjectSpace | null {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(projectSpaceCacheKey(projectId))
    if (!raw) return null
    const entry = JSON.parse(raw) as ProjectSpaceCacheEntry
    const userId = useAuthStore.getState().user?.id || 'anonymous'
    if (entry.projectId !== projectId || entry.userId !== userId || !entry.space) return null
    if (Date.now() - Number(entry.cachedAt || 0) > PROJECT_SPACE_CACHE_MAX_AGE_MS) return null
    return entry.space
  } catch {
    return null
  }
}

function writeProjectSpaceCache(projectId: string, space: ProjectSpace): void {
  if (typeof window === 'undefined') return
  try {
    const userId = useAuthStore.getState().user?.id || 'anonymous'
    const entry: ProjectSpaceCacheEntry = {
      projectId,
      userId,
      cachedAt: Date.now(),
      space,
    }
    window.localStorage.setItem(projectSpaceCacheKey(projectId), JSON.stringify(entry))
  } catch {
    // Storage may be full or disabled; the live store still works.
  }
}

function readProjectSelection(): ProjectSelectionEntry | null {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(PROJECT_SELECTION_KEY)
    if (!raw) return null
    const entry = JSON.parse(raw) as ProjectSelectionEntry
    const userId = useAuthStore.getState().user?.id || 'anonymous'
    if (entry.userId !== userId || !entry.projectId) return null
    return entry
  } catch {
    return null
  }
}

function writeProjectSelection(projectId: string, channelId: string) {
  if (typeof window === 'undefined' || !projectId) return
  try {
    const userId = useAuthStore.getState().user?.id || 'anonymous'
    const entry: ProjectSelectionEntry = {
      userId,
      projectId,
      channelId,
      updatedAt: Date.now(),
    }
    window.localStorage.setItem(PROJECT_SELECTION_KEY, JSON.stringify(entry))
  } catch {
    // Selection restore is best effort only.
  }
}

function clearProjectSelection() {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.removeItem(PROJECT_SELECTION_KEY)
  } catch {
    // ignore
  }
}
