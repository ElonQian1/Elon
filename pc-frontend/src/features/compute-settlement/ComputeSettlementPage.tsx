import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import {
  ArrowRight,
  CircleCheck,
  Clock3,
  RefreshCw,
  ShieldCheck,
  TriangleAlert,
  WalletCards,
} from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import {
  computeSettlementApi,
  type PlatformSettlementAccount,
  type SettlementReleaseBatchReport,
  type SettlementReleaseCandidatePage,
  type SettlementWithdrawalQueuePage,
  type WithdrawalStatus,
} from './computeSettlementApi'
import styles from './ComputeSettlementPage.module.css'

const WITHDRAWAL_FILTERS: Array<{ value: WithdrawalStatus; label: string }> = [
  { value: 'pending', label: '待处理' },
  { value: 'external_paid_attested', label: '已登记付款' },
  { value: 'cancelled', label: '已取消' },
  { value: 'rejected', label: '已拒绝' },
  { value: 'all', label: '全部' },
]

export default function ComputeSettlementPage() {
  const user = useAuthStore((state) => state.user)
  const isAdmin = user?.role === 'admin' || user?.role === 'owner'
  const [account, setAccount] = useState<PlatformSettlementAccount | null>(null)
  const [due, setDue] = useState<SettlementReleaseCandidatePage | null>(null)
  const [withdrawals, setWithdrawals] = useState<SettlementWithdrawalQueuePage | null>(null)
  const [withdrawalStatus, setWithdrawalStatus] = useState<WithdrawalStatus>('pending')
  const [loading, setLoading] = useState(false)
  const [releasing, setReleasing] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const load = useCallback(async () => {
    if (!isAdmin) return
    setLoading(true)
    setError('')
    try {
      const [nextAccount, nextDue, nextWithdrawals] = await Promise.all([
        computeSettlementApi.platformAccount(),
        computeSettlementApi.dueReleases(),
        computeSettlementApi.withdrawals(withdrawalStatus),
      ])
      setAccount(nextAccount)
      setDue(nextDue)
      setWithdrawals(nextWithdrawals)
    } catch (reason) {
      setError(messageOf(reason, '结算数据读取失败'))
    } finally {
      setLoading(false)
    }
  }, [isAdmin, withdrawalStatus])

  useEffect(() => {
    void load()
  }, [load])

  const eligibleCount = useMemo(
    () => due?.candidates.filter((candidate) => candidate.eligible).length ?? 0,
    [due],
  )

  async function releaseDue() {
    if (eligibleCount === 0 || releasing) return
    const confirmed = window.confirm(
      `将逐笔释放 ${eligibleCount} 笔已到期结算到内部可用余额。继续吗？`,
    )
    if (!confirmed) return
    setReleasing(true)
    setError('')
    setNotice('')
    try {
      const report = await computeSettlementApi.releaseDue()
      setNotice(batchMessage(report))
      await load()
    } catch (reason) {
      setError(messageOf(reason, '到期结算释放失败'))
    } finally {
      setReleasing(false)
    }
  }

  if (!isAdmin) {
    return (
      <main className={styles.denied}>
        <ShieldCheck size={24} aria-hidden="true" />
        <h1>需要平台管理员权限</h1>
        <p>当前账号不能查看算力结算账本。</p>
      </main>
    )
  }

  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <div>
          <span className={styles.eyebrow}>算力市场账本</span>
          <h1>算力结算</h1>
          <p>核对平台收益、释放到期结算并查看 Provider 提款状态。</p>
        </div>
        <div className={styles.actions}>
          <button type="button" className={styles.secondaryButton} onClick={() => void load()} disabled={loading}>
            <RefreshCw size={15} className={loading ? styles.spinning : ''} aria-hidden="true" />
            刷新
          </button>
          <button type="button" className={styles.primaryButton} onClick={() => void releaseDue()} disabled={releasing || eligibleCount === 0}>
            <CircleCheck size={15} aria-hidden="true" />
            {releasing ? '正在释放' : `释放 ${eligibleCount} 笔`}
          </button>
        </div>
      </header>

      {error && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
      {notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}

      <section className={styles.ledgerBand} aria-label="平台结算账户">
        <div className={styles.balanceBlock} data-tone="pending">
          <span>待结算</span>
          <strong>{formatCny(account?.pending_micros)}</strong>
          <small>{account?.settlement_posting_count ?? 0} 笔结算入账</small>
        </div>
        <div className={styles.flowMarker} aria-hidden="true">
          <span>{formatCny(account?.corrected_margin_micros)} 已纠正</span>
          <ArrowRight size={18} />
          <span>{formatCny(account?.released_margin_micros)} 已释放</span>
        </div>
        <div className={styles.balanceBlock} data-tone="available">
          <span>内部可用</span>
          <strong>{formatCny(account?.available_micros)}</strong>
          <small>修订 {account?.account_revision ?? 0}</small>
        </div>
        <div className={styles.auditBlock}>
          <ShieldCheck size={18} aria-hidden="true" />
          <div><strong>账本已核对</strong><span>{shortDigest(account?.projection_digest)}</span></div>
        </div>
      </section>

      <section className={styles.section}>
        <header className={styles.sectionHeader}>
          <div><h2>到期释放</h2><p>每笔独立处理，挑战中的结算保持阻断。</p></div>
          <span className={styles.count}>{due?.candidates.length ?? 0}</span>
        </header>
        <div className={styles.tableHeader} data-grid="release">
          <span>结算</span><span>到期时间</span><span>挑战状态</span><span>处理条件</span>
        </div>
        <div className={styles.rows}>
          {due?.candidates.map((candidate) => (
            <div className={styles.dataRow} data-grid="release" key={candidate.settlement_receipt_id}>
              <div><strong>{shortId(candidate.settlement_receipt_id)}</strong><small>{shortId(candidate.lease_id)}</small></div>
              <span>{formatTime(candidate.challenge_deadline)}</span>
              <span className={styles.status} data-tone={candidate.challenge_gate.blocked ? 'blocked' : 'clear'}>{challengeLabel(candidate.challenge_gate.status)}</span>
              <span className={styles.status} data-tone={candidate.eligible ? 'clear' : 'blocked'}>{candidate.eligible ? '可释放' : '已阻断'}</span>
            </div>
          ))}
          {!loading && (due?.candidates.length ?? 0) === 0 && <EmptyRow icon={<Clock3 size={18} />} text="暂无到期结算" />}
        </div>
      </section>

      <section className={styles.section}>
        <header className={styles.sectionHeader}>
          <div><h2>Provider 提款</h2><p>这里只展示内部冻结及外部付款声明。</p></div>
          <WalletCards size={19} aria-hidden="true" />
        </header>
        <div className={styles.tabs} role="tablist" aria-label="提款状态">
          {WITHDRAWAL_FILTERS.map((filter) => (
            <button
              type="button"
              role="tab"
              aria-selected={withdrawalStatus === filter.value}
              className={withdrawalStatus === filter.value ? styles.activeTab : ''}
              key={filter.value}
              onClick={() => setWithdrawalStatus(filter.value)}
            >
              {filter.label}
            </button>
          ))}
        </div>
        <div className={styles.tableHeader} data-grid="withdrawal">
          <span>Provider</span><span>金额</span><span>目标</span><span>状态</span><span>申请时间</span>
        </div>
        <div className={styles.rows}>
          {withdrawals?.items.map((item) => (
            <div className={styles.dataRow} data-grid="withdrawal" key={item.request.withdrawal_id}>
              <div><strong>{shortId(item.request.provider_id)}</strong><small>{shortId(item.request.withdrawal_id)}</small></div>
              <strong>{formatCny(item.request.amount_micros)}</strong>
              <span>{destinationLabel(item.request.destination_kind)}</span>
              <span className={styles.status} data-tone={withdrawalTone(item.status)}>{withdrawalLabel(item.status)}</span>
              <span>{formatTime(item.request.requested_at)}</span>
            </div>
          ))}
          {!loading && (withdrawals?.items.length ?? 0) === 0 && <EmptyRow icon={<WalletCards size={18} />} text="当前筛选下没有提款记录" />}
        </div>
      </section>
    </main>
  )
}

