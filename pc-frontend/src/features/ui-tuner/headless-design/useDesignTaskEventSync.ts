import { useCallback, useEffect, useRef, useState } from 'react'
import {
  commitDesignEventCheckpoint,
  getDesignEventCheckpoint,
} from './designPlanningApi'
import {
  bindDesignTask,
  getDesignTaskBinding,
  listDesignEvents,
  renewDesignTaskBinding,
  settleDesignTaskBinding,
} from './designSessionApi'
import type { DesignEventCheckpoint } from './designPlanningTypes'
import type { DesignEvent, DesignTaskBinding } from './types'

const CHECKPOINT_CONSUMER_ID = 'pc-ui-tuner'

export interface DesignTaskActivity {
  running: boolean
  taskId: string
  succeeded?: boolean
}

interface Input {
  active: boolean
  projectRoot: string
  designSessionId?: string
  draftId?: string
  reload: (designSessionId?: string) => Promise<unknown>
}

export function useDesignTaskEventSync(input: Input) {
  const [taskId, setTaskId] = useState('')
  const [binding, setBinding] = useState<DesignTaskBinding | null>(null)
  const [cursor, setCursor] = useState('')
  const [latestEvents, setLatestEvents] = useState<DesignEvent[]>([])
  const [checkpoint, setCheckpoint] = useState<DesignEventCheckpoint | null>(null)
  const [lastSyncedAt, setLastSyncedAt] = useState('')
  const [error, setError] = useState('')
  const cursorRef = useRef('')
  const checkpointRevisionRef = useRef(0)
  const checkpointSupportedRef = useRef(true)
  const taskIdRef = useRef('')
  const designSessionIdRef = useRef(input.designSessionId)
  const draftIdRef = useRef(input.draftId)
  designSessionIdRef.current = input.designSessionId
  draftIdRef.current = input.draftId

  const follow = useCallback(async (nextTaskId: string) => {
    taskIdRef.current = nextTaskId
    cursorRef.current = ''
    checkpointRevisionRef.current = 0
    checkpointSupportedRef.current = true
    setTaskId('')
    setBinding(null)
    setCursor('')
    setCheckpoint(null)
    setLatestEvents([])
    setError('')
    try {
      const restored = await getDesignEventCheckpoint(input.projectRoot, CHECKPOINT_CONSUMER_ID, nextTaskId)
      cursorRef.current = restored.resumeAfterCursor
      checkpointRevisionRef.current = restored.revision
      setCursor(restored.resumeAfterCursor)
      setCheckpoint(restored.checkpoint ?? null)
    } catch {
      checkpointSupportedRef.current = false
    }
    try {
      if (designSessionIdRef.current) {
        const result = await bindDesignTask({
          projectRoot: input.projectRoot,
          taskId: nextTaskId,
          designSessionId: designSessionIdRef.current,
          draftId: draftIdRef.current,
          leaseSeconds: 900,
        })
        setBinding(result.binding ?? null)
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计任务绑定失败')
    } finally {
      if (taskIdRef.current === nextTaskId) setTaskId(nextTaskId)
    }
  }, [input.projectRoot])

  const settle = useCallback(async (activity: DesignTaskActivity) => {
    const activeBinding = binding?.taskId === activity.taskId ? binding : null
    try {
      if (activeBinding) {
        const result = await settleDesignTaskBinding({
          projectRoot: input.projectRoot,
          taskId: activity.taskId,
          leaseId: activeBinding.leaseId,
          succeeded: activity.succeeded,
        })
        setBinding(result.binding ?? activeBinding)
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计任务 lease 结算失败')
    } finally {
      if (taskIdRef.current === activity.taskId) {
        taskIdRef.current = ''
        setTaskId('')
      }
    }
  }, [binding, input.projectRoot])

  const onTaskActivityChange = useCallback((activity: DesignTaskActivity) => (
    activity.running ? follow(activity.taskId) : settle(activity)
  ), [follow, settle])

  useEffect(() => {
    if (!input.active || !taskId) return
    let cancelled = false
    let retryTimer = 0
    const tick = async () => {
      try {
        const page = await listDesignEvents({
          projectRoot: input.projectRoot,
          taskId,
          afterCursor: cursorRef.current || undefined,
          limit: 50,
          waitMs: 12_000,
        })
        if (cancelled) return
        if (page.events.length) {
          const designSessionId = [...page.events].reverse()
            .find((event) => event.designSessionId)?.designSessionId ?? binding?.designSessionId
          if (page.events.some((event) => event.eventType === 'TASK_BOUND')) {
            const result = await getDesignTaskBinding(input.projectRoot, taskId)
            if (!cancelled) setBinding(result.binding ?? null)
          }
          await input.reload(designSessionId ?? undefined)
          if (!cancelled) {
            if (checkpointSupportedRef.current) {
              const committed = await commitDesignEventCheckpoint({
                projectRoot: input.projectRoot,
                consumerId: CHECKPOINT_CONSUMER_ID,
                taskId,
                cursor: page.cursor,
                expectedRevision: checkpointRevisionRef.current,
              })
              checkpointRevisionRef.current = committed.checkpoint.revision
              setCheckpoint(committed.checkpoint)
            }
            cursorRef.current = page.cursor
            setCursor(page.cursor)
            setLatestEvents((previous) => [...previous, ...page.events].slice(-20))
            setLastSyncedAt(new Date().toISOString())
          }
        }
        if (!cancelled) setError('')
      } catch (reason) {
        if (!cancelled) setError(reason instanceof Error ? reason.message : '设计事件跟随失败')
        await new Promise<void>((resolve) => { retryTimer = window.setTimeout(resolve, 1_500) })
      }
      if (!cancelled) void tick()
    }
    void tick()
    return () => {
      cancelled = true
      window.clearTimeout(retryTimer)
    }
  }, [binding, input.active, input.projectRoot, input.reload, taskId])

  useEffect(() => {
    if (!input.active || !taskId || !binding || binding.status !== 'ACTIVE') return
    const timer = window.setInterval(() => {
      void renewDesignTaskBinding({
        projectRoot: input.projectRoot,
        taskId,
        leaseId: binding.leaseId,
        leaseSeconds: 900,
      }).then((result) => setBinding(result.binding ?? binding))
        .catch((reason) => setError(reason instanceof Error ? reason.message : '设计任务 lease 续租失败'))
    }, 5 * 60_000)
    return () => window.clearInterval(timer)
  }, [binding, input.active, input.projectRoot, taskId])

  return {
    active: Boolean(taskId), taskId, binding, cursor, checkpoint, latestEvents,
    lastSyncedAt, error, onTaskActivityChange,
  }
}
