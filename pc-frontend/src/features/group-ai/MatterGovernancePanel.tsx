import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { AlertTriangle, Check, GitMerge, Network, RefreshCw, ShieldCheck, Wallet, X } from 'lucide-react'
import { loadMatterGovernance, updateMatterMergeRequest } from './api'
import BudgetPolicyPanel from './BudgetPolicyPanel'
import MergeGatePanel from './MergeGatePanel'
import type { MatterGovernanceSummary, ProjectAiMergeRequest, ProjectAiReview } from './types'
import styles from './MatterGovernancePanel.module.css'

interface Props {
  projectId: string
  matterId: string
  refreshKey?: number
}

export default function MatterGovernancePanel({ projectId, matterId, refreshKey = 0 }: Props) {
  const [governance, setGovernance] = useState<MatterGovernanceSummary | null>(null)
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')

  async function refresh() {
    setBusy('refresh')
    setError('')
    try {
      const response = await loadMatterGovernance(projectId, matterId)
      setGovernance(response.governance)
    } catch (err) {
      setError((err as { message?: string }).message ?? '治理信息加载失败')
    } finally {
      setBusy('')
    }
  }

  useEffect(() => {
    void refresh()
  }, [projectId, matterId, refreshKey])

  async function setMergeStatus(request: ProjectAiMergeRequest, status: string) {
    setBusy(`${request.id}:${status}`)
    setError('')
    try {
      const response = await updateMatterMergeRequest(projectId, matterId, request.id, { status })
      setGovernance((current) => {
        if (!current) return current
        return {
          ...current,
          merge_requests: current.merge_requests.map((item) =>
            item.id === request.id ? response.merge_request : item,
          ),
        }
      })
    } catch (err) {
      setError((err as { message?: string }).message ?? '合并队列更新失败')
    } finally {
      setBusy('')
    }
  }

  const counts = useMemo(() => {
    const nodes = governance?.task_graph.nodes.length ?? 0
    const edges = governance?.task_graph.edges.length ?? 0
    const reviews = governance?.reviews.length ?? 0
    const merges = governance?.merge_requests.length ?? 0
    return { nodes, edges, reviews, merges }
  }, [governance])

  return (
    <section className={styles.panel}>
      <div className={styles.header}>
        <div>
          <h4>治理闭环</h4>
          <span>
            {counts.nodes} 节点 · {counts.edges} 依赖 · {counts.reviews} Review · {counts.merges} 合并项
          </span>
        </div>
        <button className={styles.iconButton} disabled={busy === 'refresh'} onClick={refresh} type="button">
          <RefreshCw size={14} />
        </button>
      </div>

      {error && <div className={styles.error}>{error}</div>}

      {governance ? (
        <div className={styles.content}>
          <SummaryStrip governance={governance} />
          <BudgetPolicyPanel
            governance={governance}
            matterId={matterId}
            onChanged={() => void refresh()}
            projectId={projectId}
          />
          <TaskGraph governance={governance} />
          <ReviewList reviews={governance.reviews} />
          <MergeQueue
            busy={busy}
            matterId={matterId}
            onChanged={() => void refresh()}
            requests={governance.merge_requests}
            projectId={projectId}
            onStatus={(request, status) => void setMergeStatus(request, status)}
          />
        </div>
      ) : (
        <div className={styles.empty}>正在加载治理信息</div>
      )}
    </section>
  )
}

function SummaryStrip({ governance }: { governance: MatterGovernanceSummary }) {
  const warnings = [...governance.policy.warnings, ...governance.budget.warnings]
  const budgetMax = governance.budget.max_billed_cost_rmb_fen
  const budgetDetail =
    typeof budgetMax === 'number'
      ? `${governance.budget.remaining_billed_cost_rmb_fen ?? 0} 分剩余`
      : `${governance.budget.compute_call_count} 次调用`
  return (
    <div className={styles.summaryGrid}>
      <Metric
        icon={<Wallet size={15} />}
        label="预算"
        value={`${governance.budget.billed_cost_rmb_fen} 分`}
        detail={budgetDetail}
      />
      <Metric
        icon={<ShieldCheck size={15} />}
        label="门禁"
        value={governance.review_gate.status}
        detail={`${governance.review_gate.passed_reviews} 通过 · ${governance.review_gate.blockers.length} 阻塞`}
      />
      <Metric
        icon={<AlertTriangle size={15} />}
        label="风险"
        value={`${warnings.length} 条`}
        detail={warnings[0] ?? '暂无策略警告'}
      />
    </div>
  )
}

