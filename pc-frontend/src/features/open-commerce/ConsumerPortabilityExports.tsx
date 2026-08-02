import { useCallback, useEffect, useState } from 'react'
import { Download, FileArchive, RefreshCw } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerPortabilityExport,
  ConsumerPortabilityExportSummary,
} from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

export default function ConsumerPortabilityExports({ projectId }: { projectId: string }) {
  const [exports, setExports] = useState<ConsumerPortabilityExportSummary[]>([])
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listConsumerPortabilityExports(projectId)
      setExports(response.exports)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function createAndDownload() {
    setBusy(true)
    setMessage('')
    try {
      const result = await openCommerceClientApi.createConsumerPortabilityExport(
        projectId,
        `pc-export-${crypto.randomUUID()}`,
      )
      await downloadVerifiedExport(result)
      setMessage('可携带数据包已校验并下载。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function download(summary: ConsumerPortabilityExportSummary) {
    setBusy(true)
    setMessage('')
    try {
      const result = await openCommerceClientApi.getConsumerPortabilityExport(projectId, summary.id)
      await downloadVerifiedExport(result)
      setMessage('历史数据包已校验并下载。')
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
          <strong>我的可携带数据包</strong>
          <small>导出关系、删除回执、本人低敏偏好和披露快照；不含订单、联系方式、支付或账号标识。</small>
        </span>
        <div style={commerceStyles.headerActions}>
          <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新数据包">
            <RefreshCw size={14} />
          </button>
          <button style={actionStyle('primary', busy)} type="button" onClick={createAndDownload} disabled={busy}>
            <FileArchive size={14} />生成并下载
          </button>
        </div>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        {exports.map((item) => (
          <article key={item.id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{new Date(item.created_at).toLocaleString()}</strong>
              <span style={badgeStyle()}>SHA-256</span>
            </header>
            <p style={{ ...commerceStyles.itemText, overflowWrap: 'anywhere' }}>{item.payload_sha256}</p>
            <small style={commerceStyles.itemMeta}>
              关系 {item.relationship_count} · 续期 {item.renewal_count} · 删除请求 {item.data_request_count}
              {' · '}偏好档案 {item.preference_profile_included ? '1' : '0'} · 披露 {item.preference_disclosure_count}
            </small>
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <code style={{ ...commerceStyles.itemMeta, overflowWrap: 'anywhere' }}>{item.id}</code>
              <button style={actionStyle('secondary', busy)} type="button" onClick={() => download(item)} disabled={busy}>
                <Download size={13} />下载
              </button>
            </footer>
          </article>
        ))}
        {exports.length === 0 && <p className={base.empty}>尚未生成可携带数据包。</p>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

async function downloadVerifiedExport(value: ConsumerPortabilityExport) {
  if (!crypto.subtle) throw new Error('当前浏览器无法执行 SHA-256 校验')
  const payloadBytes = new TextEncoder().encode(JSON.stringify(value.payload))
  const digestBytes = new Uint8Array(await crypto.subtle.digest('SHA-256', payloadBytes))
  const digest = Array.from(digestBytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
  if (digest !== value.payload_sha256) throw new Error('数据包摘要校验失败，已停止下载')

  const blob = new Blob([JSON.stringify(value, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = `consumer-portability-${value.id}.json`
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  window.setTimeout(() => URL.revokeObjectURL(url), 0)
}
