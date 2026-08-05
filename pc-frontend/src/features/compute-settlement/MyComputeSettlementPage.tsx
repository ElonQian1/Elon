import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import {
  CircleCheck,
  CircleDollarSign,
  Clock3,
  Plus,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
  TriangleAlert,
  WalletCards,
} from 'lucide-react'
import {
  type SettlementWithdrawalQueuePage,
  type SettlementWithdrawalRequest,
  type WithdrawalStatus,
} from './computeSettlementApi'
import {
  myComputeSettlementApi,
  type CreateMyWithdrawalBody,
  type CreateMyComputeProviderBody,
  type ComputeSettlementChallengeHistoryItem,
  type MyComputeProvider,
  type ProviderSettlementAccount,
} from './myComputeSettlementApi'
import CreateComputeProviderDialog from './CreateComputeProviderDialog'
import WithdrawalRequestDialog from './WithdrawalRequestDialog'
import SettlementChallengeHistoryList from './SettlementChallengeHistoryList'
import styles from './MyComputeSettlementPage.module.css'

const STATUS_FILTERS: Array<{ value: WithdrawalStatus; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'pending', label: '待处理' },
  { value: 'external_paid_attested', label: '已登记付款' },
  { value: 'cancelled', label: '已取消' },
  { value: 'rejected', label: '已拒绝' },
]

