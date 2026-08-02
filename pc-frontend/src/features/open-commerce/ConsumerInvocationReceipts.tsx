import { useCallback, useEffect, useState } from 'react'
import { Download, ReceiptText, RefreshCw } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerInvocationReceipt,
  ConsumerInvocationReceiptSummary,
} from './openCommerceClientTypes'
import { errorText, formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

export default function ConsumerInvocationReceipts({ refreshKey }: { refreshKey: number }) {
  const [receipts, setReceipts] = useState<ConsumerInvocationReceiptSummary[]>([])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setBusy(true)
    setMessage('')
    try {
      const response = await openCommerceClientApi.listConsumerInvocationReceipts()
      setReceipts(response.receipts)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh, refreshKey])

  async function download(invocationId: string) {
    setBusy(true)
    setMessage('')
    try {
      const receipt = await openCommerceClientApi.getConsumerInvocationReceipt(invocationId)
      await downloadVerifiedReceipt(receipt)
      setMessage('调用凭证已校验并下载。')
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <span>
          <strong>我的 AI 调用凭证</strong>
          <small>账号范围内的终态调用；不保存原始输入，也不代表支付或链上存证。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新调用凭证">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        {receipts.map((receipt) => (
          <article key={receipt.invocation_id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{receipt.capability_key}</strong>
              <span style={badgeStyle(receipt.status === 'succeeded' ? 'neutral' : 'warn')}>
                {receipt.status === 'succeeded' ? '已完成' : '已失败'}
              </span>
            </header>
            <p style={commerceStyles.itemText}>商户 {receipt.merchant_id} · 应用 {receipt.requester_app_id}</p>
            <small style={commerceStyles.itemMeta}>
              {formatMicros(receipt.amount_micros, receipt.currency)} · 未扣真实资金 · {new Date(receipt.completed_at).toLocaleString()}
            </small>
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <span style={commerceStyles.itemMeta}>
                <ReceiptText size={13} /> {receipt.result_available ? '含商户返回结果' : receipt.error_code ?? '无返回结果'}
              </span>
              <button style={actionStyle('secondary', busy)} type="button" onClick={() => download(receipt.invocation_id)} disabled={busy}>
                <Download size={13} />校验并下载
              </button>
            </footer>
          </article>
        ))}
        {receipts.length === 0 && <p className={base.empty}>尚无终态调用凭证。</p>}
      </div>
      {message && (
        <div style={{ ...commerceStyles.message, ...(message.includes('失败') ? errorMessageStyle : {}) }}>
          {message}
        </div>
      )}
    </section>
  )
}

async function downloadVerifiedReceipt(receipt: ConsumerInvocationReceipt) {
  if (!crypto.subtle) throw new Error('当前浏览器无法执行 SHA-256 校验')
  const payloadBytes = new TextEncoder().encode(receipt.payload_json)
  const digestBytes = new Uint8Array(await crypto.subtle.digest('SHA-256', payloadBytes))
  const digest = Array.from(digestBytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
  if (digest !== receipt.payload_sha256) throw new Error('调用凭证摘要校验失败，已停止下载')

  const payload = JSON.parse(receipt.payload_json) as ConsumerInvocationReceipt['payload']
  if (JSON.stringify(payload) !== JSON.stringify(receipt.payload)) {
    throw new Error('调用凭证负载不一致，已停止下载')
  }

  const blob = new Blob([
    JSON.stringify({
      schema: receipt.schema,
      payload_sha256: receipt.payload_sha256,
      payload_json: receipt.payload_json,
      payload,
    }, null, 2),
  ], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = `consumer-invocation-${receipt.payload.invocation_id}.json`
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  window.setTimeout(() => URL.revokeObjectURL(url), 0)
}
