import { PlayCircle, RotateCcw } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import type { TaskTerminalActionModel } from './taskTerminalActionModel'
import styles from './TaskTerminalActions.module.css'

interface Props {
  action: TaskTerminalActionModel
  taskId: string
  localNodeReady: boolean
  localNodeRequired: boolean
  onContinue?: (taskId: string) => void | Promise<void>
}

type ActionState = 'idle' | 'waiting' | 'starting' | 'started' | 'error'

export default function TaskTerminalActions({
  action,
  taskId,
  localNodeReady,
  localNodeRequired,
  onContinue,
}: Props) {
  const [state, setState] = useState<ActionState>('idle')
  const [feedback, setFeedback] = useState('')
  const startedRef = useRef(false)
  const requiresNode = action.requiresNode || localNodeRequired

  const start = useCallback(async () => {
    if (!taskId || !onContinue || startedRef.current) return
    startedRef.current = true
    setState('starting')
    setFeedback('正在接回原任务和会话上下文...')
    try {
      await onContinue(taskId)
      setState('started')
      setFeedback('已从原任务继续处理，无需重新发送提示词。')
    } catch (error) {
      startedRef.current = false
      setState('error')
      setFeedback((error as { message?: string }).message ?? '继续任务失败，请稍后重试。')
    }
  }, [onContinue, taskId])

  useEffect(() => {
    if (state !== 'waiting' || !localNodeReady) return
    setFeedback('节点已恢复，正在自动继续原任务...')
    void start()
  }, [localNodeReady, start, state])

  if (!action.visible || !taskId || !onContinue) return null

  const handleClick = () => {
    if (requiresNode && !localNodeReady) {
      setState('waiting')
      setFeedback('已保留原任务和会话上下文；Win 端重连后会自动继续，无需重新发送提示词。')
      return
    }
    void start()
  }
  const pending = state === 'waiting' || state === 'starting' || state === 'started'
  const label = state === 'waiting'
    ? '等待节点重连'
    : state === 'starting'
      ? '正在继续'
      : state === 'started'
        ? '已继续'
        : action.label
  const Icon = action.label.includes('重试') ? RotateCcw : PlayCircle

  return (
    <div className={styles.root} data-terminal-action={state}>
      <button type="button" className={styles.action} onClick={handleClick} disabled={pending}>
        <Icon size={14} aria-hidden="true" />
        <span>{label}</span>
      </button>
      {feedback && (
        <span className={[styles.feedback, state === 'error' ? styles.feedbackError : ''].filter(Boolean).join(' ')} aria-live="polite">
          {feedback}
        </span>
      )}
    </div>
  )
}