export default function MyComputeSettlementPage() {
  const [providers, setProviders] = useState<MyComputeProvider[]>([])
  const [providerId, setProviderId] = useState('')
  const [account, setAccount] = useState<ProviderSettlementAccount | null>(null)
  const [queue, setQueue] = useState<SettlementWithdrawalQueuePage | null>(null)
  const [challengeHistory, setChallengeHistory] = useState<ComputeSettlementChallengeHistoryItem[]>([])
  const [status, setStatus] = useState<WithdrawalStatus>('all')
  const [loadingProviders, setLoadingProviders] = useState(false)
  const [loadingAccount, setLoadingAccount] = useState(false)
  const [requestOpen, setRequestOpen] = useState(false)
  const [providerDialogOpen, setProviderDialogOpen] = useState(false)
  const [creatingProvider, setCreatingProvider] = useState(false)
  const [submitting, setSubmitting] = useState(false)
  const [cancellingId, setCancellingId] = useState('')
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const selectedProvider = useMemo(
    () => providers.find((provider) => provider.provider_id === providerId) ?? null,
    [providerId, providers],
  )

  const loadProviders = useCallback(async () => {
    setLoadingProviders(true)
    setError('')
    try {
      const nextProviders = await myComputeSettlementApi.providers()
      setProviders(nextProviders)
      setProviderId((current) => (
        nextProviders.some((provider) => provider.provider_id === current)
          ? current
          : nextProviders[0]?.provider_id ?? ''
      ))
    } catch (reason) {
      setError(messageOf(reason, 'Provider 列表读取失败'))
    } finally {
      setLoadingProviders(false)
    }
  }, [])

  const loadAccount = useCallback(async () => {
    if (!providerId) {
      setAccount(null)
      setQueue(null)
      setChallengeHistory([])
      return
    }
    setLoadingAccount(true)
    setError('')
    try {
      const [nextAccount, nextQueue, nextChallengeHistory] = await Promise.all([
        myComputeSettlementApi.account(providerId),
        myComputeSettlementApi.withdrawals(providerId, status),
        myComputeSettlementApi.challengeHistory(providerId),
      ])
      setAccount(nextAccount)
      setQueue(nextQueue)
      setChallengeHistory(nextChallengeHistory)
    } catch (reason) {
      setError(messageOf(reason, '算力收益读取失败'))
    } finally {
      setLoadingAccount(false)
    }
  }, [providerId, status])

  useEffect(() => { void loadProviders() }, [loadProviders])
  useEffect(() => { void loadAccount() }, [loadAccount])

  async function createWithdrawal(body: CreateMyWithdrawalBody) {
    if (!providerId || submitting) return
    setSubmitting(true)
    setError('')
    setNotice('')
    try {
      const request = await myComputeSettlementApi.createWithdrawal(providerId, body)
      setNotice(`提款申请 ${shortId(request.withdrawal_id)} 已登记，金额已转入内部 withdrawn。`)
      setRequestOpen(false)
      await loadAccount()
    } catch (reason) {
      setError(messageOf(reason, '提款申请失败'))
    } finally {
      setSubmitting(false)
    }
  }

  async function createProvider(body: CreateMyComputeProviderBody) {
    if (creatingProvider) return
    setCreatingProvider(true)
    setError('')
    setNotice('')
    try {
      const provider = await myComputeSettlementApi.createProvider(body)
      await loadProviders()
      setProviderId(provider.provider_id)
      setProviderDialogOpen(false)
      setNotice(`Provider“${provider.display_name}”已登记，当前状态为登记中。`)
    } catch (reason) {
      setError(messageOf(reason, 'Provider 登记失败'))
    } finally {
      setCreatingProvider(false)
    }
  }

  async function cancelWithdrawal(request: SettlementWithdrawalRequest) {
    if (!providerId || cancellingId) return
    if (!window.confirm(`取消 ${formatCny(request.amount_micros)} 提款申请并退回 available？`)) return
    setCancellingId(request.withdrawal_id)
    setError('')
    setNotice('')
    try {
      await myComputeSettlementApi.cancelWithdrawal(providerId, request.withdrawal_id, {
        expected_withdrawal_event_digest: request.event_digest,
        expected_request_posting_id: request.request_posting_id,
        expected_request_posting_digest: request.request_posting_digest,
        reason_code: 'provider_owner_cancelled_from_pc',
        reason_detail: 'Provider owner cancelled the pending withdrawal from the PC settlement page.',
        idempotency_key: `pc-provider-withdrawal-cancel:${request.event_digest}`,
        confirm_internal_refund_only: true,
      })
      setNotice('提款申请已取消，内部余额已退回 available。')
      await loadAccount()
    } catch (reason) {
      setError(messageOf(reason, '提款取消失败'))
    } finally {
      setCancellingId('')
    }
  }

  async function refresh() {
    await loadProviders()
    await loadAccount()
  }

  const loading = loadingProviders || loadingAccount

  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <div>
          <span className={styles.eyebrow}>Provider 账户</span>
          <h1>我的算力收益</h1>
          <p>查看内部结算余额与提款申请状态。</p>
        </div>
        <div className={styles.headerControls}>
          <label className={styles.providerSelect}>
            <span>Provider</span>
            <select value={providerId} onChange={(event) => setProviderId(event.target.value)} disabled={loadingProviders || providers.length === 0}>
              {providers.length === 0 && <option value="">暂无 Provider</option>}
              {providers.map((provider) => <option value={provider.provider_id} key={provider.provider_id}>{provider.display_name}</option>)}
            </select>
          </label>
          <button type="button" className={styles.iconButton} onClick={() => void refresh()} disabled={loading} aria-label="刷新" title="刷新">
            <RefreshCw size={16} className={loading ? styles.spinning : ''} aria-hidden="true" />
          </button>
          <button type="button" className={styles.secondaryButton} onClick={() => { setError(''); setProviderDialogOpen(true) }}>
            <Plus size={16} aria-hidden="true" />登记 Provider
          </button>
          <button type="button" className={styles.primaryButton} onClick={() => { setError(''); setRequestOpen(true) }} disabled={!account || account.available_micros <= 0}>
            <CircleDollarSign size={16} aria-hidden="true" />申请提款
          </button>
        </div>
      </header>

      {error && !requestOpen && !providerDialogOpen && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
      {notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}

      {providers.length === 0 && !loadingProviders ? (
        <section className={styles.emptyState}>
          <WalletCards size={24} aria-hidden="true" />
          <h2>尚未登记算力 Provider</h2>
          <p>账户建立后，结算收益和提款申请会显示在这里。</p>
          <button type="button" className={styles.emptyAction} onClick={() => { setError(''); setProviderDialogOpen(true) }}><Plus size={16} />登记 Provider</button>
        </section>
      ) : (
        <>
          <section className={styles.providerBand} aria-label="当前 Provider">
            <div><span>名称</span><strong>{selectedProvider?.display_name ?? '—'}</strong><small>{shortId(selectedProvider?.provider_id)}</small></div>
            <div><span>状态</span><strong>{providerStatusLabel(selectedProvider?.status)}</strong><small>{selectedProvider?.trust_tier ?? '—'}</small></div>
            <div><span>区域</span><strong>{selectedProvider?.home_region || '未指定'}</strong><small>策略修订 {selectedProvider?.policy_revision ?? 0}</small></div>
            <div className={styles.audit}><ShieldCheck size={17} /><div><strong>账本已核对</strong><small>{shortDigest(account?.projection_digest)}</small></div></div>
          </section>

          <section className={styles.balanceBand} aria-label="结算余额">
            <Balance label="待结算" value={account?.pending_micros} detail="挑战窗口内" tone="pending" />
            <Balance label="可申请" value={account?.available_micros} detail={`账户修订 ${account?.account_revision ?? 0}`} tone="available" />
            <Balance label="提款处理中" value={account?.withdrawn_micros} detail={`${account?.pending_terminal_count ?? 0} 笔待终态`} tone="withdrawn" />
            <Balance label="已退回" value={account?.returned_to_available_micros} detail="取消或拒绝" tone="returned" />
          </section>

          <SettlementChallengeHistoryList items={challengeHistory} loading={loadingAccount} />

          <section className={styles.section}>
            <header className={styles.sectionHeader}>
              <div><h2>提款记录</h2><p>外部付款证明由平台管理员登记。</p></div>
              <span>{queue?.items.length ?? 0}</span>
            </header>
            <div className={styles.tabs} role="tablist" aria-label="提款状态">
              {STATUS_FILTERS.map((filter) => (
                <button type="button" role="tab" aria-selected={status === filter.value} className={status === filter.value ? styles.activeTab : ''} onClick={() => setStatus(filter.value)} key={filter.value}>{filter.label}</button>
              ))}
            </div>
            <div className={styles.tableHeader}>
              <span>申请</span><span>金额</span><span>目标</span><span>状态</span><span>时间</span><span>操作</span>
            </div>
            <div className={styles.rows}>
              {queue?.items.map((item) => (
                <div className={styles.dataRow} key={item.request.withdrawal_id}>
                  <div><strong>{shortId(item.request.withdrawal_id)}</strong><small>{shortDigest(item.request.event_digest)}</small></div>
                  <strong>{formatCny(item.request.amount_micros)}</strong>
                  <span>{destinationLabel(item.request.destination_kind)}</span>
                  <span className={styles.status} data-tone={withdrawalTone(item.status)}>{withdrawalLabel(item.status)}</span>
                  <span>{formatTime(item.request.requested_at)}</span>
                  <span>
                    {item.status === 'pending' ? (
                      <button type="button" className={styles.cancelAction} onClick={() => void cancelWithdrawal(item.request)} disabled={Boolean(cancellingId)} aria-label="取消提款" title="取消提款">
                        <RotateCcw size={15} className={cancellingId === item.request.withdrawal_id ? styles.spinning : ''} aria-hidden="true" />
                      </button>
                    ) : '—'}
                  </span>
                </div>
              ))}
              {!loadingAccount && (queue?.items.length ?? 0) === 0 && <EmptyRow icon={<Clock3 size={18} />} text="当前筛选下没有提款记录" />}
            </div>
          </section>
        </>
      )}

      {requestOpen && account && (
        <WithdrawalRequestDialog
          availableMicros={account.available_micros}
          busy={submitting}
          error={error}
          onClose={() => setRequestOpen(false)}
          onSubmit={createWithdrawal}
        />
      )}
      {providerDialogOpen && (
        <CreateComputeProviderDialog
          busy={creatingProvider}
          error={error}
          onClose={() => setProviderDialogOpen(false)}
          onSubmit={createProvider}
        />
      )}
    </main>
  )
}

