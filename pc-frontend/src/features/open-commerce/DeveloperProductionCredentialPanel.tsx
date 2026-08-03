import { useCallback, useEffect, useMemo, useState } from 'react'
import { Ban, Copy, KeyRound, RefreshCw } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { OpenCommerceDeveloperApp } from './openCommerceClientTypes'
import type { DeveloperProductionCredential } from './developerProductionCredentialTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

function statusLabel(credential: DeveloperProductionCredential) {
  if (credential.status === 'revoked') return '已撤销'
  if (new Date(credential.expires_at).getTime() <= Date.now()) return '已到期'
  return '可用'
}

export default function DeveloperProductionCredentialPanel({
  projectId,
  apps,
  canEdit,
}: {
  projectId: string
  apps: OpenCommerceDeveloperApp[]
  canEdit: boolean
}) {
  const [appRecordId, setAppRecordId] = useState('')
  const [credentials, setCredentials] = useState<DeveloperProductionCredential[]>([])
  const [issuanceEnabled, setIssuanceEnabled] = useState(false)
  const [reasons, setReasons] = useState<Record<string, string>>({})
  const [busyId, setBusyId] = useState('')
  const [message, setMessage] = useState('')

  const selectedApp = useMemo(
    () => apps.find((app) => app.id === appRecordId),
    [appRecordId, apps],
  )

  useEffect(() => {
    if (!apps.some((app) => app.id === appRecordId)) {
      setAppRecordId(apps[0]?.id ?? '')
    }
  }, [appRecordId, apps])

  const refresh = useCallback(async () => {
    if (!selectedApp || !canEdit) {
      setCredentials([])
      return
    }
    try {
      const response = await openCommerceClientApi.listDeveloperProductionCredentials(
        projectId,
        selectedApp.id,
      )
      setCredentials(response.credentials)
      setIssuanceEnabled(response.issuance_enabled)
      setMessage('')
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [canEdit, projectId, selectedApp])

  useEffect(() => { refresh() }, [refresh])

  async function revoke(credential: DeveloperProductionCredential) {
    if (!selectedApp) return
    const reason = reasons[credential.id]?.trim() || '项目方主动撤销生产凭据'
    setBusyId(credential.id)
    setMessage('')
    try {
      await openCommerceClientApi.revokeDeveloperProductionCredential(
        projectId,
        selectedApp.id,
        credential.id,
        reason,
      )
      setMessage('生产凭据已撤销，后续调用将失败关闭。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusyId('')
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>App 生产凭据</strong>
        <div style={commerceStyles.headerActions}>
          <span style={badgeStyle(issuanceEnabled ? 'neutral' : 'warn')}>{issuanceEnabled ? '运营开关已启用' : '运营开关关闭'}</span>
          <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新生产凭据"><RefreshCw size={13} /></button>
        </div>
      </header>
      <div className={base.formCard} style={commerceStyles.sectionBody}>
        <label>开发者 App<select value={appRecordId} onChange={(event) => setAppRecordId(event.target.value)}>
          {apps.map((app) => <option key={app.id} value={app.id}>{app.display_name} · {app.app_id}</option>)}
        </select></label>
        <div style={commerceStyles.list}>
          {credentials.map((credential) => {
            const active = credential.status === 'active' && new Date(credential.expires_at).getTime() > Date.now()
            const busy = busyId === credential.id
            return (
              <article className={base.formCard} style={listItemStyle()} key={credential.id}>
                <header style={commerceStyles.itemHeader}>
                  <h3 style={commerceStyles.itemTitle}><KeyRound size={14} />{credential.token_hint}</h3>
                  <span style={badgeStyle(active ? 'neutral' : 'warn')}>{statusLabel(credential)}</span>
                </header>
                <code style={commerceStyles.itemMeta}>R{credential.manifest_revision} · {credential.scopes.join(', ')}</code>
                <small style={commerceStyles.itemMeta}>到期：{new Date(credential.expires_at).toLocaleString()} · 最近使用：{credential.last_used_at ? new Date(credential.last_used_at).toLocaleString() : '尚未使用'}</small>
                {credential.revocation_reason && <p style={commerceStyles.itemText}>撤销原因：{credential.revocation_reason}</p>}
                {active && <footer style={commerceStyles.itemHeader}>
                  <input value={reasons[credential.id] ?? ''} onChange={(event) => setReasons((current) => ({ ...current, [credential.id]: event.target.value }))} placeholder="撤销原因（可选）" disabled={busy} />
                  <button style={actionStyle('danger', busy)} type="button" onClick={() => revoke(credential)} disabled={busy}><Ban size={13} />立即撤销</button>
                </footer>}
              </article>
            )
          })}
          {credentials.length === 0 && <p className={base.empty}>当前 App 尚无生产凭据。签发操作只在平台准入审查区提供。</p>}
        </div>
        <small style={commerceStyles.itemMeta}><Copy size={12} />完整密钥仅在签发瞬间显示；此处只保留末尾提示、范围和审计状态。</small>
        {message && <div style={commerceStyles.message}>{message}</div>}
      </div>
    </section>
  )
}
