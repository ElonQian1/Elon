import { useEffect, useRef, useState } from 'react'
import type { AgentOption } from '../models/types'
import { selectedAgentForRuntimeRoute } from '../models/routeModelPolicy'
import type { RuntimeRoute } from './runtimeRoutes'
import type { MemberConversationEntry } from './memberConversationApi'
import {
  initialProjectPrewarmFromStorage,
  persistProjectPrewarmSelection,
  PROJECT_PREWARM_COOLDOWN_MS,
  requestProjectPrewarm,
} from './projectPrewarm'
import styles from './ConversationComposer.module.css'

interface ComposerRuntimeTogglesProps {
  activeProjectId: string
  directPcCliActive: boolean
  shouldPreferLocalNode: boolean
  localNodeReady: boolean
  directPcCliAvailable: boolean
  composerDisabled: boolean
  onDirectPcCliChange: (enabled: boolean) => void
  isOwnConversationTarget: boolean
  sessionView: string | 'new' | null
  memberConversations: MemberConversationEntry[]
  selectedAgent: string
  modelOptions: AgentOption[]
  composerRuntimeRoute: RuntimeRoute
}

export default function ComposerRuntimeToggles({
  activeProjectId,
  directPcCliActive,
  shouldPreferLocalNode,
  localNodeReady,
  directPcCliAvailable,
  composerDisabled,
  onDirectPcCliChange,
  isOwnConversationTarget,
  sessionView,
  memberConversations,
  selectedAgent,
  modelOptions,
  composerRuntimeRoute,
}: ComposerRuntimeTogglesProps) {
  const [projectPrewarmEnabled, setProjectPrewarmEnabled] = useState(() => initialProjectPrewarmFromStorage(
    typeof window === 'undefined' ? null : window.localStorage,
  ))
  const projectPrewarmRef = useRef<Map<string, number>>(new Map())

  useEffect(() => {
    persistProjectPrewarmSelection(window.localStorage, projectPrewarmEnabled)
  }, [projectPrewarmEnabled])

  useEffect(() => {
    if (!projectPrewarmEnabled) return
    if (!activeProjectId || !isOwnConversationTarget || !sessionView || sessionView === 'new') return
    const conversationId = String(sessionView)
    const agent = selectedAgentForRuntimeRoute(selectedAgent, modelOptions, composerRuntimeRoute)
    const key = `${activeProjectId}:${conversationId}:${agent || 'default'}`
    const now = Date.now()
    const lastStartedAt = projectPrewarmRef.current.get(key) ?? 0
    if (now - lastStartedAt < PROJECT_PREWARM_COOLDOWN_MS) return
    projectPrewarmRef.current.set(key, now)

    const conversation = memberConversations.find((item) => item.id === conversationId)
    const payload: {
      conversation_id: string
      conversation_title?: string | null
      agent?: string
      trace_id?: string
    } = {
      conversation_id: conversationId,
      conversation_title: conversation?.title ?? null,
      trace_id: `pc_prewarm:${activeProjectId}:${conversationId}:${now}`,
    }
    if (agent) payload.agent = agent

    requestProjectPrewarm(activeProjectId, payload)
      .catch((err: { status?: number; message?: string }) => {
        console.warn('[ProjectPrewarm] failed:', err?.status, err?.message)
      })
  }, [
    projectPrewarmEnabled,
    activeProjectId,
    isOwnConversationTarget,
    sessionView,
    memberConversations,
    selectedAgent,
    modelOptions,
    composerRuntimeRoute,
  ])

  return (
    <>
      <label
        className={styles.directCliToggle}
        data-active={directPcCliActive ? 'true' : 'false'}
        data-default-local={shouldPreferLocalNode && localNodeReady ? 'true' : 'false'}
        data-disabled={!directPcCliAvailable || composerDisabled ? 'true' : 'false'}
        title="默认先使用平台 AI；打开后强制交给本机 AI CLI"
      >
        <input
          type="checkbox"
          checked={directPcCliActive}
          disabled={!directPcCliAvailable || composerDisabled}
          onChange={(event) => onDirectPcCliChange(event.target.checked)}
        />
        <span className={styles.directCliSwitch} aria-hidden="true" />
        <span className={styles.directCliCopy}>
          <strong>{directPcCliActive ? '直连CLI' : '自动'}</strong>
          <em>{!directPcCliAvailable ? '未就绪' : directPcCliActive ? '直连' : '自动'}</em>
        </span>
      </label>
      <label
        className={styles.projectPrewarmToggle}
        data-active={projectPrewarmEnabled ? 'true' : 'false'}
        data-disabled={!activeProjectId ? 'true' : 'false'}
        title="打开后，进入项目会话会提前确认本机节点和工作区状态"
      >
        <input
          type="checkbox"
          checked={projectPrewarmEnabled}
          disabled={!activeProjectId}
          onChange={(event) => setProjectPrewarmEnabled(event.target.checked)}
        />
        <span aria-hidden="true" />
        <strong>预热</strong>
      </label>
    </>
  )
}
