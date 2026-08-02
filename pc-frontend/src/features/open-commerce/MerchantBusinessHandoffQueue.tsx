import { useCallback, useEffect, useState } from 'react'
import { ArrowRightLeft, ListTodo, RefreshCw } from 'lucide-react'
import { openCommerceApi } from './openCommerceApi'
import type {
  MerchantBusinessEvidenceSummary,
  OpenCommerceBusinessHandoffQueue,
  OpenCommerceBusinessHandoffQueueState,
  OpenCommerceIntegration,
} from './openCommerceTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle } from './openCommerceStyles'
import styles from './MerchantBusinessHandoffQueue.module.css'

type QueueFilter = 'all' | OpenCommerceBusinessHandoffQueueState

type Props = {
  projectId: string
  merchantId: string
  integrations: OpenCommerceIntegration[]
  canEdit: boolean
  revision: number
  onSelect: (evidence: MerchantBusinessEvidenceSummary) => void
}

const filters: Array<{ value: QueueFilter; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'pending', label: '待处理' },
  { value: 'retry_required', label: '需重试' },
]

export default function MerchantBusinessHandoffQueue({
  projectId,
  merchantId,
  integrations,
  canEdit,
  revision,
  onSelect,
}: Props) {
  const [filter, setFilter] = useState<QueueFilter>('all')
  const [queue, setQueue] = useState<OpenCommerceBusinessHandoffQueue | null>(null)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    if (!projectId || !merchantId) return
    setBusy(true)
    setMessage('')
    try {
      setQueue(await openCommerceApi.listBusinessHandoffQueue(
        projectId,
        merchantId,
        filter === 'all' ? undefined : filter,
      ))
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }, [filter, merchantId, projectId])

  useEffect(() => {
    void refresh()
  }, [refresh, revision])

  const hasIntegration = integrations.some(
    (item) => item.merchant_id === merchantId && item.status !== 'disabled',
  )

  return (
    <section className={base.integrationSection}>
      <header>
        <span>
          <strong><ListTodo size={14} />待衔接任务</strong>
          <small>由业务证据与最新回执实时计算；成功或忽略后自动移出，处理失败会保留重试。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新待衔接任务">
          <RefreshCw size={14} />
        </button>
      </header>
      <div className={styles.body}>
        <div className={styles.toolbar}>
          <div className={styles.filters} aria-label="待衔接任务筛选">
            {filters.map((item) => (
              <button
                key={item.value}
                type="button"
                aria-pressed={filter === item.value}
                disabled={busy}
                onClick={() => setFilter(item.value)}
              >
                {item.label}
              </button>
            ))}
          </div>
          <span className={styles.counts}>
            待处理 {queue?.returned_pending_count ?? 0} · 需重试 {queue?.returned_retry_required_count ?? 0}
          </span>
        </div>
        <div className={styles.list}>
          {(queue?.items ?? []).map((item) => (
            <article className={styles.item} key={item.evidence.invocation_id}>
              <header>
                <span>
                  <strong>{item.evidence.capability_key}</strong>
                  <small>{item.evidence.business_receipt?.reference_id ?? `调用 ${item.evidence.invocation_id.slice(0, 12)}…`}</small>
                </span>
                <span style={badgeStyle(item.queue_state === 'pending' ? 'warn' : 'danger')}>
                  {item.queue_state === 'pending' ? '待处理' : '需重试'}
                </span>
              </header>
              <p>
                {item.queue_state === 'retry_required'
                  ? `上次失败：${item.latest_receipt?.error_code ?? '未提供原因'}`
                  : item.can_apply ? '有效业务回执，可记录真实 ERP/CRM 处理结果。' : '仅有结果摘要，可记录忽略或失败，不能声明已入库。'}
              </p>
              <footer>
                <small>结果摘要 {item.evidence.result_sha256?.slice(0, 12)}… · 未扣真实资金</small>
                {canEdit && hasIntegration && (
                  <button style={actionStyle('secondary', busy)} type="button" onClick={() => onSelect(item.evidence)} disabled={busy}>
                    <ArrowRightLeft size={13} />处理
                  </button>
                )}
              </footer>
            </article>
          ))}
        </div>
        {(queue?.items.length ?? 0) === 0 && !message && (
          <p className={base.empty}>当前筛选下没有待衔接任务。</p>
        )}
        {queue?.has_more && <p className={styles.boundary}>当前仅显示最近 100 条，请先处理当前任务后继续刷新。</p>}
        {!hasIntegration && canEdit && <p className={styles.boundary}>请先登记可用的商户接入器，再记录真实处理结果。</p>}
        {message && <p className={styles.error}>{message}</p>}
      </div>
    </section>
  )
}
