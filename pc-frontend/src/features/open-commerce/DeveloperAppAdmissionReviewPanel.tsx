import { useCallback, useEffect, useState } from 'react'
import { Check, PauseCircle, RefreshCw, Undo2 } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  DeveloperAppAdmissionReviewItem,
  DeveloperAppAdmissionRiskTier,
} from './developerAppAdmissionTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

export default function DeveloperAppAdmissionReviewPanel() {
  const [items, setItems] = useState<DeveloperAppAdmissionReviewItem[]>([])
  const [notes, setNotes] = useState<Record<string, string>>({})
  const [riskTiers, setRiskTiers] = useState<Record<string, DeveloperAppAdmissionRiskTier>>({})
  const [busyId, setBusyId] = useState('')
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listReviewableDeveloperAppAdmissions()
      setItems(response.items)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [])

  useEffect(() => { refresh() }, [refresh])

  async function review(
    item: DeveloperAppAdmissionReviewItem,
    decision: 'approved' | 'changes_requested' | 'suspended',
  ) {
    const note = notes[item.app.id]?.trim() ?? ''
    if (decision !== 'approved' && !note) {
      setMessage('退回或暂停准入时必须填写审查说明。')
      return
    }
    setBusyId(item.app.id)
    setMessage('')
    try {
      await openCommerceClientApi.reviewDeveloperAppAdmission(item.app.id, {
        expected_manifest_revision: item.admission.manifest_revision,
        decision,
        risk_tier: riskTiers[item.app.id] ?? 'standard',
        note,
      })
      setNotes((current) => ({ ...current, [item.app.id]: '' }))
      setMessage(decision === 'approved' ? '准入记录已批准。' : decision === 'suspended' ? '准入记录已暂停。' : '补充要求已退回。')
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
        <strong>平台 App 准入审查</strong>
        <div style={commerceStyles.headerActions}>
          <span style={badgeStyle(items.some((item) => item.admission.status === 'submitted') ? 'warn' : 'neutral')}>{items.length} 条</span>
          <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新准入队列"><RefreshCw size={13} /></button>
        </div>
      </header>
      <div className={base.formCard} style={commerceStyles.sectionBody}>
        <div style={commerceStyles.list}>
          {items.map((item) => {
            const busy = busyId === item.app.id
            const pending = item.admission.status === 'submitted'
            return (
              <article className={base.formCard} style={listItemStyle()} key={item.admission.id}>
                <header style={commerceStyles.itemHeader}>
                  <h3 style={commerceStyles.itemTitle}>{item.app.display_name}</h3>
                  <span style={badgeStyle(pending ? 'warn' : 'neutral')}>{pending ? '待审' : '已批准'}</span>
                </header>
                <code style={commerceStyles.itemMeta}>{item.app.app_id} · R{item.admission.manifest_revision}</code>
                <p style={commerceStyles.itemText}>{item.admission.organization_name}</p>
                <code style={commerceStyles.itemMeta}>{item.admission.jurisdiction} · {item.admission.registration_id}</code>
                {pending && <label>风险层级<select value={riskTiers[item.app.id] ?? 'standard'} onChange={(event) => setRiskTiers((current) => ({ ...current, [item.app.id]: event.target.value as DeveloperAppAdmissionRiskTier }))} disabled={busy}>
                  <option value="low">低</option><option value="standard">标准</option><option value="enhanced">加强</option>
                </select></label>}
                <label>审查说明<textarea value={notes[item.app.id] ?? ''} onChange={(event) => setNotes((current) => ({ ...current, [item.app.id]: event.target.value }))} disabled={busy} placeholder={pending ? '批准时可选；退回时必填' : '暂停原因必填'} /></label>
                <footer style={commerceStyles.itemHeader}>
                  <small style={commerceStyles.itemMeta}>不会签发生产凭据</small>
                  <div style={commerceStyles.headerActions}>
                    {pending ? <>
                      <button style={actionStyle('secondary', busy)} type="button" onClick={() => review(item, 'changes_requested')} disabled={busy}><Undo2 size={13} />要求补充</button>
                      <button style={actionStyle('primary', busy)} type="button" onClick={() => review(item, 'approved')} disabled={busy}><Check size={13} />批准准入记录</button>
                    </> : <button style={actionStyle('danger', busy)} type="button" onClick={() => review(item, 'suspended')} disabled={busy}><PauseCircle size={13} />暂停准入</button>}
                  </div>
                </footer>
              </article>
            )
          })}
          {items.length === 0 && <p className={base.empty}>当前没有待审或已批准的准入记录。</p>}
        </div>
        {message && <div style={commerceStyles.message}>{message}</div>}
      </div>
    </section>
  )
}
