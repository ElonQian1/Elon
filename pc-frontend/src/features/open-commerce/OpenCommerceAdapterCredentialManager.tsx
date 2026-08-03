import { useCallback, useEffect, useState } from 'react'
import { Ban, Clipboard, KeyRound, RefreshCw, RotateCw, X } from 'lucide-react'
import { openCommerceApi } from './openCommerceApi'
import type {
  OpenCommerceAdapterCredential,
  OpenCommerceAdapterCredentialIssue,
  OpenCommerceIntegration,
} from './openCommerceTypes'
import { errorText } from './openCommerceUi'
import { actionStyle, badgeStyle } from './openCommerceStyles'
import styles from './OpenCommerceAdapterCredentialManager.module.css'

type Props = {
  projectId: string
  integrations: OpenCommerceIntegration[]
  canEdit: boolean
}

export default function OpenCommerceAdapterCredentialManager({
  projectId,
  integrations,
  canEdit,
}: Props) {
  const [credentials, setCredentials] = useState<OpenCommerceAdapterCredential[]>([])
  const [issued, setIssued] = useState<OpenCommerceAdapterCredentialIssue | null>(null)
  const [expiresInDays, setExpiresInDays] = useState(90)
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null)

  const refresh = useCallback(async () => {
    if (!projectId) return
    setBusy('refresh')
    try {
      const result = await openCommerceApi.listAdapterCredentials(projectId)
      setCredentials(result.credentials)
    } catch (error) {
      setMessage({ text: errorText(error), error: true })
    } finally {
      setBusy('')
    }
  }, [projectId])

  useEffect(() => {
    void refresh()
  }, [refresh])

  async function rotate(integration: OpenCommerceIntegration, existing?: OpenCommerceAdapterCredential) {
    const verb = existing ? '轮换' : '签发'
    if (!globalThis.confirm(`${verb}后明文 Token 只显示一次，有效期 ${expiresInDays} 天${existing ? '，旧 Token 会立即失效' : ''}。是否继续？`)) return
    setBusy(integration.id)
    setMessage(null)
    setIssued(null)
    try {
      const result = await openCommerceApi.rotateAdapterCredential(projectId, integration.id, expiresInDays)
      setIssued(result)
      setMessage({ text: `已${verb}机器凭据，请立即复制并交给该接入器。`, error: false })
      await refresh()
    } catch (error) {
      setMessage({ text: errorText(error), error: true })
    } finally {
      setBusy('')
    }
  }

  async function revoke(credential: OpenCommerceAdapterCredential) {
    if (!globalThis.confirm('撤销后，该接入器将无法再提交业务衔接回执。是否继续？')) return
    setBusy(credential.integration_id)
    setMessage(null)
    setIssued(null)
    try {
      await openCommerceApi.revokeAdapterCredential(projectId, credential.id)
      setMessage({ text: '机器凭据已撤销，原 Token 已立即失效。', error: false })
      await refresh()
    } catch (error) {
      setMessage({ text: errorText(error), error: true })
    } finally {
      setBusy('')
    }
  }

  async function copyToken() {
    if (!issued) return
    try {
      await navigator.clipboard.writeText(issued.adapter_token)
      setMessage({ text: 'Token 已复制。关闭本提示后无法再次查看。', error: false })
    } catch {
      setMessage({ text: '浏览器未允许自动复制，请手动选择 Token。', error: true })
    }
  }

  return (
    <div className={styles.manager}>
      <header>
        <span>
          <strong><KeyRound size={14} />接入器机器凭据</strong>
          <small>固定只允许写入业务衔接回执，不能读取经营数据或调用消费者能力。</small>
        </span>
        <div className={styles.headerActions}>
          <label className={styles.expiryControl}>
            有效期
            <select value={expiresInDays} onChange={(event) => setExpiresInDays(Number(event.target.value))} disabled={busy !== ''}>
              <option value={30}>30 天</option>
              <option value={90}>90 天</option>
              <option value={365}>365 天</option>
            </select>
          </label>
          <button style={actionStyle('icon', busy !== '')} type="button" onClick={refresh} disabled={busy !== ''} title="刷新机器凭据">
            <RefreshCw size={14} />
          </button>
        </div>
      </header>

      {issued && (
        <div className={styles.oneTimeToken}>
          <header>
            <strong>一次性 Token</strong>
            <button style={actionStyle('icon')} type="button" onClick={() => setIssued(null)} title="关闭一次性 Token">
              <X size={14} />
            </button>
          </header>
          <code>{issued.adapter_token}</code>
          <p>服务端只保存摘要，有效至 {new Date(issued.credential.expires_at).toLocaleString('zh-CN')}。本页面关闭或刷新后无法找回，只能重新轮换。</p>
          <button style={actionStyle('primary')} type="button" onClick={copyToken}>
            <Clipboard size={14} />复制 Token
          </button>
        </div>
      )}

      <div className={styles.rows}>
        {integrations.map((integration) => {
          const credential = credentials.find((item) => item.integration_id === integration.id)
          const expiry = credential ? credentialExpiry(credential) : 'missing'
          const active = expiry === 'active' || expiry === 'expiring'
          return (
            <article key={integration.id}>
              <span>
                <strong>{integration.display_name}</strong>
                <small>{credential ? `版本 ${credential.credential_version} · ${credential.token_hint}` : '尚未签发'}</small>
              </span>
              <span style={badgeStyle(expiry === 'expired' ? 'danger' : active ? expiry === 'expiring' ? 'warn' : 'neutral' : 'warn')}>
                {expiryLabel(expiry)}
              </span>
              <p>
                权限：business_handoff.write
                {credential ? ` · 到期 ${new Date(credential.expires_at).toLocaleDateString('zh-CN')}` : ''}
                {credential?.last_used_at ? ` · 最近使用 ${new Date(credential.last_used_at).toLocaleString('zh-CN')}` : ''}
              </p>
              <footer>
                <button
                  style={actionStyle('secondary', !canEdit || busy !== '' || integration.status === 'disabled')}
                  type="button"
                  onClick={() => rotate(integration, credential)}
                  disabled={!canEdit || busy !== '' || integration.status === 'disabled'}
                >
                  <RotateCw size={13} />{credential ? '轮换' : '签发'}
                </button>
                {active && credential && (
                  <button
                    style={actionStyle('danger', !canEdit || busy !== '')}
                    type="button"
                    onClick={() => revoke(credential)}
                    disabled={!canEdit || busy !== ''}
                  >
                    <Ban size={13} />撤销
                  </button>
                )}
              </footer>
            </article>
          )
        })}
        {integrations.length === 0 && <p className={styles.empty}>登记数据接入后，才可为实际接入器签发机器凭据。</p>}
      </div>
      {message && <p className={message.error ? styles.error : styles.success}>{message.text}</p>}
    </div>
  )
}

type CredentialExpiry = 'active' | 'expiring' | 'expired' | 'revoked' | 'missing'

function credentialExpiry(credential: OpenCommerceAdapterCredential): CredentialExpiry {
  if (credential.status === 'revoked') return 'revoked'
  if (credential.is_expired || new Date(credential.expires_at).getTime() <= Date.now()) return 'expired'
  const remaining = new Date(credential.expires_at).getTime() - Date.now()
  return remaining <= 14 * 24 * 60 * 60 * 1000 ? 'expiring' : 'active'
}

function expiryLabel(expiry: CredentialExpiry) {
  const labels: Record<CredentialExpiry, string> = {
    active: '可鉴权',
    expiring: '即将到期',
    expired: '已到期',
    revoked: '已撤销',
    missing: '未配置',
  }
  return labels[expiry]
}
