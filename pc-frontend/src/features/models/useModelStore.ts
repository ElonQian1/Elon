import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { api } from '../../api/client'
import { buildOptions, resolveSelection } from './modelUtils'
import type { AgentOption, AgentConfigResponse } from './types'

const CACHE_AGENT_KEY = 'elon_pc_selected_agent_name'
const CACHE_LABEL_KEY = 'elon_pc_selected_model_label'

interface ModelState {
  options: AgentOption[]
  selectedAgent: string
  label: string
  codexCliOnly: boolean
  byokEnabled: boolean
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
      initialized: false,
      loading: false,
      error: '',

      load: async (userId: string) => {
        set({ loading: true, error: '' })
        try {
          const data = await api.get<AgentConfigResponse>(
            `/api/user/${encodeURIComponent(userId)}/agent`,
          )
          const options = buildOptions(data ?? {})
          const cachedAgent = localStorage.getItem(CACHE_AGENT_KEY) ?? ''
          const { selectedAgent, label } = resolveSelection(data ?? {}, options, cachedAgent)

          localStorage.setItem(CACHE_AGENT_KEY, selectedAgent)
          localStorage.setItem(CACHE_LABEL_KEY, label)

          set({
            options,
            selectedAgent,
            label,
            codexCliOnly: !!(data?.codex_cli_only),
            byokEnabled: !!(data?.user_byok_api_enabled),
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
        localStorage.setItem(CACHE_AGENT_KEY, option.agentName)
        localStorage.setItem(CACHE_LABEL_KEY, option.label)
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
