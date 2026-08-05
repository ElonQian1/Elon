import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import {
  ArrowRight,
  ChevronRight,
  CircleCheck,
  Clock3,
  FileCheck2,
  History,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
  TriangleAlert,
  WalletCards,
} from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import {
  computeSettlementApi,
  type PlatformSettlementAccount,
  type SettlementReleaseBatchReport,
  type SettlementReleaseBatchHistoryPage,
  type SettlementReleaseCandidatePage,
  type SettlementWithdrawalRequest,
  type SettlementWithdrawalQueuePage,
  type TerminalizeSettlementWithdrawalBody,
  type WithdrawalStatus,
} from './computeSettlementApi'
import WithdrawalTerminalDialog from './WithdrawalTerminalDialog'
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
  const [dueCursor, setDueCursor] = useState<string | null>(null)
  const [batchHistory, setBatchHistory] = useState<SettlementReleaseBatchHistoryPage | null>(null)
  const [batchCursor, setBatchCursor] = useState<string | null>(null)
  const [withdrawals, setWithdrawals] = useState<SettlementWithdrawalQueuePage | null>(null)
  const [withdrawalStatus, setWithdrawalStatus] = useState<WithdrawalStatus>('pending')
  const [loading, setLoading] = useState(false)
  const [releasing, setReleasing] = useState(false)
  const [terminalizing, setTerminalizing] = useState(false)
  const [selectedWithdrawal, setSelectedWithdrawal] = useState<SettlementWithdrawalRequest | null>(null)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const releaseAttempt = useRef<{ cursor: string | null; idempotencyKey: string } | null>(null)

  const load = useCallback(async () => {
    if (!isAdmin) return
    setLoading(true)
    setError('')
    try {
      const [nextAccount, nextDue, nextHistory, nextWithdrawals] = await Promise.all([
        computeSettlementApi.platformAccount(),
        computeSettlementApi.dueReleases(50, dueCursor),
        computeSettlementApi.releaseBatchHistory(20, batchCursor),
        computeSettlementApi.withdrawals(withdrawalStatus),
      ])
      setAccount(nextAccount)
      setDue(nextDue)
      setBatchHistory(nextHistory)
      setWithdrawals(nextWithdrawals)
    } catch (reason) {
      setError(messageOf(reason, '结算数据读取失败'))
    } finally {
      setLoading(false)
    }
  }, [batchCursor, dueCursor, isAdmin, withdrawalStatus])

  useEffect(() => {
    void load()
  }, [load])

  const eligibleCount = useMemo(
    () => due?.candidates.filter((candidate) => candidate.eligible).length ?? 0,
    [due],
  )

  function refreshFromFirstPage() {
    releaseAttempt.current = null
    if (dueCursor || batchCursor) {
      setDueCursor(null)
      setBatchCursor(null)
      return
    }
    void load()
  }

  function changeDueCursor(cursor: string | null) {
    releaseAttempt.current = null
    setDueCursor(cursor)
  }

  async function releaseDue() {
    if (eligibleCount === 0 || releasing) return
    const confirmed = window.confirm(
      `将逐笔释放当前页 ${eligibleCount} 笔已到期结算到内部可用余额。继续吗？`,
    )
    if (!confirmed) return
    setReleasing(true)
    setError('')
    setNotice('')
    try {
      const attempt = releaseAttempt.current?.cursor === dueCursor
        ? releaseAttempt.current
        : {
          cursor: dueCursor,
          idempotencyKey: newIdempotencyKey(),
        }
      releaseAttempt.current = attempt
      const report = await computeSettlementApi.releaseDue(
        50,
        dueCursor,
        attempt.idempotencyKey,
      )
      releaseAttempt.current = null
      setNotice(batchMessage(report))
      if (dueCursor || batchCursor) {
        setDueCursor(null)
        setBatchCursor(null)
      } else await load()
    } catch (reason) {
      setError(messageOf(reason, '到期结算释放失败'))
    } finally {
      setReleasing(false)
    }
  }

  async function terminalizeWithdrawal(body: TerminalizeSettlementWithdrawalBody) {
    if (!selectedWithdrawal || terminalizing) return
    setTerminalizing(true)
    setError('')
    setNotice('')
    try {
      const terminal = await computeSettlementApi.terminalizeWithdrawal(
        selectedWithdrawal.withdrawal_id,
        body,
      )
      setNotice(terminal.action === 'rejected' ? '提款已拒绝，内部余额已退回。' : '外部付款证明已登记。')
      setSelectedWithdrawal(null)
      await load()
    } catch (reason) {
      setError(messageOf(reason, '提款处理失败'))
    } finally {
      setTerminalizing(false)
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
          <button type="button" className={styles.secondaryButton} onClick={refreshFromFirstPage} disabled={loading}>
            <RefreshCw size={15} className={loading ? styles.spinning : ''} aria-hidden="true" />
            刷新
          </button>
          <button type="button" className={styles.primaryButton} onClick={() => void releaseDue()} disabled={releasing || eligibleCount === 0}>
            <CircleCheck size={15} aria-hidden="true" />
            {releasing ? '正在释放' : `释放本页 ${eligibleCount} 笔`}
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
          <span className={styles.count}>
            {due ? `本页 ${due.candidates.length} / 共 ${due.total_due_candidates}` : '—'}
          </span>
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
        {(dueCursor || due?.has_more) && (
          <div className={styles.pageActions} aria-label="到期释放分页">
            {dueCursor && (
              <button type="button" onClick={() => changeDueCursor(null)} disabled={loading || releasing}>
                <RotateCcw size={14} aria-hidden="true" />
                返回第一页
              </button>
            )}
            {due?.has_more && due.next_cursor && (
              <button type="button" onClick={() => changeDueCursor(due.next_cursor ?? null)} disabled={loading || releasing}>
                下一页
                <ChevronRight size={14} aria-hidden="true" />
              </button>
            )}
          </div>
        )}
      </section>

      <section className={styles.section}>
        <header className={styles.sectionHeader}>
          <div><h2>释放批次历史</h2><p>批次意图和完成回执独立保存；未完成不代表余额未变化。</p></div>
          <History size={19} aria-hidden="true" />
        </header>
        <div className={styles.tableHeader} data-grid="batch">
          <span>批次</span><span>开始时间</span><span>候选页</span><span>扫描</span><span>结果</span><span>状态</span>
        </div>
        <div className={styles.rows}>
          {batchHistory?.items.map((item) => (
            <div className={styles.dataRow} data-grid="batch" key={item.batch_run_id}>
              <div><strong>{shortId(item.batch_run_id)}</strong><small>{shortDigest(item.report_digest ?? item.candidate_page_digest)}</small></div>
              <span>{formatTime(item.started_at)}</span>
              <span>{item.requested_cursor_present ? '续页' : '第一页'} · 共 {item.total_due_candidates}</span>
              <span>{item.scanned} / {item.eligible} 可释放</span>
              <span>{item.status === 'completed' ? `成功 ${item.released ?? 0} · 跳过 ${item.skipped ?? 0} · 失败 ${item.failed ?? 0}` : '等待完成回执'}</span>
              <span className={styles.status} data-tone={item.status === 'completed' ? 'clear' : 'pending'}>{item.status === 'completed' ? '已完成' : '未完成'}</span>
            </div>
          ))}
          {!loading && (batchHistory?.items.length ?? 0) === 0 && <EmptyRow icon={<History size={18} />} text="暂无释放批次" />}
        </div>
        {(batchCursor || batchHistory?.has_more) && (
          <div className={styles.pageActions} aria-label="释放批次历史分页">
            {batchCursor && (
              <button type="button" onClick={() => setBatchCursor(null)} disabled={loading || releasing}>
                <RotateCcw size={14} aria-hidden="true" />
                返回第一页
              </button>
            )}
            {batchHistory?.has_more && batchHistory.next_cursor && (
              <button type="button" onClick={() => setBatchCursor(batchHistory.next_cursor ?? null)} disabled={loading || releasing}>
                下一页
                <ChevronRight size={14} aria-hidden="true" />
              </button>
            )}
          </div>
        )}
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
          <span>Provider</span><span>金额</span><span>目标</span><span>状态</span><span>申请时间</span><span>操作</span>
        </div>
        <div className={styles.rows}>
          {withdrawals?.items.map((item) => (
            <div className={styles.dataRow} data-grid="withdrawal" key={item.request.withdrawal_id}>
              <div><strong>{shortId(item.request.provider_id)}</strong><small>{shortId(item.request.withdrawal_id)}</small></div>
              <strong>{formatCny(item.request.amount_micros)}</strong>
              <span>{destinationLabel(item.request.destination_kind)}</span>
              <span className={styles.status} data-tone={withdrawalTone(item.status)}>{withdrawalLabel(item.status)}</span>
              <span>{formatTime(item.request.requested_at)}</span>
              <span>
                {item.status === 'pending' ? (
                  <button
                    type="button"
                    className={styles.rowAction}
                    onClick={() => setSelectedWithdrawal(item.request)}
                    aria-label={`处理提款 ${shortId(item.request.withdrawal_id)}`}
                    title="处理提款"
                  >
                    <FileCheck2 size={15} aria-hidden="true" />
                  </button>
                ) : '—'}
              </span>
            </div>
          ))}
          {!loading && (withdrawals?.items.length ?? 0) === 0 && <EmptyRow icon={<WalletCards size={18} />} text="当前筛选下没有提款记录" />}
        </div>
      </section>

      {selectedWithdrawal && (
        <WithdrawalTerminalDialog
          request={selectedWithdrawal}
          busy={terminalizing}
          error={error}
          onClose={() => setSelectedWithdrawal(null)}
          onSubmit={terminalizeWithdrawal}
        />
      )}
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
  const replay = report.replayed ? '（幂等重放）' : ''
  return `批次 ${shortId(report.batch_run_id)}${replay}：已释放 ${report.released.length} 笔，跳过 ${report.skipped.length} 笔，失败 ${report.failed.length} 笔。`
}

function newIdempotencyKey() {
  const suffix = typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`
  return `pc-release-batch:${suffix}`
}

function messageOf(reason: unknown, fallback: string) {
  if (reason instanceof Error && reason.message) return reason.message
  if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message
  return fallback
}
