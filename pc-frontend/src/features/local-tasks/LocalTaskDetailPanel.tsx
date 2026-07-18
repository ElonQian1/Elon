import { AlertTriangle, Check, CircleStop, Clock3, ShieldQuestion, X } from 'lucide-react'
import { readableTaskTitle } from '../../lib/taskTitle'
import {
  localTaskStatus,
  syncStateLabel,
} from './localTaskModel'
import type {
  LocalTaskApproval,
  LocalTaskApprovalDecision,
  LocalTaskDetail,
  LocalTaskEvent,
} from './types'
import styles from './LocalTasksPage.module.css'
import LocalTaskSupervisionPanel from './LocalTaskSupervisionPanel'

interface Props {
  detail: LocalTaskDetail | null
  loading: boolean
  actionKey: string
  onCancel: () => void
  onDecision: (approval: LocalTaskApproval, decision: LocalTaskApprovalDecision) => void
}

export default function LocalTaskDetailPanel({
  detail,
  loading,
  actionKey,
  onCancel,
  onDecision,
}: Props) {
  if (!detail) {
    return (
      <section className={styles.detailEmpty}>
        <Clock3 size={30} aria-hidden="true" />
        <strong>{loading ? '正在读取本机任务…' : '选择一个本机任务'}</strong>
        <span>任务进度、工具审批、最终回复和 Token 用量会保存在这里。</span>
      </section>
    )
  }

  const { task } = detail
  const status = localTaskStatus(task.status)
  return (
    <section className={styles.detailPane}>
      <header className={styles.detailHeader}>
        <div className={styles.detailTitle}>
          <div>
            <span className={styles.statusBadge} data-tone={status.tone}>{status.label}</span>
            <span className={styles.syncBadge}>{syncStateLabel(task.sync_state)}</span>
          </div>
          <h2>{readableTaskTitle(task.prompt)}</h2>
          <code>{task.id}</code>
        </div>
        {task.can_cancel && !status.terminal && (
          <button
            className={styles.dangerButton}
            type="button"
            onClick={onCancel}
            disabled={actionKey === 'cancel'}
          >
            <CircleStop size={15} aria-hidden="true" />
            {actionKey === 'cancel' ? '正在停止…' : '停止任务'}
          </button>
        )}
      </header>

      <div className={styles.detailScroll}>
        <section className={styles.summaryGrid}>
          <Summary label="运行环境" value={task.cli_name || 'codex'} />
          <Summary label="权限" value={permissionLabel(task.runtime_permission)} />
          <Summary label="输入 Token" value={formatNumber(task.token_usage.input_tokens)} />
          <Summary label="输出 Token" value={formatNumber(task.token_usage.output_tokens)} />
          <Summary label="Token 合计" value={formatNumber(task.token_usage.total_tokens)} />
          <Summary label="最后更新" value={formatTime(task.updated_at_ms || task.started_at_ms)} />
        </section>

        <section className={styles.infoCard}>
          <h3>任务内容</h3>
          <p className={styles.promptText}>{task.prompt || '节点未返回任务提示词。'}</p>
          <dl className={styles.taskMeta}>
            <div><dt>工作目录</dt><dd>{task.workspace_path || '-'}</dd></div>
            <div><dt>项目 / 频道</dt><dd>{task.project_id || '-'} / {task.channel_id || '联网后自动匹配 AI开发频道'}</dd></div>
            <div><dt>会话</dt><dd>{task.conversation_id || '-'}</dd></div>
          </dl>
        </section>

        <LocalTaskSupervisionPanel supervision={detail.supervision} />

        {detail.approvals.length > 0 && (
          <section className={styles.infoCard}>
            <h3>工具审批</h3>
            <div className={styles.approvalList}>
              {detail.approvals.map((approval) => (
                <div className={styles.approvalItem} key={approval.approval_id} data-actionable={approval.actionable}>
                  <ShieldQuestion size={18} aria-hidden="true" />
                  <div>
                    <strong>{approval.label || approval.tool || '工具操作'}</strong>
                    <span>{approval.tool || approval.approval_id} · {approval.meta || approval.status}</span>
                    {approval.checkpoint != null && (
                      <details>
                        <summary>查看参数</summary>
                        <pre>{formatJson(approval.checkpoint)}</pre>
                      </details>
                    )}
                  </div>
                  {approval.actionable ? (
                    <div className={styles.approvalActions}>
                      <button
                        type="button"
                        onClick={() => onDecision(approval, 'approve')}
                        disabled={Boolean(actionKey)}
                      >
                        <Check size={14} aria-hidden="true" />批准
                      </button>
                      <button
                        type="button"
                        data-tone="danger"
                        onClick={() => onDecision(approval, 'deny')}
                        disabled={Boolean(actionKey)}
                      >
                        <X size={14} aria-hidden="true" />拒绝
                      </button>
                    </div>
                  ) : <em>{approval.decision || approval.status || '不可操作'}</em>}
                </div>
              ))}
            </div>
          </section>
        )}

        {task.final_reply && (
          <section className={styles.infoCard}>
            <h3>最终回复</h3>
            <pre className={styles.finalReply}>{task.final_reply}</pre>
          </section>
        )}

        {task.error && (
          <section className={styles.errorCard}>
            <AlertTriangle size={17} aria-hidden="true" />
            <div><strong>任务异常</strong><p>{task.error}</p></div>
          </section>
        )}

        <section className={styles.infoCard}>
          <div className={styles.sectionHeading}>
            <h3>本机事件</h3>
            <span>{detail.events.length} 条 · 游标 {detail.last_event_seq}</span>
          </div>
          <div className={styles.eventList}>
            {detail.events.map((event, index) => (
              <EventRow event={event} key={event.seq || `${event.type}-${index}`} />
            ))}
            {!detail.events.length && <p className={styles.noEvents}>任务尚未产生可展示事件。</p>}
          </div>
        </section>
      </div>
    </section>
  )
}

function Summary({ label, value }: { label: string; value: string }) {
  return <div className={styles.summaryItem}><span>{label}</span><strong>{value}</strong></div>
}

function EventRow({ event }: { event: LocalTaskEvent }) {
  return (
    <article className={styles.eventRow} data-type={event.type}>
      <div className={styles.eventMeta}>
        <strong>{eventTypeLabel(event.type)}</strong>
        <span>{event.stream ? `${event.stream} · ` : ''}{formatTime(event.at_ms)}{event.seq ? ` · #${event.seq}` : ''}</span>
      </div>
      <pre>{event.text || formatJson(event.raw)}</pre>
    </article>
  )
}

function eventTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    started: '任务启动',
    cli_chunk: 'Codex 输出',
    assistant_message: 'Codex 消息',
    tool_call: '工具调用',
    tool_result: '工具结果',
    tool_approval_required: '等待工具审批',
    tool_approval_decision: '工具审批决定',
    usage: 'Token 用量',
    final_reply: '最终回复',
    done: '任务完成',
    failed: '任务失败',
  }
  return labels[type] ?? type
}

function permissionLabel(value: string): string {
  if (value === 'danger_full_access') return '完整本机命令行'
  if (value === 'full_access') return '完全访问'
  if (value === 'project_write') return '项目目录写入'
  return value || '-'
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat('zh-CN').format(value || 0)
}

function formatTime(value?: number): string {
  if (!value) return '-'
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit',
  }).format(new Date(value))
}

function formatJson(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}
