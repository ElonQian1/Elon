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
  LocalTaskContinuationInput,
} from './types'
import styles from './LocalTasksPage.module.css'
import LocalTaskSupervisionPanel from './LocalTaskSupervisionPanel'
import LocalTaskUpdateRecoveryPanel from './LocalTaskUpdateRecoveryPanel'
import LocalTaskContinuationPanel from './LocalTaskContinuationPanel'

interface Props {
  detail: LocalTaskDetail | null
  loading: boolean
  actionKey: string
  onCancel: () => void
  onDecision: (approval: LocalTaskApproval, decision: LocalTaskApprovalDecision) => void
  onContinue: (input: LocalTaskContinuationInput) => Promise<boolean>
}

export default function LocalTaskDetailPanel({
  detail,
  loading,
  actionKey,
  onCancel,
  onDecision,
  onContinue,
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
        <section className={styles.runtimeCard} data-phase={detail.runtime.phase}>
          <div className={styles.sectionHeading}>
            <h3>当前阶段 · {phaseLabel(detail.runtime.phase)}</h3>
            <span>心跳 {formatTime(detail.runtime.heartbeat)}</span>
          </div>
          <p className={styles.currentCommand}>
            {detail.runtime.current_command || phaseHint(detail.runtime.phase)}
          </p>
          <dl className={styles.runtimeMeta}>
            <div><dt>最近进展</dt><dd>{formatTime(detail.runtime.last_progress)}</dd></div>
            <div><dt>空闲</dt><dd>{formatDuration(detail.runtime.idle_duration)}</dd></div>
            <div><dt>总时限</dt><dd>{formatDuration(detail.runtime.timeout_policy.total_timeout_secs)}</dd></div>
            <div><dt>空闲策略</dt><dd>{detail.runtime.timeout_policy.progress_aware ? formatDuration(detail.runtime.timeout_policy.idle_timeout_secs) : '固定总时限'}</dd></div>
          </dl>
        </section>

        <section className={styles.summaryGrid}>
          <Summary label="运行环境" value={task.cli_name || 'codex'} />
          <Summary label="权限" value={permissionLabel(task.runtime_permission)} />
          <Summary label="输入 Token" value={formatNumber(task.token_usage.input_tokens)} />
          <Summary label="输出 Token" value={formatNumber(task.token_usage.output_tokens)} />
          <Summary label="Token 合计" value={formatNumber(task.token_usage.total_tokens)} />
          <Summary label="最后更新" value={formatTime(task.updated_at_ms || task.started_at_ms)} />
        </section>

        {detail.recovery_timing && (
          <section className={styles.infoCard} data-testid="recovery-timing">
            <div className={styles.sectionHeading}>
              <h3>{detail.recovery_timing.mode === 'supersede' ? '需求变更承接耗时' : '任务继续耗时'}</h3>
              <span data-tone={detail.recovery_timing.handoff_within_target === false ? 'danger' : 'done'}>
                交接目标 {formatMilliseconds(detail.recovery_timing.handoff_target_ms)}
              </span>
            </div>
            <dl className={styles.taskMeta}>
              <div><dt>恢复交接</dt><dd>{formatMilliseconds(detail.recovery_timing.handoff_ms)}</dd></div>
              <div><dt>继续后的开发</dt><dd>{formatMilliseconds(detail.recovery_timing.resumed_work_ms)}</dd></div>
              <div><dt>合计</dt><dd>{formatMilliseconds(detail.recovery_timing.total_since_parent_finished_ms)}</dd></div>
              <div><dt>父任务</dt><dd>{detail.recovery_timing.parent_task_id || '-'}</dd></div>
            </dl>
            <p className={styles.promptText}>恢复交接和后续开发分别统计，不再把编译、测试时间算成恢复耗时。</p>
          </section>
        )}

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
        <LocalTaskUpdateRecoveryPanel
          recovery={detail.update_recovery}
          resumeWorkspace={detail.resume_workspace_status}
        />
        <LocalTaskContinuationPanel detail={detail} busy={actionKey === 'continue'} onContinue={onContinue} />

        {detail.cancel_audit && (
          <section className={styles.infoCard} data-testid="cancel-audit">
            <h3>取消来源</h3>
            <dl className={styles.taskMeta}>
              <div><dt>请求者</dt><dd>{detail.cancel_audit.requested_by || '-'}</dd></div>
              <div><dt>入口</dt><dd>{detail.cancel_audit.source || 'legacy'}</dd></div>
              <div><dt>原因</dt><dd>{detail.cancel_audit.reason || '-'}</dd></div>
              <div><dt>中断来源</dt><dd>{detail.cancel_audit.interruption_source || 'legacy_unknown'}</dd></div>
              <div><dt>时间</dt><dd>{formatTime(detail.cancel_audit.requested_at_ms)}</dd></div>
            </dl>
          </section>
        )}

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

function phaseLabel(phase: string): string {
  return ({
    reasoning: '推理', command: '命令', editing: '文件修改', verification: '验证',
    approval: '等待审批', finalizing: '收尾', done: '完成', failed: '失败', canceled: '已取消',
  } as Record<string, string>)[phase] || phase || '推理'
}

function phaseHint(phase: string): string {
  if (phase === 'reasoning') return 'Codex 正在推理；节点心跳持续更新。'
  if (phase === 'approval') return '等待用户处理工具审批。'
  if (phase === 'finalizing') return '正在提交、发布或执行统一收尾。'
  return '节点正在处理当前阶段。'
}

function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '-'
  if (seconds < 60) return `${Math.floor(seconds)} 秒`
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟`
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  return minutes ? `${hours} 小时 ${minutes} 分钟` : `${hours} 小时`
}

function formatMilliseconds(milliseconds?: number): string {
  if (milliseconds == null || !Number.isFinite(milliseconds) || milliseconds < 0) return '-'
  if (milliseconds < 60_000) return `${Math.max(1, Math.round(milliseconds / 1_000))} 秒`
  const minutes = Math.floor(milliseconds / 60_000)
  const seconds = Math.round((milliseconds % 60_000) / 1_000)
  if (minutes < 60) return seconds ? `${minutes} 分 ${seconds} 秒` : `${minutes} 分钟`
  const hours = Math.floor(minutes / 60)
  const remainder = minutes % 60
  return remainder ? `${hours} 小时 ${remainder} 分钟` : `${hours} 小时`
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
