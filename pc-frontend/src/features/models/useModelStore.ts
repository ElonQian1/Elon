import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { api } from '../../api/client'
import { safeNodeAdminUrl } from '../../lib/utils'
import { probeLocalNode } from '../node/localNodeApi'
import type { LocalNodeStatus } from '../node/types'
import { buildOptions, mergeLocalNodeOptions, resolveSelection } from './modelUtils'
import type { AgentOption, AgentConfigResponse } from './types'

const CACHE_AGENT_KEY = 'elon_pc_selected_agent_name'
const CACHE_LABEL_KEY = 'elon_pc_selected_model_label'

function accountCacheKey(baseKey: string, userId: string): string {
  return `${baseKey}:user:${encodeURIComponent(userId.trim())}`
}

interface ModelState {
  options: AgentOption[]
  selectedAgent: string
  label: string
  codexCliOnly: boolean
  byokEnabled: boolean
  localNodeOnline: boolean
  localAiSummary: string
  initialized: boolean
  loading: boolean
  error: string

  load: (userId: string) => Promise<void>
  saveSelection: (option: AgentOption, userId: string) => Promise<void>
  reset: () => void
}

export const useModelStore = create<ModelState>()(
  persist(
    (set, get) => ({
      options: [],
      selectedAgent: '',
      label: 'AI',
      codexCliOnly: false,
      byokEnabled: false,
      localNodeOnline: false,
      localAiSummary: '',
      initialized: false,
      loading: false,
      error: '',

      load: async (userId: string) => {
        // 清掉上一个账号在 Zustand 持久化状态里的展示值；真正的选择只从
        // 当前账号的服务端配置和当前账号缓存恢复，避免新账号短暂继承旧账号模型。
        set({
          loading: true,
          error: '',
          selectedAgent: '',
          label: 'AI',
          options: [],
        })
        try {
          const [data, localStatus] = await Promise.all([
            api.get<AgentConfigResponse>(`/api/user/${encodeURIComponent(userId)}/agent`),
            probeLocalNode(safeNodeAdminUrl())
              .then((status) => status as LocalNodeStatus)
              .catch(() => null),
          ])
          const options = mergeLocalNodeOptions(buildOptions(data ?? {}), localStatus)
          const cachedAgent = localStorage.getItem(accountCacheKey(CACHE_AGENT_KEY, userId)) ?? ''
          const { selectedAgent, label } = resolveSelection(data ?? {}, options, cachedAgent)
          const localCliCount =
            localStatus?.allowed_clis?.length ??
            localStatus?.cli_tools?.filter((item) => item.available !== false).length ??
            0
          const localModelCount = localStatus?.local_ai?.models?.length ?? localStatus?.models?.length ?? 0

          localStorage.setItem(accountCacheKey(CACHE_AGENT_KEY, userId), selectedAgent)
          localStorage.setItem(accountCacheKey(CACHE_LABEL_KEY, userId), label)

          set({
            options,
            selectedAgent,
            label,
            codexCliOnly: !!(data?.codex_cli_only),
            byokEnabled: !!(data?.user_byok_api_enabled),
            localNodeOnline: !!localStatus,
            localAiSummary: localStatus
              ? `本机AI：CLI ${localCliCount} 个，本机模型 ${localModelCount} 个`
              : '本机节点未连接',
            initialized: true,
            error: '',
          })
        } catch (err) {
          set({ error: (err as { message?: string }).message ?? '模型列表加载失败' })
        } finally {
          set({ loading: false })
        }
      },

      saveSelection: async (option: AgentOption, userId: string) => {
        const { codexCliOnly } = get()
        if (codexCliOnly && !option.agentName) {
          throw new Error('当前已锁定使用 Codex CLI。')
        }
        await api.put(`/api/user/${encodeURIComponent(userId)}/agent`, {
          use_agent: option.agentName || null,
          api_base: null,
          api_key: null,
          model: null,
        })
        localStorage.setItem(accountCacheKey(CACHE_AGENT_KEY, userId), option.agentName)
        localStorage.setItem(accountCacheKey(CACHE_LABEL_KEY, userId), option.label)
        set({ selectedAgent: option.agentName, label: option.label })
      },

      reset: () => {
        localStorage.removeItem(CACHE_AGENT_KEY)
        set({
          options: [],
          selectedAgent: '',
          label: 'AI',
          codexCliOnly: false,
          byokEnabled: false,
          localNodeOnline: false,
          localAiSummary: '',
          initialized: false,
          error: '',
        })
      },
    }),
    {
      name: 'elon_model',
      partialize: (s) => ({ selectedAgent: s.selectedAgent, label: s.label }),
    },
  ),
)
