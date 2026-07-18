import { Check, CirclePause, CirclePlay, GitPullRequestArrow, RotateCcw } from 'lucide-react'
import type { GlobalPublishStatus, SelfEvolutionQueue } from './types'
import styles from './LocalTasksPage.module.css'

interface Props {
  evolution: SelfEvolutionQueue
  publish: GlobalPublishStatus
  actionKey: string
  onAction: (logicalId: string, action: 'pause' | 'resume' | 'approve' | 'reject') => void
}

export default function LocalOperationsPanel({ evolution, publish, actionKey, onAction }: Props) {
  const activeItems = evolution.items.filter((item) => item.status !== 'completed')
  return (
    <section className={styles.operationsStrip} aria-label="后台调度状态">
      <article className={styles.operationCard} data-active={activeItems.length > 0}>
        <header>
          <div><RotateCcw size={15} aria-hidden="true" /><strong>低优先自进化</strong></div>
          <span>{activeItems.length ? `${activeItems.length} 项` : '队列空闲'}</span>
        </header>
        <p>{gateSummary(evolution)}</p>
        <div className={styles.operationItems}>
          {activeItems.slice(0, 3).map((item) => (
            <div className={styles.operationItem} key={item.logical_id} data-status={item.status}>
              <div>
                <strong>{item.project_id} · 第 {item.generation || 0} 代</strong>
                <span>{evolutionStatus(item.status)}{item.pause_reason ? ` · ${pauseReason(item.pause_reason)}` : ''}</span>
                <code>{item.active_task_id || item.logical_id}</code>
              </div>
              <div className={styles.operationActions}>
                {matches(item.status, 'running', 'starting') && (
                  <button type="button" onClick={() => onAction(item.logical_id, 'pause')} disabled={Boolean(actionKey)}>
                    <CirclePause size={13} />暂停
                  </button>
                )}
                {matches(item.status, 'paused', 'failed') && (
                  <button type="button" onClick={() => onAction(item.logical_id, 'resume')} disabled={Boolean(actionKey)}>
                    <CirclePlay size={13} />继续
                  </button>
                )}
                {item.status === 'review_required' && (
                  <>
                    <button type="button" onClick={() => onAction(item.logical_id, 'approve')} disabled={Boolean(actionKey)}>
                      <Check size={13} />通过
                    </button>
                    <button type="button" onClick={() => onAction(item.logical_id, 'reject')} disabled={Boolean(actionKey)}>
                      退回
                    </button>
                  </>
                )}
              </div>
            </div>
          ))}
          {!activeItems.length && <span className={styles.operationEmpty}>用户任务结束并释放资源后，改进项才会进入独立会话和 worktree。</span>}
        </div>
      </article>

      <article className={styles.operationCard} data-active={Boolean(publish.owner)}>
        <header>
          <div><GitPullRequestArrow size={15} aria-hidden="true" /><strong>全局发布租约</strong></div>
          <span>{publish.owner ? '发布占用中' : '当前空闲'}</span>
        </header>
        <p>FIFO · 同 kind+SHA 合并 · release SHA 固定不可变</p>
        {publish.owner ? (
          <div className={styles.publishOwner}>
            <strong>{publish.owner.kind} · {publish.owner.builderLabel || '未知 builder'}</strong>
            <code>{shortSha(publish.owner.sha)}</code>
            <span>{publish.waiterCount} 个 waiter</span>
          </div>
        ) : <span className={styles.operationEmpty}>没有 owner；下一位 waiter 将按请求顺序获得租约。</span>}
        {publish.waiters.length > 0 && (
          <ol className={styles.publishWaiters}>
            {publish.waiters.slice(0, 3).map((waiter, index) => (
              <li key={waiter.token}><span>#{index + 1} {waiter.kind}</span><code>{shortSha(waiter.sha)}</code><em>{waiter.builderLabel}</em></li>
            ))}
          </ol>
        )}
      </article>
    </section>
  )
}

function gateSummary(queue: SelfEvolutionQueue): string {
  const { gates } = queue
  if (gates.foreground_task_ids.length) return `已让路：${gates.foreground_task_ids.length} 个前台用户任务正在运行`
  if (gates.publish_active) return `已让路：全局发布 owner ${gates.publish_owner || '未知'}`
  if (gates.update_active) return '已让路：Windows 节点正在更新或恢复'
  if (gates.resource_pressure) return '已让路：本机资源压力达到低优先任务阈值'
  if (gates.publish_status === 'unavailable') return '本机队列可用；云端发布状态暂不可见'
  return '资源空闲时自动启动；让路结束后自动恢复下一代'
}

function evolutionStatus(status: string): string {
  return ({
    queued: '排队中', starting: '正在启动', running: '低优先运行中',
    pause_requested: '正在保存现场并让路', paused: '已暂停，等待自动恢复',
    review_required: '待审查', completed: '审查通过', failed: '执行失败',
  } as Record<string, string>)[status] || status
}

function pauseReason(reason: string): string {
  return ({
    foreground_task: '前台任务优先', global_publish: '发布优先', node_update: '更新优先',
    resource_pressure: '资源压力', manual_pause: '手动暂停', node_restart: '节点重启',
    review_changes_requested: '审查退回',
  } as Record<string, string>)[reason] || reason
}

function matches(value: string, ...states: string[]): boolean {
  return states.includes(value)
}

function shortSha(value: string): string {
  return value ? value.slice(0, 12) : '-'
}
