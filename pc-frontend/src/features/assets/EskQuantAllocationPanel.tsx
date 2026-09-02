import { useState } from 'react'
import { FlaskConical, RotateCcw } from 'lucide-react'

import { eskAssetApi, type EskQuantAllocationRequest } from './eskAssetApi'
import styles from './EskQuantAllocationPanel.module.css'

interface Props {
  available: string
  enabled: boolean
  requests: EskQuantAllocationRequest[]
  previewMode: boolean
  onChanged: () => Promise<void>
}

export default function EskQuantAllocationPanel({ available, enabled, requests, previewMode, onChanged }: Props) {
  const [amount, setAmount] = useState('')
  const [idempotencyKey, setIdempotencyKey] = useState(newIdempotencyKey)
  const [working, setWorking] = useState<string | null>(null)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  async function submit(event: React.FormEvent) {
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
        setMessage('预览模式：不会提交量化分配申请')
      } else {
        await eskAssetApi.createQuantAllocation(normalized, idempotencyKey)
        setAmount('')
        setIdempotencyKey(newIdempotencyKey())
        setMessage('已占用对应 ESK；目前仍是 Paper 申请，尚未形成量化仓位。')
        await onChanged()
      }
    } catch (reason) {
      setError(errorMessage(reason, '量化分配申请提交失败'))
    } finally {
      setWorking(null)
    }
  }

  async function cancel(requestId: string) {
    setWorking(requestId)
    setMessage('')
    setError('')
    try {
      if (previewMode) {
        setMessage('预览模式：不会撤销量化分配申请')
      } else {
        await eskAssetApi.cancelQuantAllocation(requestId)
        setMessage('量化分配申请已撤销，占用的 ESK 已恢复为可用。')
        await onChanged()
      }
    } catch (reason) {
      setError(errorMessage(reason, '撤销量化分配申请失败'))
    } finally {
      setWorking(null)
    }
  }

  return (
    <section className={styles.panel} aria-labelledby="esk-quant-allocation-title">
      <div className={styles.heading}>
        <span className={styles.icon}><FlaskConical size={17} /></span>
        <div>
          <strong id="esk-quant-allocation-title">申请投入一龙量化</strong>
          <p>当前仅登记 Paper 意向，不转移资金、不创建仓位、不承诺收益。</p>
        </div>
      </div>

      <div className={styles.available}>可申请数量 <strong>{available} ESK</strong></div>
      <form className={styles.form} onSubmit={submit}>
        <label htmlFor="esk-quant-allocation-amount">申请占用数量</label>
        <div>
          <input
            id="esk-quant-allocation-amount"
            value={amount}
            onChange={(event) => setAmount(event.target.value)}
            placeholder="例如 100.000000"
            inputMode="decimal"
            autoComplete="off"
            disabled={!enabled || working !== null}
          />
          <button type="submit" disabled={!enabled || working !== null || !amount.trim()}>
            {working === 'create' ? '申请中…' : '申请量化占用'}
          </button>
        </div>
        {!enabled && <p className={styles.hint}>ESK Paper 写入尚未启用，当前只能查看量化占用记录。</p>}
      </form>

      {message && <p className={styles.success} role="status">{message}</p>}
      {error && <p className={styles.error} role="alert">{error}</p>}

      <div className={styles.history}>
        <div className={styles.historyTitle}><strong>量化分配申请</strong><span>{requests.length} 条</span></div>
        {requests.length === 0 ? <p className={styles.empty}>暂无量化分配申请</p> : requests.map((request) => (
          <div className={styles.request} key={request.request_id}>
            <div>
              <strong>{request.amount} ESK</strong>
              <span>{formatDateTime(request.submitted_at)} · {request.status === 'submitted' ? '已占用，尚未形成仓位' : '已撤销'}</span>
            </div>
            {request.status === 'submitted' && (
              <button type="button" onClick={() => void cancel(request.request_id)} disabled={working !== null}>
                <RotateCcw size={13} />{working === request.request_id ? '撤销中…' : '撤销申请'}
              </button>
            )}
          </div>
        ))}
      </div>
    </section>
  )
}

function newIdempotencyKey() {
  return `esk-quant-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`}`
}

function errorMessage(reason: unknown, fallback: string) {
  return (reason as { message?: string } | null)?.message || fallback
}

function formatDateTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { dateStyle: 'short', timeStyle: 'short' })
}
