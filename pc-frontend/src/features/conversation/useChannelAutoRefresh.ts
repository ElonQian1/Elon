/**
 * P1.2：频道消息实时刷新
 *
 * 两条刷新路径：
 * 1. 监听 `elon:project-task-done` 自定义事件（由 useNotifications 从 /ws/app 派发）
 *    → 事件中的 projectId 与当前活跃项目匹配时立即刷新
 * 2. 自适应轮询：
 *    - 有未完成任务（ai_task / ai_progress 消息出现但还没 ai_result）→ 3s
 *    - 空闲 → 8s
 */
import { useEffect, useRef } from 'react'
import { useProjectStore } from './useProjectStore'

const POLL_FAST_MS = 3_000   // 任务进行中
const POLL_IDLE_MS = 8_000   // 空闲

interface TaskDoneEvent extends CustomEvent {
  detail: { projectId?: string; conversationId?: string }
}

interface GroupAiMatterEvent extends CustomEvent {
  detail: { projectId?: string; matterId?: string; matterEventType?: string }
}

export function useChannelAutoRefresh() {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    // 监听任务完成事件
    function onTaskDone(e: TaskDoneEvent) {
      const { activeProjectId, activeChannelId, loadMessages } = useProjectStore.getState()
      const eventProjectId = e.detail?.projectId ?? ''
      // 仅当事件属于当前活跃项目时触发刷新
      if (activeProjectId && activeChannelId && (!eventProjectId || eventProjectId === activeProjectId)) {
        loadMessages(activeProjectId, activeChannelId).catch(() => {})
      }
    }

    function onGroupAiMatterEvent(e: GroupAiMatterEvent) {
      const { activeProjectId, activeChannelId, loadMessages } = useProjectStore.getState()
      const eventProjectId = e.detail?.projectId ?? ''
      if (activeProjectId && activeChannelId && (!eventProjectId || eventProjectId === activeProjectId)) {
        loadMessages(activeProjectId, activeChannelId).catch(() => {})
      }
    }

    window.addEventListener('elon:project-task-done', onTaskDone as EventListener)
    window.addEventListener('elon:project-ai-matter-event', onGroupAiMatterEvent as EventListener)
    return () => {
      window.removeEventListener('elon:project-task-done', onTaskDone as EventListener)
      window.removeEventListener('elon:project-ai-matter-event', onGroupAiMatterEvent as EventListener)
    }
  }, [])

  // 自适应轮询
  useEffect(() => {
    let currentChannel = ''

    function schedule() {
      if (timerRef.current) clearTimeout(timerRef.current)

      const { activeProjectId, activeChannelId, messages, sendingMessage } = useProjectStore.getState()
      if (!activeProjectId || !activeChannelId) return

      const hasOpenTask = hasRunningTask(messages)
      const interval = (hasOpenTask || sendingMessage) ? POLL_FAST_MS : POLL_IDLE_MS

      timerRef.current = setTimeout(async () => {
        const { activeProjectId: pid, activeChannelId: cid, sendingMessage: busy } = useProjectStore.getState()
        if (pid && cid && !busy) {
          await useProjectStore.getState().loadMessages(pid, cid).catch(() => {})
        }
        schedule()
      }, interval)
    }

    // Zustand 4 单参数 subscribe：检测 channel 切换
    const unsub = useProjectStore.subscribe((s) => {
      const key = `${s.activeProjectId}:${s.activeChannelId}`
      if (key !== currentChannel) {
        currentChannel = key
        schedule()
      }
    })
    schedule()

    return () => {
      unsub()
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [])
}

/** 判断消息列表里是否有正在运行的任务（有 ai_task 但无对应 ai_result）*/
function hasRunningTask(messages: { kind?: string; role?: string; task_id?: string; taskId?: string }[]): boolean {
  const taskIds = new Set<string>()
  const doneIds = new Set<string>()
  for (const m of messages) {
    const kind = (m.kind ?? m.role ?? '').toLowerCase()
    const taskId = m.task_id ?? m.taskId ?? ''
    if (!taskId) continue
    if (kind === 'ai_task') taskIds.add(taskId)
    if (kind === 'ai_result') doneIds.add(taskId)
  }
  for (const id of taskIds) {
    if (!doneIds.has(id)) return true
  }
  return false
}
