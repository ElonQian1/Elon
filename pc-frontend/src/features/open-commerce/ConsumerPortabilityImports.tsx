import { useCallback, useEffect, useRef, useState } from 'react'
import { Download, RefreshCw, ShieldCheck, Trash2, Upload } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerPortabilityExport,
  ConsumerPortabilityImportSummary,
  SignedConsumerPortabilityPackage,
} from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'
import {
  decryptPortabilityArchive,
  isPortabilityEncryptedArchive,
} from './portabilityArchive'

export default function ConsumerPortabilityImports({ projectId }: { projectId: string }) {
  const [imports, setImports] = useState<ConsumerPortabilityImportSummary[]>([])
  const [sourceOperator, setSourceOperator] = useState('')
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [archivePassphrase, setArchivePassphrase] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const fileRef = useRef<HTMLInputElement>(null)

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listConsumerPortabilityImports(projectId)
      setImports(response.imports)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function importPackage() {
    if (!selectedFile) {
      setMessage('请选择可携带数据包 JSON 文件。')
      return
    }
    setBusy(true)
    setMessage('')
    try {
      if (selectedFile.size > 9 * 1024 * 1024) throw new Error('数据包文件超过 9 MiB 本地处理上限')
      const fileValue = JSON.parse(await selectedFile.text()) as unknown
      const decryptedValue = isPortabilityEncryptedArchive(fileValue)
        ? await decryptPortabilityArchive(fileValue, archivePassphrase)
        : fileValue
      const parsed = decryptedValue as
        | ConsumerPortabilityExport
        | SignedConsumerPortabilityPackage
      const packageValue = isSignedPackage(parsed) ? parsed.package : parsed
      const signature = isSignedPackage(parsed) ? parsed.signature : undefined
      const effectiveSourceOperator = isSignedPackage(parsed)
        ? parsed.source_operator
        : sourceOperator.trim()
      if (!effectiveSourceOperator) throw new Error('请填写来源运营方或来源环境名称')
      assertPortabilityPackageShape(packageValue)
      await openCommerceClientApi.createConsumerPortabilityImport(
        projectId,
        effectiveSourceOperator,
        packageValue,
        signature,
      )
      setSelectedFile(null)
      if (fileRef.current) fileRef.current.value = ''
      setMessage('数据包完整性已验证，并作为隔离快照保存。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function download(importId: string) {
    setBusy(true)
    setMessage('')
    try {
      const result = await openCommerceClientApi.getConsumerPortabilityImport(projectId, importId)
      const blob = new Blob([JSON.stringify(result.package, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const anchor = document.createElement('a')
      anchor.href = url
      anchor.download = `imported-portability-${result.source_package_id}.json`
      document.body.appendChild(anchor)
      anchor.click()
      anchor.remove()
      window.setTimeout(() => URL.revokeObjectURL(url), 0)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function remove(item: ConsumerPortabilityImportSummary) {
    if (!window.confirm(`删除来自“${item.source_operator}”的隔离快照？`)) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.deleteConsumerPortabilityImport(projectId, item.id)
      setMessage('隔离快照已删除。')
      await refresh()
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
          <strong>外部可携带数据包</strong>
          <small>验证内容完整性后隔离保存；不会恢复授权、合并档案、写入 ERP 或发起交易。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新导入记录">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        <div style={{ display: 'grid', gap: 8 }}>
          <input
            type="text"
            value={sourceOperator}
            maxLength={160}
            onChange={(event) => setSourceOperator(event.target.value)}
            placeholder="来源运营方或来源环境"
            disabled={busy}
          />
          <input
            type="password"
            value={archivePassphrase}
            minLength={12}
            maxLength={256}
            onChange={(event) => setArchivePassphrase(event.target.value)}
            placeholder="加密归档口令（普通 JSON 留空）"
            disabled={busy}
          />
          <input
            ref={fileRef}
            type="file"
            accept="application/json,.json"
            onChange={(event) => setSelectedFile(event.target.files?.[0] ?? null)}
            disabled={busy}
          />
          <button style={actionStyle('primary', busy)} type="button" onClick={importPackage} disabled={busy}>
            <Upload size={14} />验证并隔离导入
          </button>
        </div>
        {imports.map((item) => (
          <article key={item.id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{item.source_operator}</strong>
              <span style={badgeStyle(item.trust_status === 'trusted_operator_signature_verified' ? 'neutral' : 'warn')}>
                <ShieldCheck size={12} />
                {item.trust_status === 'trusted_operator_signature_verified' ? '签名可信' : '仅完整性'}
              </span>
            </header>
            <small style={commerceStyles.itemMeta}>
              {new Date(item.imported_at).toLocaleString()} · 关系 {item.relationship_count}
              {' · '}请求 {item.data_request_count} · 调用凭证 {item.invocation_receipt_count}
              {' · '}商户身份声明 {item.merchant_identity_claim_count}
            </small>
            <p style={{ ...commerceStyles.itemText, overflowWrap: 'anywhere' }}>{item.envelope_sha256}</p>
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <code style={{ ...commerceStyles.itemMeta, overflowWrap: 'anywhere' }}>{item.source_package_id}</code>
              <div style={commerceStyles.headerActions}>
                <button style={actionStyle('secondary', busy)} type="button" onClick={() => download(item.id)} disabled={busy} title="下载原数据包">
                  <Download size={13} />
                </button>
                <button style={actionStyle('danger', busy)} type="button" onClick={() => remove(item)} disabled={busy} title="删除隔离快照">
                  <Trash2 size={13} />
                </button>
              </div>
            </footer>
          </article>
        ))}
        {imports.length === 0 && <p className={base.empty}>尚未导入外部数据包。</p>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function assertPortabilityPackageShape(value: ConsumerPortabilityExport) {
  if (!value || typeof value !== 'object') throw new Error('数据包 JSON 格式无效')
  if (typeof value.schema !== 'string' || typeof value.payload_json !== 'string') {
    throw new Error('数据包缺少版本或规范负载')
  }
  if (typeof value.payload_sha256 !== 'string' || !value.payload) {
    throw new Error('数据包缺少摘要或负载')
  }
}

function isSignedPackage(
  value: ConsumerPortabilityExport | SignedConsumerPortabilityPackage,
): value is SignedConsumerPortabilityPackage {
  const candidate = value as SignedConsumerPortabilityPackage
  return Boolean(candidate.package && candidate.signature)
}
