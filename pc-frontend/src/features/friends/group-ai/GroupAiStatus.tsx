import { useEffect, useRef, useState } from 'react'
import { Bot, ExternalLink, RefreshCw, Square, Check } from 'lucide-react'
import { useGroupAiTask } from './groupAiStore'
import styles from './GroupAi.module.css'

export default function GroupAiStatus({ owner, group, onDelivered }: { owner: string; group: string; onDelivered: () => void }) {
  const task = useGroupAiTask()
  const refresh = useRef(onDelivered)
  refresh.current = onDelivered
  const [error, setError] = useState('')
  const [dismissed, setDismissed] = useState('')
  const phase = task?.progress.phase
  useEffect(() => {
    if (task?.input.owner === owner && task.input.group === group && phase === 'completed') refresh.current()
  }, [task, owner, group, phase])
  useEffect(() => { setError('') }, [task, phase])
  if (!task || task.input.owner !== owner || task.operation === dismissed) return null
  const { message, busy, hasAnswer } = task.progress
  const ended = phase === 'completed' || phase === 'cancelled'
  return <section className={styles.status} aria-label="群聊 AI 进度" aria-busy={busy}>
    <Bot size={18} aria-hidden="true" />
    <div className={styles.statusText}><strong>{task.input.title}</strong><span role="status">{message}</span>{error && <span role="alert" className={styles.warning}>{error}</span>}</div>
    <div className={styles.actions}>
      {!busy && !ended && <button type="button" onClick={() => void task.resume()}><RefreshCw size={16} />{hasAnswer ? '重试发送回答' : task.dispatched ? '检查结果' : '重试'}</button>}
      {!busy && !ended && <button type="button" onClick={() => void task.show().catch(failure => setError(String(failure?.message || failure)))}><ExternalLink size={16} />打开 AI 网页</button>}
      {!ended && phase !== 'delivering' && <button type="button" title="停止处理" aria-label="停止处理" onClick={() => void task.cancel()}><Square size={16} /></button>}
      {ended && <button type="button" title="收起" aria-label="收起群聊 AI 状态" onClick={() => setDismissed(task.operation)}><Check size={16} /></button>}
    </div>
  </section>
}
