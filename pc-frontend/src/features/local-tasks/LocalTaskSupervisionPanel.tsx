import { MonitorCheck } from 'lucide-react'
import type { LocalTaskSupervisionState } from './types'
import styles from './LocalTasksPage.module.css'

export default function LocalTaskSupervisionPanel({
  supervision,
}: {
  supervision: LocalTaskSupervisionState
}) {
  if (!supervision.enabled || !supervision.contract) return null

  const { contract, evidence, review } = supervision
  return (
    <section className={styles.supervisionCard} data-verdict={review?.verdict || 'observing'}>
      <div className={styles.sectionHeading}>
        <h3><MonitorCheck size={15} aria-hidden="true" />桌面监督闭环</h3>
        <span>{reviewLabel(review?.verdict)}</span>
      </div>
      <p className={styles.supervisionIntro}>
        PC 本机节点负责执行，Codex 桌面端依据任务记录、工具结果与产物独立验收。
      </p>
      <dl className={styles.supervisionMeta}>
        <div><dt>任务角色</dt><dd>{roleLabel(contract.task_role)}</dd></div>
        <div><dt>改进策略</dt><dd>{policyLabel(contract.improvement_policy)}</dd></div>
        <div><dt>工具调用</dt><dd>{evidence.tool_calls} 次</dd></div>
        <div><dt>失败工具</dt><dd>{evidence.failed_tools} 次</dd></div>
      </dl>
      <div className={styles.supervisionColumns}>
        <div>
          <strong>验收条件</strong>
          <ul>{contract.acceptance_criteria.map((item) => <li key={item}>{item}</li>)}</ul>
        </div>
        <div>
          <strong>监督证据</strong>
          <p>{evidence.event_count} 条事件 · {evidence.tool_results} 条工具结果 · {evidence.file_change_events} 条文件变更事件</p>
          {evidence.changed_files.length > 0 && <p>{evidence.changed_files.join('、')}</p>}
        </div>
      </div>
      {review && (
        <div className={styles.supervisionReview}>
          <strong>桌面验收：{reviewLabel(review.verdict)}</strong>
          <p>{review.summary || '监督者未填写补充说明。'}</p>
          {review.improvements.length > 0 && <ul>{review.improvements.map((item) => <li key={item}>{item}</li>)}</ul>}
        </div>
      )}
    </section>
  )
}

function reviewLabel(verdict?: string): string {
  const labels: Record<string, string> = {
    observing: '监督中', accepted: '验收通过', needs_follow_up: '需要跟进',
    blocked_capability: '能力阻塞', rejected: '验收未通过',
  }
  return labels[verdict || 'observing'] || verdict || '监督中'
}

function roleLabel(role: string): string {
  const labels: Record<string, string> = {
    requirement: '用户需求', capability_repair: '能力修复',
    resume_original: '恢复原任务', post_task_improvement: '任务后改进',
  }
  return labels[role] || role
}

function policyLabel(policy: string): string {
  const labels: Record<string, string> = {
    after_task_or_unblock: '任务后改进；阻塞时先修复',
    after_task_only: '完成任务后再改进', observe_only: '仅监督，不派生改进',
  }
  return labels[policy] || policy
}