function EmptyRow({ icon, text }: { icon: ReactNode; text: string }) {
  return <div className={styles.empty}>{icon}<span>{text}</span></div>
}

function formatCny(micros?: number | null) {
  const amount = typeof micros === 'number' && Number.isFinite(micros) ? micros : 0
  const value = amount / 1_000_000
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY' }).format(value)
}

function formatTime(value?: string | null) {
  if (!value) return '—'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false })
}

function shortId(value?: string | null) {
  if (!value) return '—'
  return value.length <= 20 ? value : `${value.slice(0, 10)}…${value.slice(-6)}`
}

function shortDigest(value?: string | null) {
  if (!value) return '等待账本数据'
  return `${value.slice(0, 12)}…${value.slice(-8)}`
}

function challengeLabel(status: string) {
  return ({ none: '无挑战', open: '挑战中', accepted: '待纠正', accepted_corrected: '已纠正', rejected: '已驳回', withdrawn: '已撤回' } as Record<string, string>)[status] ?? status
}

function withdrawalLabel(status: string) {
  return ({ pending: '待处理', cancelled: '已取消', rejected: '已拒绝', external_paid_attested: '已登记付款' } as Record<string, string>)[status] ?? status
}

function withdrawalTone(status: string) {
  if (status === 'pending') return 'pending'
  if (status === 'external_paid_attested') return 'clear'
  return 'muted'
}

function destinationLabel(kind: string) {
  return ({ bank_account_vault_ref: '银行金库引用', digital_wallet_vault_ref: '钱包金库引用', sui_address_ref: 'Sui 公开地址', other_vault_ref: '其他金库引用' } as Record<string, string>)[kind] ?? kind
}

function batchMessage(report: SettlementReleaseBatchReport) {
  return `已释放 ${report.released.length} 笔，跳过 ${report.skipped.length} 笔，失败 ${report.failed.length} 笔。`
}

function messageOf(reason: unknown, fallback: string) {
  if (reason instanceof Error && reason.message) return reason.message
  if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message
  return fallback
}
