import { useEffect, useMemo, useState } from 'react'
import { FileCheck2, Save, Send } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { OpenCommerceDeveloperApp } from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles } from './openCommerceStyles'

function statusLabel(status: OpenCommerceDeveloperApp['manifest_status']) {
  if (status === 'submitted') return '审核中'
  if (status === 'changes_requested') return '需修改'
  if (status === 'approved') return '资料已审核'
  return '草稿'
}

function nullable(value: string) {
  return value.trim() || null
}

function splitScopes(value: string) {
  return value
    .split(/[\s,，]+/)
    .map((scope) => scope.trim())
    .filter(Boolean)
}

export default function DeveloperAppManifestPanel({
  projectId,
  apps,
  canEdit,
  onChanged,
}: {
  projectId: string
  apps: OpenCommerceDeveloperApp[]
  canEdit: boolean
  onChanged: () => Promise<void>
}) {
  const [appRecordId, setAppRecordId] = useState('')
  const [homepageUrl, setHomepageUrl] = useState('')
  const [privacyPolicyUrl, setPrivacyPolicyUrl] = useState('')
  const [termsUrl, setTermsUrl] = useState('')
  const [supportEmail, setSupportEmail] = useState('')
  const [requestedScopes, setRequestedScopes] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const selectedApp = useMemo(
    () => apps.find((app) => app.id === appRecordId),
    [appRecordId, apps],
  )

  useEffect(() => {
    if (!apps.some((app) => app.id === appRecordId)) {
      setAppRecordId(apps.find((app) => app.status === 'active')?.id ?? apps[0]?.id ?? '')
    }
  }, [appRecordId, apps])

  useEffect(() => {
    if (!selectedApp) return
    setHomepageUrl(selectedApp.homepage_url ?? '')
    setPrivacyPolicyUrl(selectedApp.privacy_policy_url ?? '')
    setTermsUrl(selectedApp.terms_url ?? '')
    setSupportEmail(selectedApp.support_email ?? '')
    setRequestedScopes(selectedApp.requested_scopes.join(', '))
    setMessage('')
  }, [selectedApp])

  async function saveManifest(event: React.FormEvent) {
    event.preventDefault()
    if (!selectedApp) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.updateDeveloperAppManifest(projectId, selectedApp.id, {
        expected_manifest_revision: selectedApp.manifest_revision,
        homepage_url: nullable(homepageUrl),
        privacy_policy_url: nullable(privacyPolicyUrl),
        terms_url: nullable(termsUrl),
        support_email: nullable(supportEmail),
        requested_scopes: splitScopes(requestedScopes),
      })
      setMessage('资料草稿已保存。修改已审核资料会产生新修订并重新进入草稿状态。')
      await onChanged()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function submitManifest() {
    if (!selectedApp) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.submitDeveloperAppManifest(
        projectId,
        selectedApp.id,
        selectedApp.manifest_revision,
      )
      setMessage('资料已提交审核。审核通过不会自动签发生产凭据或开放真实交易。')
      await onChanged()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  const locked = selectedApp?.manifest_status === 'submitted'
  const editable = canEdit && selectedApp?.status === 'active' && !locked && !busy
  const submittable = editable && selectedApp?.manifest_status !== 'approved'

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>App 资料与能力申请</strong>
        <span style={badgeStyle(selectedApp?.manifest_status === 'changes_requested' ? 'warn' : 'neutral')}>
          {selectedApp ? `${statusLabel(selectedApp.manifest_status)} · R${selectedApp.manifest_revision}` : '未选择'}
        </span>
      </header>
      <form className={base.formCard} style={commerceStyles.sectionBody} onSubmit={saveManifest}>
        <div style={commerceStyles.grid}>
          <label>开发者 App<select value={appRecordId} onChange={(event) => setAppRecordId(event.target.value)}>
            {apps.map((app) => <option key={app.id} value={app.id}>{app.display_name} · {app.app_id}</option>)}
          </select></label>
          <label>支持邮箱<input type="email" value={supportEmail} onChange={(event) => setSupportEmail(event.target.value)} disabled={!editable} placeholder="support@example.com" /></label>
          <label style={commerceStyles.wideField}>应用主页<input type="url" value={homepageUrl} onChange={(event) => setHomepageUrl(event.target.value)} disabled={!editable} placeholder="https://example.com" /></label>
          <label style={commerceStyles.wideField}>隐私政策<input type="url" value={privacyPolicyUrl} onChange={(event) => setPrivacyPolicyUrl(event.target.value)} disabled={!editable} placeholder="https://example.com/privacy" /></label>
          <label style={commerceStyles.wideField}>服务条款<input type="url" value={termsUrl} onChange={(event) => setTermsUrl(event.target.value)} disabled={!editable} placeholder="https://example.com/terms" /></label>
          <label style={commerceStyles.wideField}>申请能力<textarea value={requestedScopes} onChange={(event) => setRequestedScopes(event.target.value)} disabled={!editable} placeholder="catalog.search, order.quote, order.create" /></label>
        </div>
        {selectedApp?.review_note && <p style={commerceStyles.itemText}>审核说明：{selectedApp.review_note}</p>}
        <div style={commerceStyles.headerActions}>
          <button style={actionStyle('secondary', !editable)} type="submit" disabled={!editable}><Save size={13} />保存草稿</button>
          <button style={actionStyle('primary', !submittable)} type="button" onClick={submitManifest} disabled={!submittable}><Send size={13} />提交审核</button>
        </div>
        <small style={commerceStyles.itemMeta}><FileCheck2 size={12} />资料审核仅形成平台审核记录，不代表生产授权、工商认证或外部平台背书。</small>
        {apps.length === 0 && <p className={base.empty}>请先注册沙盒应用。</p>}
        {message && <div style={commerceStyles.message}>{message}</div>}
      </form>
    </section>
  )
}
