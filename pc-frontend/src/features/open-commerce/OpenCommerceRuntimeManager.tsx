import { useEffect, useState } from 'react'
import { openCommerceApi } from './openCommerceApi'
import type { OpenCommerceRuntimeBinding } from './openCommerceTypes'
import { commerceStyles } from './openCommerceStyles'
import styles from './OpenCommercePanel.module.css'

interface Props {
  projectId: string
  merchantId: string
  binding?: OpenCommerceRuntimeBinding
  canEdit: boolean
  onChanged: () => Promise<void>
}

export default function OpenCommerceRuntimeManager({
  projectId,
  merchantId,
  binding,
  canEdit,
  onChanged,
}: Props) {
  const [endpoint, setEndpoint] = useState('')
  const [credentialRef, setCredentialRef] = useState('OPEN_COMMERCE_RUNTIME_SECRET_')
  const [manifestSha256, setManifestSha256] = useState('')
  const [timeoutMs, setTimeoutMs] = useState(5000)
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  useEffect(() => {
    setEndpoint(binding?.endpoint_base_url ?? '')
    setCredentialRef(binding?.credential_ref ?? 'OPEN_COMMERCE_RUNTIME_SECRET_')
    setManifestSha256(binding?.manifest_sha256 ?? '')
    setTimeoutMs(binding?.timeout_ms ?? 5000)
  }, [binding, merchantId])

  async function save(event: React.FormEvent) {
    event.preventDefault()
    setBusy('save')
    setMessage('')
    try {
      await openCommerceApi.upsertRuntime(projectId, merchantId, {
        endpoint_base_url: endpoint,
        credential_ref: credentialRef,
        manifest_sha256: manifestSha256.trim() || undefined,
        timeout_ms: timeoutMs,
      })
      setMessage('运行绑定已保存。完成签名健康验证后，真实能力才会启用。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function verify() {
    setBusy('verify')
    setMessage('')
    try {
      await openCommerceApi.verifyRuntime(projectId, merchantId)
      setMessage('商户身份、签名密钥和能力清单已验证。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  return (
    <section className={styles.integrationSection}>
      <header>
        <span>
          <strong>商户应用运行时</strong>
          <small>平台只保存服务端密钥引用；不会把密钥或直连地址公开给消费者 AI</small>
        </span>
        <em>{binding ? statusLabel(binding.status) : '未配置'}</em>
      </header>
      <div className={styles.integrationGrid}>
        <div className={styles.runtimeSummary}>
          <strong>{binding?.endpoint_base_url ?? '尚未绑定商户后端'}</strong>
          <code>{binding?.credential_ref ?? '服务端环境变量引用未设置'}</code>
          <p>
            {binding?.last_verified_at
              ? `最近验证：${new Date(binding.last_verified_at).toLocaleString('zh-CN')}`
              : '保存配置不代表验证成功。'}
          </p>
          {binding?.last_error_code && <small>错误代码：{binding.last_error_code}</small>}
          <button type="button" onClick={verify} disabled={!canEdit || !binding || busy !== ''}>
            {busy === 'verify' ? '验证中…' : '签名验证'}
          </button>
        </div>
        <form className={styles.formCard} onSubmit={save}>
          <header><strong>受控运行绑定</strong><small>生产地址必须为白名单内的 HTTPS 主机</small></header>
          <label>商户后端地址<input type="url" value={endpoint} onChange={(event) => setEndpoint(event.target.value)} placeholder="https://merchant.example.com" required disabled={!canEdit} /></label>
          <label>密钥环境变量引用<input value={credentialRef} onChange={(event) => setCredentialRef(event.target.value.toUpperCase())} pattern="OPEN_COMMERCE_RUNTIME_SECRET_[A-Z0-9_]+" required disabled={!canEdit} /></label>
          <label>能力清单 SHA-256（可选）<input value={manifestSha256} onChange={(event) => setManifestSha256(event.target.value.toLowerCase())} pattern="[a-f0-9]{64}" disabled={!canEdit} /></label>
          <label>超时毫秒<input type="number" min={500} max={15000} value={timeoutMs} onChange={(event) => setTimeoutMs(Number(event.target.value))} disabled={!canEdit} /></label>
          <button type="submit" disabled={!canEdit || busy !== ''}>{busy === 'save' ? '保存中…' : '保存运行绑定'}</button>
        </form>
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function statusLabel(status: OpenCommerceRuntimeBinding['status']) {
  const labels = {
    configured: '待验证',
    active: '已验证',
    degraded: '调用异常',
    disabled: '已停用',
  }
  return labels[status]
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return '操作失败，请稍后重试'
}
