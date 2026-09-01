import { useCallback, useEffect, useState } from 'react'
import { CircleDollarSign, RefreshCw, ShieldCheck } from 'lucide-react'

import {
  eskAssetApi,
  type EskAssetSnapshot,
  type EskSellbackRequest,
} from './eskAssetApi'
import styles from './EskAssetCard.module.css'

interface Props {
  initialSnapshot?: EskAssetSnapshot
  initialRequests?: EskSellbackRequest[]
  previewMode?: boolean
}

export default function EskAssetCard({ initialSnapshot, initialRequests, previewMode = false }: Props) {
  const [snapshot, setSnapshot] = useState<EskAssetSnapshot | null>(initialSnapshot ?? null)
  const [requests, setRequests] = useState<EskSellbackRequest[]>(initialRequests ?? [])
  const [amount, setAmount] = useState('')
  const [idempotencyKey, setIdempotencyKey] = useState(newIdempotencyKey)
  const [loading, setLoading] = useState(!initialSnapshot)
  const [working, setWorking] = useState<string | null>(null)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    if (previewMode) return
    setLoading(true)
    setError('')
    try {
      const [account, history] = await Promise.all([
        eskAssetApi.account(),
        eskAssetApi.sellbackRequests(),
      ])
      setSnapshot(account)
      setRequests(history.requests)
    } catch (reason) {
      setError(errorMessage(reason, '暂时无法读取 ESK 资产'))
    } finally {
      setLoading(false)
    }
  }, [previewMode])

  useEffect(() => { void load() }, [load])

  async function submitSellback(event: React.FormEvent) {
    event.preventDefault()
    const normalized = amount.trim()
    if (!/^\d+(\.\d{1,6})?$/.test(normalized) || /^0+(\.0+)?$/.test(normalized)) {
      setError('请输入大于 0、最多六位小数的 ESK 数量')
      return
    }
    setWorking('create')
    setMessage('')
    setError('')
    try {
      if (previewMode) {
        setMessage('预览模式：不会提交申请')
      } else {
        await eskAssetApi.createSellback(normalized, idempotencyKey)
        setAmount('')
        setIdempotencyKey(newIdempotencyKey())
        setMessage('卖回申请已提交；这不代表成交或付款。')
        await load()
      }
    } catch (reason) {
      setError(errorMessage(reason, '卖回申请提交失败'))
    } finally {
      setWorking(null)
    }
  }

  async function cancelSellback(requestId: string) {
    setWorking(requestId)
    setMessage('')
    setError('')
    try {
      if (previewMode) {
        setMessage('预览模式：不会撤销申请')
      } else {
        await eskAssetApi.cancelSellback(requestId)
        setMessage('卖回申请已撤销，冻结的 ESK 已恢复为可用。')
        await load()
      }
    } catch (reason) {
      setError(errorMessage(reason, '撤销卖回申请失败'))
    } finally {
      setWorking(null)
    }
  }

  if (loading && !snapshot) return <div className={styles.skeleton} role="status">正在读取 ESK 资产…</div>
  if (!snapshot) return (
    <div className={styles.unavailable} role="alert">
      <strong>ESK 资产暂不可用</strong>
      <span>{error || '服务器尚未返回资产资料。'}</span>
      <button type="button" onClick={() => void load()}><RefreshCw size={14} />重新读取</button>
    </div>
  )

  const canRequest = snapshot.mode === 'paper' && snapshot.sellback.request_enabled
  return (
    <div className={styles.card} data-esk-mode={snapshot.mode} data-chain-status={snapshot.asset.chain_status}>
      <div className={styles.hero}>
        <div className={styles.assetMark}><CircleDollarSign size={28} /></div>
        <div className={styles.identity}>
          <span>{snapshot.asset.name}</span>
          <strong>{snapshot.balance.total} <small>{snapshot.asset.symbol}</small></strong>
          <p>我的 ESK 总持有量</p>
        </div>
        <button className={styles.refresh} type="button" onClick={() => void load()} disabled={loading || previewMode} aria-label="刷新 ESK 资产">
          <RefreshCw size={16} className={loading ? styles.spinning : undefined} />
        </button>
      </div>

      <div className={styles.badges} aria-label="ESK 资产状态">
        <span className={styles.paperBadge}>Paper 登记</span>
        <span className={styles.chainBadge}>尚未上链</span>
        <span className={styles.safeBadge}><ShieldCheck size={13} />未发生资金划转</span>
      </div>

      <div className={styles.balanceGrid}>
        <div><span>当前可用</span><strong>{snapshot.balance.available} ESK</strong></div>
        <div><span>卖回申请冻结</span><strong>{snapshot.balance.reserved_for_sellback} ESK</strong></div>
      </div>

      <div className={styles.notice}>
        <strong>当前是可核对的 Paper 资产记录</strong>
        <p>{snapshot.status_message}</p>
        <p>未设置官方卖回价格；申请不代表成交或付款，提交后仍需官方另行审核和结算。</p>
      </div>

      <form className={styles.sellbackForm} onSubmit={submitSellback}>
        <div>
          <label htmlFor="esk-sellback-amount">申请卖回数量</label>
          <span>最多六位小数</span>
        </div>
        <div className={styles.formRow}>
          <input
            id="esk-sellback-amount"
            value={amount}
            onChange={(event) => setAmount(event.target.value)}
            placeholder="例如 100.000000"
            inputMode="decimal"
            autoComplete="off"
            disabled={!canRequest || working !== null}
          />
          <button type="submit" disabled={!canRequest || working !== null || !amount.trim()}>
            {working === 'create' ? '提交中…' : '申请卖回'}
          </button>
        </div>
        {!canRequest && <p className={styles.disabledHint}>ESK Paper 写入尚未启用，当前只能查看已登记余额。</p>}
      </form>

      {message && <p className={styles.success} role="status">{message}</p>}
      {error && <p className={styles.error} role="alert">{error}</p>}

      <div className={styles.history}>
        <div className={styles.historyTitle}>
          <strong>卖回申请记录</strong>
          <span>{requests.length} 条</span>
        </div>
        {requests.length === 0 ? (
          <p className={styles.empty}>暂无卖回申请</p>
        ) : requests.map((request) => (
          <div className={styles.request} key={request.request_id}>
            <div>
              <strong>{request.amount} ESK</strong>
              <span>{formatDateTime(request.submitted_at)} · {request.status === 'submitted' ? '已提交，等待处理' : '已撤销'}</span>
            </div>
            {request.status === 'submitted' && (
              <button type="button" onClick={() => void cancelSellback(request.request_id)} disabled={working !== null}>
                {working === request.request_id ? '撤销中…' : '撤销申请'}
              </button>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}

function newIdempotencyKey() {
  return `esk-sellback-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`}`
}

function errorMessage(reason: unknown, fallback: string) {
  return (reason as { message?: string } | null)?.message || fallback
}

function formatDateTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { dateStyle: 'short', timeStyle: 'short' })
}
