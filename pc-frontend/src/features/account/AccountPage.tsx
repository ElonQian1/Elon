import { useEffect, useState } from 'react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import UserAvatar from '../shell/UserAvatar'
import styles from './AccountPage.module.css'

interface Balance {
  billing_enabled?: boolean
  balance_fen?: number | null
  balance_yuan?: number | null
  this_month_cost_fen?: number | null
  this_month_cost_yuan?: number | null
  month_consumption_fen?: number | null
  month_consumption_yuan?: number | null
  trial_credit?: {
    amount_fen?: number | null
    amount_yuan?: number | null
    granted?: boolean
    granted_fen?: number | null
    granted_yuan?: number | null
    granted_at?: string | null
  } | null
}

interface BillingRecord {
  id?: string
  amount_fen?: number
  amount_yuan?: number
  description?: string
  created_at?: string
  type?: string
  model?: string | null
  input_tokens?: number
  cached_input_tokens?: number
  output_tokens?: number
  cost_rmb_fen?: number
}

export default function AccountPage() {
  const user = useAuthStore((s) => s.user)
  const fetchMe = useAuthStore((s) => s.fetchMe)

  const [nickname, setNickname] = useState(user?.nickname ?? '')
  const [savingNickname, setSavingNickname] = useState(false)
  const [nicknameMsg, setNicknameMsg] = useState('')

  const [balance, setBalance] = useState<Balance | null>(null)
  const [billing, setBilling] = useState<BillingRecord[]>([])
  const [billingLoading, setBillingLoading] = useState(true)

  useEffect(() => {
    setNickname(user?.nickname ?? '')
  }, [user?.nickname])

  useEffect(() => {
    loadBalance()
    loadBilling()
  }, [])

  async function loadBalance() {
    try {
      const data = await api.get<Balance>('/api/me/balance')
      setBalance(data)
    } catch { /* ignore */ }
  }

  async function loadBilling() {
    setBillingLoading(true)
    try {
      const data = await api.get<{ events?: BillingRecord[], records?: BillingRecord[] }>('/api/me/billing?page=1&size=20')
      setBilling(data.events ?? data.records ?? [])
    } catch { /* ignore */ }
    finally { setBillingLoading(false) }
  }

  async function handleSaveNickname(e: React.FormEvent) {
    e.preventDefault()
    setSavingNickname(true)
    setNicknameMsg('')
    try {
      await api.put('/api/me/profile', { nickname: nickname.trim() || null })
      await fetchMe()
      setNicknameMsg('昵称已保存')
    } catch (err) {
      setNicknameMsg((err as { message?: string }).message ?? '保存失败')
    } finally {
      setSavingNickname(false)
    }
  }

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>账号设置</h1>
      </header>

      <div className={styles.sections}>
        {/* 账号信息 */}
        <section className={styles.section}>
          <h2 className={styles.sectionTitle}>账号信息</h2>
          <div className={styles.profileCard}>
            <UserAvatar user={user} size="panel" showStatus />
            <div className={styles.profileInfo}>
              <strong>{user?.nickname ?? user?.account}</strong>
              <span>{user?.account}</span>
              <span className={styles.userId}>ID：{user?.id}</span>
            </div>
          </div>
        </section>

        {/* 修改昵称 */}
        <section className={styles.section}>
          <h2 className={styles.sectionTitle}>修改昵称</h2>
          <form onSubmit={handleSaveNickname} className={styles.form}>
            <input
              className={styles.input}
              value={nickname}
              onChange={(e) => setNickname(e.target.value)}
              placeholder="输入昵称（留空则使用账号）"
              maxLength={30}
            />
            <button
              className={styles.saveBtn}
              type="submit"
              disabled={savingNickname}
            >
              {savingNickname ? '保存中…' : '保存'}
            </button>
          </form>
          {nicknameMsg && (
            <p className={[styles.msg, nicknameMsg.includes('失败') || nicknameMsg.includes('error') ? styles.msgError : styles.msgOk].join(' ')}>
              {nicknameMsg}
            </p>
          )}
        </section>

        {/* 余额 */}
        <section className={styles.section}>
          <h2 className={styles.sectionTitle}>余额 & 消费</h2>
          {balance === null ? (
            <p className={styles.loading}>读取中…</p>
          ) : (
            <div className={styles.balanceGrid}>
              <div className={styles.balanceCard}>
                <span>当前余额</span>
                <strong>
                  {balanceYuan(balance) != null
                    ? formatYuan(balanceYuan(balance))
                    : '—'}
                </strong>
              </div>
              {monthCostYuan(balance) != null && (
                <div className={styles.balanceCard}>
                  <span>本月消费</span>
                  <strong>{formatYuan(monthCostYuan(balance))}</strong>
                </div>
              )}
              {balance.trial_credit && (
                <div className={styles.balanceCard}>
                  <span>试用额度</span>
                  <strong>
                    {balance.trial_credit.granted
                      ? `已领 ${formatYuan(trialGrantedYuan(balance))}`
                      : `可领 ${formatYuan(trialAmountYuan(balance))}`}
                  </strong>
                  <small className={styles.balanceHint}>
                    {balance.trial_credit.granted
                      ? `自动领取于 ${formatDateTime(balance.trial_credit.granted_at)}`
                      : '首次 AI 调用会自动发放'}
                  </small>
                </div>
              )}
            </div>
          )}
        </section>

        {/* 账单记录 */}
        <section className={styles.section}>
          <h2 className={styles.sectionTitle}>最近账单</h2>
          {billingLoading ? (
            <p className={styles.loading}>读取中…</p>
          ) : billing.length === 0 ? (
            <p className={styles.empty}>暂无账单记录</p>
          ) : (
            <div className={styles.billingList}>
              {billing.map((r, i) => (
                <div key={r.id ?? i} className={styles.billingRow}>
                  <div className={styles.billingDesc}>
                    <strong>{billingDescription(r)}</strong>
                    {r.created_at && (
                      <span>{formatDateTime(r.created_at)}</span>
                    )}
                  </div>
                  <span className={[styles.billingAmount, billingAmountFen(r) >= 0 ? styles.positive : styles.negative].join(' ')}>
                    {formatSignedFen(billingAmountFen(r))}
                  </span>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  )
}

function balanceYuan(balance: Balance): number | null {
  if (balance.balance_yuan != null) return balance.balance_yuan
  if (balance.balance_fen != null) return balance.balance_fen / 100
  return null
}

function monthCostYuan(balance: Balance): number | null {
  if (balance.this_month_cost_yuan != null) return balance.this_month_cost_yuan
  if (balance.this_month_cost_fen != null) return balance.this_month_cost_fen / 100
  if (balance.month_consumption_yuan != null) return balance.month_consumption_yuan
  if (balance.month_consumption_fen != null) return balance.month_consumption_fen / 100
  return null
}

function trialAmountYuan(balance: Balance): number | null {
  const trial = balance.trial_credit
  if (!trial) return null
  if (trial.amount_yuan != null) return trial.amount_yuan
  if (trial.amount_fen != null) return trial.amount_fen / 100
  return null
}

function trialGrantedYuan(balance: Balance): number | null {
  const trial = balance.trial_credit
  if (!trial) return null
  if (trial.granted_yuan != null) return trial.granted_yuan
  if (trial.granted_fen != null) return trial.granted_fen / 100
  return trialAmountYuan(balance)
}

function formatYuan(value: number | null | undefined) {
  return value == null ? '¥ 0.00' : `¥ ${value.toFixed(2)}`
}

function billingAmountFen(record: BillingRecord) {
  if (record.cost_rmb_fen != null) return -Math.abs(record.cost_rmb_fen)
  if (record.amount_fen != null) return record.amount_fen
  if (record.amount_yuan != null) return Math.round(record.amount_yuan * 100)
  return 0
}

function formatSignedFen(fen: number) {
  const sign = fen >= 0 ? '+' : '-'
  return `${sign}¥ ${Math.abs(fen / 100).toFixed(2)}`
}

function billingDescription(record: BillingRecord) {
  if (record.description) return record.description
  if (record.type) return record.type
  const totalTokens = (record.input_tokens ?? 0) + (record.cached_input_tokens ?? 0) + (record.output_tokens ?? 0)
  const model = record.model || 'AI 调用'
  return totalTokens > 0 ? `${model} · ${totalTokens} tokens` : model
}

function formatDateTime(value?: string | null) {
  if (!value) return '未知时间'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString('zh-CN', { dateStyle: 'short', timeStyle: 'short' })
}