function Balance({ label, value, detail, tone }: { label: string; value?: number | null; detail: string; tone: string }) {
  return <div className={styles.balance} data-tone={tone}><span>{label}</span><strong>{formatCny(value)}</strong><small>{detail}</small></div>
}

function EmptyRow({ icon, text }: { icon: ReactNode; text: string }) {
  return <div className={styles.emptyRow}>{icon}<span>{text}</span></div>
}

function formatCny(micros?: number | null) {
  const amount = typeof micros === 'number' && Number.isFinite(micros) ? micros : 0
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY' }).format(amount / 1_000_000)
}

function formatTime(value?: string | null) {
  if (!value) return '—'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false })
}

function shortId(value?: string | null) {
  if (!value) return '—'
  return value.length <= 22 ? value : `${value.slice(0, 11)}…${value.slice(-7)}`
}

function shortDigest(value?: string | null) {
  if (!value) return '等待账本数据'
  return `${value.slice(0, 11)}…${value.slice(-7)}`
}

function providerStatusLabel(status?: string | null) {
  return ({ registering: '登记中', active: '已激活', quarantined: '已隔离', retired: '已退场' } as Record<string, string>)[status ?? ''] ?? status ?? '—'
}

function destinationLabel(kind: string) {
  return ({ bank_account_vault_ref: '银行金库', digital_wallet_vault_ref: '数字钱包', sui_address_ref: 'Sui 地址', other_vault_ref: '其他目标' } as Record<string, string>)[kind] ?? kind
}

function withdrawalLabel(status: string) {
  return ({ pending: '待处理', cancelled: '已取消', rejected: '已拒绝', external_paid_attested: '已登记付款' } as Record<string, string>)[status] ?? status
}

function withdrawalTone(status: string) {
  if (status === 'pending') return 'pending'
  if (status === 'external_paid_attested') return 'clear'
  return 'muted'
}

function messageOf(reason: unknown, fallback: string) {
  if (reason instanceof Error && reason.message) return reason.message
  if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message
  return fallback
}
