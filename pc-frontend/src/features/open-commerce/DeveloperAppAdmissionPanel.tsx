import { useEffect, useMemo, useState } from 'react'
import { Send, ShieldCheck } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { OpenCommerceDeveloperApp } from './openCommerceClientTypes'
import type { DeveloperAppAdmission } from './developerAppAdmissionTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles } from './openCommerceStyles'

function admissionLabel(admission: DeveloperAppAdmission | null, currentRevision: number) {
  if (!admission) return '未申请'
  if (admission.manifest_revision !== currentRevision) return '资料已变化'
  if (admission.status === 'submitted') return '准入审核中'
  if (admission.status === 'changes_requested') return '需补充材料'
  if (admission.status === 'approved') return '准入记录已批准'
  return '准入已暂停'
}

export default function DeveloperAppAdmissionPanel({
  projectId,
  apps,
  canEdit,
}: {
  projectId: string
  apps: OpenCommerceDeveloperApp[]
  canEdit: boolean
}) {
  const [appRecordId, setAppRecordId] = useState('')
  const [admission, setAdmission] = useState<DeveloperAppAdmission | null>(null)
  const [organizationName, setOrganizationName] = useState('')
  const [jurisdiction, setJurisdiction] = useState('')
  const [registrationId, setRegistrationId] = useState('')
  const [attested, setAttested] = useState(false)
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
    if (!selectedApp) {
      setAdmission(null)
      return
    }
    let active = true
    setAdmission(null)
    setBusy(true)
    openCommerceClientApi.getDeveloperAppAdmission(projectId, selectedApp.id)
      .then((response) => {
        if (!active) return
        setAdmission(response.admission)
        setOrganizationName(response.admission?.organization_name ?? '')
        setJurisdiction(response.admission?.jurisdiction ?? '')
        setRegistrationId(response.admission?.registration_id ?? '')
        setAttested(false)
        setMessage('')
      })
      .catch((error) => active && setMessage(errorText(error)))
      .finally(() => active && setBusy(false))
    return () => { active = false }
  }, [projectId, selectedApp])

  async function submit(event: React.FormEvent) {
    event.preventDefault()
    if (!selectedApp) return
    setBusy(true)
    setMessage('')
    try {
      const result = await openCommerceClientApi.submitDeveloperAppAdmission(
        projectId,
        selectedApp.id,
        {
          expected_manifest_revision: selectedApp.manifest_revision,
          organization_name: organizationName,
          jurisdiction,
          registration_id: registrationId,
          information_attested: attested,
        },
      )
      setAdmission(result)
      setAttested(false)
      setMessage('准入申请已提交；本步骤不会签发生产凭据或开启真实交易。')
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  const currentAdmission = admission?.manifest_revision === selectedApp?.manifest_revision
  const manifestReady = selectedApp?.status === 'active'
    && selectedApp.manifest_status === 'approved'
    && selectedApp.domain_verification_status === 'verified'
    && selectedApp.domain_verification_revision === selectedApp.manifest_revision
  const locked = currentAdmission
    && (admission?.status === 'submitted' || admission?.status === 'approved')
  const editable = Boolean(canEdit && manifestReady && !locked && !busy)

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>App 公共网络准入</strong>
        <span style={badgeStyle(admission?.status === 'changes_requested' || admission?.status === 'suspended' ? 'warn' : 'neutral')}>
          {selectedApp ? admissionLabel(admission, selectedApp.manifest_revision) : '未选择'}
        </span>
      </header>
      <form className={base.formCard} style={commerceStyles.sectionBody} onSubmit={submit}>
        <div style={commerceStyles.grid}>
          <label>开发者 App<select value={appRecordId} onChange={(event) => setAppRecordId(event.target.value)}>
            {apps.map((app) => <option key={app.id} value={app.id}>{app.display_name} · {app.app_id}</option>)}
          </select></label>
          <label>注册地区<input value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value)} disabled={!editable} placeholder="国家或地区" required /></label>
          <label style={commerceStyles.wideField}>主体名称<input value={organizationName} onChange={(event) => setOrganizationName(event.target.value)} disabled={!editable} required /></label>
          <label style={commerceStyles.wideField}>登记编号<input value={registrationId} onChange={(event) => setRegistrationId(event.target.value)} disabled={!editable} required /></label>
          <label style={commerceStyles.wideField}><input type="checkbox" checked={attested} onChange={(event) => setAttested(event.target.checked)} disabled={!editable} />我确认以上声明真实，并有权代表该主体提交</label>
        </div>
        {admission?.review_note && <p style={commerceStyles.itemText}>审查说明：{admission.review_note}</p>}
        {admission?.risk_tier && <small style={commerceStyles.itemMeta}>风险层级：{admission.risk_tier}</small>}
        <div style={commerceStyles.itemHeader}>
          <small style={commerceStyles.itemMeta}><ShieldCheck size={12} />仅审查当前资料修订，不签发生产凭据</small>
          <button style={actionStyle('primary', !editable || !attested)} type="submit" disabled={!editable || !attested}><Send size={13} />提交准入审查</button>
        </div>
        {!manifestReady && selectedApp && <p className={base.empty}>请先完成当前资料修订审核和主页域名验证。</p>}
        {apps.length === 0 && <p className={base.empty}>请先注册沙盒应用。</p>}
        {message && <div style={commerceStyles.message}>{message}</div>}
      </form>
    </section>
  )
}
