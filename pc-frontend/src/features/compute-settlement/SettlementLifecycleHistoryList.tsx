import { History } from 'lucide-react'
import { type ComputeSettlementLifecycleHistoryItem } from './computeSettlementLifecycleApi'
import { formatFen, formatMicros } from './settlementFormatting'
import styles from './ComputeSettlementChallengePage.module.css'

interface Props {
  items: ComputeSettlementLifecycleHistoryItem[]
  loading: boolean
}

const STATUS_LABELS: Record<ComputeSettlementLifecycleHistoryItem['lifecycle_status'], string> = {
  unchallenged_pending: '正常待结算',
  unchallenged_released: '正常已释放',
  open: '申诉待处理',
  withdrawn: '申诉已撤回',
  rejected: '申诉已驳回',
  accepted_pending_correction: '申诉待纠正',
  accepted_corrected: '已纠正待释放',
  withdrawn_released: '撤回后已释放',
  rejected_released: '驳回后已释放',
  accepted_corrected_released: '纠正后已释放',
}

export default function SettlementLifecycleHistoryList({ items, loading }: Props) {
  return (
    <section className={styles.queue} aria-label="结算历史">
      <header><div><History size={17} /><h2>结算历史</h2></div><span>{items.length}</span></header>
      {!loading && !items.length && <div className={styles.empty}>暂无结算历史</div>}
      <div className={styles.candidateList}>
        {items.map((item) => (
          <article className={styles.candidate} key={item.settlement.settlement.settlement_receipt_id}>
            <header>
              <div><b>{STATUS_LABELS[item.lifecycle_status]}</b><span>{formatTime(eventTime(item))}</span></div>
              <span className={styles.historyId}>{shortId(item.settlement.lease_id)}</span>
            </header>
            <div className={styles.facts}>
              <div><span>消费者扣结</span><strong>{formatFen(item.settlement.consumer_charged_fen)}</strong></div>
              <div><span>申诉</span><strong>{item.challenge?.reason_code ?? '无'}</strong></div>
              <div><span>纠正退款</span><strong>{formatFen(item.correction?.consumer_refund_fen ?? 0)}</strong></div>
              <div><span>Provider 释放</span><strong>{formatMicros(item.release?.provider_released_micros ?? 0)}</strong></div>
            </div>
            <div className={styles.effects}>
              <span>{balanceLabel(item.balance_status)}</span>
              <span>{item.correction ? '含金额纠正' : '原金额'}</span>
              <span>无外部付款证明</span>
            </div>
            <p className={styles.summary}>{historySummary(item)}</p>
            <code className={styles.digest}>{latestDigest(item)}</code>
          </article>
        ))}
      </div>
    </section>
  )
}

function historySummary(item: ComputeSettlementLifecycleHistoryItem) {
  return item.resolution?.statement
    ?? item.challenge?.summary
    ?? '该笔结算未发生申诉，按原金额进入结算生命周期。'
}

function eventTime(item: ComputeSettlementLifecycleHistoryItem) {
  return item.release?.released_at
    ?? item.correction?.corrected_at
    ?? item.resolution?.resolved_at
    ?? item.challenge?.opened_at
    ?? item.settlement.settled_at
}

function latestDigest(item: ComputeSettlementLifecycleHistoryItem) {
  return item.release?.event_digest
    ?? item.correction?.event_digest
    ?? item.resolution?.event_digest
    ?? item.challenge?.event_digest
    ?? item.settlement.event_digest
}

function balanceLabel(status: ComputeSettlementLifecycleHistoryItem['balance_status']) {
  switch (status) {
    case 'pending': return 'pending'
    case 'available': return 'available'
    case 'pending_blocked': return 'pending 已阻断'
    case 'release_pending': return '等待到期释放'
    case 'corrected_pending': return '纠正净额 pending'
    case 'corrected_available': return '纠正净额 available'
  }
}

function formatTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false })
}

function shortId(value: string) {
  return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}`
}