function TaskGraph({ governance }: { governance: MatterGovernanceSummary }) {
  return (
    <div className={styles.block}>
      <BlockTitle icon={<Network size={15} />} title="任务图" />
      <div className={styles.nodeList}>
        {governance.task_graph.nodes.slice(0, 8).map((node) => (
          <div className={styles.node} key={node.id}>
            <span>{node.kind}</span>
            <strong>{node.label}</strong>
            <small>{node.status}</small>
          </div>
        ))}
      </div>
      {!governance.task_graph.nodes.length && <div className={styles.empty}>暂无任务节点</div>}
    </div>
  )
}

function ReviewList({ reviews }: { reviews: ProjectAiReview[] }) {
  return (
    <div className={styles.block}>
      <BlockTitle icon={<ShieldCheck size={15} />} title="结构化 Review" />
      {reviews.slice(0, 5).map((review) => (
        <div className={styles.review} key={review.id}>
          <div>
            <strong>{review.status}</strong>
            <span>{review.severity}</span>
          </div>
          <p>{reviewSummary(review)}</p>
          {review.target_assignment_id && <small>target: {review.target_assignment_id}</small>}
        </div>
      ))}
      {!reviews.length && <div className={styles.empty}>暂无 Review 结果</div>}
    </div>
  )
}

function MergeQueue({
  projectId,
  matterId,
  requests,
  busy,
  onStatus,
  onChanged,
}: {
  projectId: string
  matterId: string
  requests: ProjectAiMergeRequest[]
  busy: string
  onStatus: (request: ProjectAiMergeRequest, status: string) => void
  onChanged: () => void
}) {
  return (
    <div className={styles.block}>
      <BlockTitle icon={<GitMerge size={15} />} title="人工合并队列" />
      {requests.map((request) => (
        <div className={styles.mergeItem} key={request.id}>
          <div className={styles.mergeTop}>
            <strong>{request.branch_name || request.assignment_id}</strong>
            <span>{request.status}</span>
          </div>
          <p>{request.notes || request.worktree_path || '等待人工确认合并方式'}</p>
          <small>{request.review_status} · {request.risk_level} · {request.merge_strategy}</small>
          <div className={styles.mergeActions}>
            <QueueButton
              busy={busy === `${request.id}:approved`}
              disabled={request.status !== 'open'}
              icon={<Check size={13} />}
              label="批准"
              onClick={() => onStatus(request, 'approved')}
            />
            <QueueButton
              busy={busy === `${request.id}:rejected`}
              disabled={request.status === 'rejected'}
              icon={<X size={13} />}
              label="拒绝"
              onClick={() => onStatus(request, 'rejected')}
            />
          </div>
          <MergeGatePanel
            matterId={matterId}
            onChanged={onChanged}
            projectId={projectId}
            request={request}
          />
        </div>
      ))}
      {!requests.length && <div className={styles.empty}>暂无待合并产物</div>}
    </div>
  )
}

function Metric({ icon, label, value, detail }: {
  icon: ReactNode
  label: string
  value: string
  detail: string
}) {
  return (
    <div className={styles.metric}>
      {icon}
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </div>
  )
}

function BlockTitle({ icon, title }: { icon: ReactNode; title: string }) {
  return (
    <div className={styles.blockTitle}>
      {icon}
      <span>{title}</span>
    </div>
  )
}

function QueueButton({ icon, label, disabled, busy, onClick }: {
  icon: ReactNode
  label: string
  disabled?: boolean
  busy?: boolean
  onClick: () => void
}) {
  return (
    <button className={styles.queueButton} disabled={disabled || busy} onClick={onClick} type="button">
      {icon}
      {busy ? '处理中' : label}
    </button>
  )
}

function reviewSummary(review: ProjectAiReview) {
  const finding = review.finding ?? {}
  return (
    stringValue(finding, 'summary') ||
    stringValue(finding, 'finding') ||
    stringValue(finding, 'merge_recommendation') ||
    'Review 已记录，等待人工查看详情'
  )
}

function stringValue(record: Record<string, unknown>, key: string) {
  const value = record[key]
  return typeof value === 'string' && value.trim() ? value.trim() : ''
}
