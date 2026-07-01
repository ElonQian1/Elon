/**
 * DevTaskGroup — 任务级折叠层
 *
 * 将同一 task_id 的所有消息（header + N progress + result）
 * 折叠成一行：
 *  - 任务运行中 → 展开，实时显示所有进度
 *  - 任务完成后 → 默认折叠，只显示 header + "查看N步详情" + result
 *  - 点击按钮可随时展开 / 折叠
 */
import { useState, useEffect, useRef } from 'react'
import { DevTaskMessage } from './DevTaskCard'
import { messageKind, messageText, taskIdOf, taskIsTerminal } from './devTaskUtils'
import type { ChatMessage, TaskContext } from './types'
import styles from './DevTaskGroup.module.css'

interface Props {
  messages: ChatMessage[]
  taskContext: TaskContext
  onCancel?: (taskId: string) => void
  onApprove?: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => void
}

export default function DevTaskGroup({ messages, taskContext, onCancel, onApprove }: Props) {
  const taskId  = taskIdOf(messages[0]) || ''
  const task    = taskId ? (taskContext.tasks.get(taskId) ?? null) : null
  const isDone  = taskIsTerminal(task)

  // 任务完成后默认折叠；从历史加载的已完成任务也默认折叠
  const [collapsed, setCollapsed] = useState(isDone)
  const prevDone = useRef(isDone)

  // 任务从"运行中"变为"完成"时自动折叠（延迟一下让用户看到结果）
  useEffect(() => {
    if (!prevDone.current && isDone) {
      const t = setTimeout(() => setCollapsed(true), 800)
      prevDone.current = true
      return () => clearTimeout(t)
    }
  }, [isDone])

  const headerMsg   = messages.find((m) => messageKind(m) === 'ai_task')
  const resultMsg   = messages.find((m) => messageKind(m) === 'ai_result')
    ?? (isDone ? latestVisibleProgress(messages) : undefined)
  const progressMsgs = messages.filter((m) => messageKind(m) === 'ai_progress')
  const progressCount = progressMsgs.length

  // 未完成：直接渲染所有消息（实时追踪进度）
  if (!isDone) {
    return (
      <>
        {messages.map((msg, i) => (
          <DevTaskMessage
            key={String(msg.id ?? i)}
            message={msg}
            context={taskContext}
            onCancel={onCancel}
            onApprove={onApprove}
          />
        ))}
      </>
    )
  }

  // 已完成：可折叠布局
  return (
    <div className={styles.group}>
      {/* 任务标题行（始终可见）*/}
      {headerMsg && (
        <DevTaskMessage
          message={headerMsg}
          context={taskContext}
          onCancel={onCancel}
          onApprove={onApprove}
        />
      )}

      {/* 折叠切换按钮（有进度消息才显示）*/}
      {progressCount > 0 && (
        <button
          type="button"
          className={styles.toggle}
          onClick={() => setCollapsed((c) => !c)}
          aria-expanded={!collapsed}
        >
          <span className={styles.toggleArrow}>{collapsed ? '▸' : '▾'}</span>
          <span>
            {collapsed
              ? `查看 ${progressCount} 步详情`
              : '收起详情'}
          </span>
        </button>
      )}

      {/* 进度消息（折叠时隐藏）*/}
      {!collapsed && progressMsgs.map((msg, i) => (
        <DevTaskMessage
          key={String(msg.id ?? i)}
          message={msg}
          context={taskContext}
          onCancel={onCancel}
          onApprove={onApprove}
        />
      ))}

      {/* 结果块（始终可见）*/}
      {resultMsg && (
        <DevTaskMessage
          message={resultMsg}
          context={taskContext}
          onCancel={onCancel}
          onApprove={onApprove}
        />
      )}
    </div>
  )
}

function latestVisibleProgress(messages: ChatMessage[]): ChatMessage | undefined {
  const progress = messages.filter((m) => messageKind(m) === 'ai_progress')
  for (let index = progress.length - 1; index >= 0; index--) {
    const message = progress[index]
    const text = messageText(message)
    if (!text || text.includes('正在处理中…') || text.startsWith('AI 还在')) continue
    return { ...message, kind: 'ai_result' }
  }
  return undefined
}
