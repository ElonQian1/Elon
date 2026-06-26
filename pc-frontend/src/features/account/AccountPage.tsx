import { useEffect, useState } from 'react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import styles from './AccountPage.module.css'

interface Balance {
  balance_fen?: number
  balance_yuan?: number
  month_consumption_fen?: number
  month_consumption_yuan?: number
}

interface BillingRecord {
  id?: string
  amount_fen?: number
  amount_yuan?: number
  description?: string
  created_at?: string
  type?: string
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
  }, []) // eslint-disable-line

  async function loadBalance() {
    try {
      const data = await api.get<Balance>('/api/me/balance')
      setBalance(data)
    } catch { /* ignore */ }
  }

  async function loadBilling() {
    setBillingLoading(true)
    try {
      const data = await api.get<{ records?: BillingRecord[] }>('/api/me/billing?limit=20')
      setBilling(data.records ?? [])
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
            <div className={styles.avatar}>
              {(user?.nickname ?? user?.account ?? '?')[0]?.toUpperCase()}
            </div>
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
                  {balance.balance_yuan != null
                    ? `¥ ${balance.balance_yuan.toFixed(2)}`
                    : '—'}
                </strong>
              </div>
              {balance.month_consumption_yuan != null && (
                <div className={styles.balanceCard}>
                  <span>本月消费</span>
                  <strong>¥ {balance.month_consumption_yuan.toFixed(2)}</strong>
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
                    <strong>{r.description ?? r.type ?? '消费记录'}</strong>
                    {r.created_at && (
                      <span>{new Date(r.created_at).toLocaleString('zh-CN', { dateStyle: 'short', timeStyle: 'short' })}</span>
                    )}
                  </div>
                  <span className={[styles.billingAmount, (r.amount_fen ?? 0) >= 0 ? styles.positive : styles.negative].join(' ')}>
                    {(r.amount_fen ?? 0) >= 0 ? '+' : ''}
                    {r.amount_yuan != null ? `¥ ${r.amount_yuan.toFixed(2)}` : `${r.amount_fen ?? 0} fen`}
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
