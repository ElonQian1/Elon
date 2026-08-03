import { useCallback, useEffect, useState } from 'react'
import { Check, RefreshCw, Undo2 } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { OpenCommerceDeveloperApp } from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  listItemStyle,
} from './openCommerceStyles'

export default function DeveloperAppManifestReviewPanel() {
  const [apps, setApps] = useState<OpenCommerceDeveloperApp[]>([])
  const [notes, setNotes] = useState<Record<string, string>>({})
  const [busyId, setBusyId] = useState('')
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listSubmittedDeveloperAppManifests()
      setApps(response.apps)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function review(
    app: OpenCommerceDeveloperApp,
    decision: 'approved' | 'changes_requested',
  ) {
    const note = notes[app.id]?.trim() ?? ''
    if (decision === 'changes_requested' && !note) {
      setMessage('要求修改时必须填写审核说明。')
      return
    }
    setBusyId(app.id)
    setMessage('')
    try {
      await openCommerceClientApi.reviewDeveloperAppManifest(app.id, {
        expected_manifest_revision: app.manifest_revision,
        decision,
        note,
      })
      setNotes((current) => ({ ...current, [app.id]: '' }))
      setMessage(decision === 'approved' ? '资料审核已通过。' : '修改要求已退回给 App 所有者。')
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
        <strong>平台 App 资料审核</strong>
        <div style={commerceStyles.headerActions}>
          <span style={badgeStyle(apps.length ? 'warn' : 'neutral')}>{apps.length} 待审</span>
          <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新待审队列"><RefreshCw size={13} /></button>
        </div>
      </header>
      <div className={base.formCard} style={commerceStyles.sectionBody}>
        <div style={commerceStyles.list}>
          {apps.map((app) => {
            const busy = busyId === app.id
            return (
              <article className={base.formCard} style={listItemStyle()} key={app.id}>
                <header style={commerceStyles.itemHeader}>
                  <h3 style={commerceStyles.itemTitle}>{app.display_name}</h3>
                  <span style={badgeStyle('warn')}>R{app.manifest_revision}</span>
                </header>
                <code style={commerceStyles.itemMeta}>{app.app_id} · 项目 {app.project_id}</code>
                <p style={commerceStyles.itemText}>主页：{app.homepage_url}</p>
                <p style={commerceStyles.itemText}>隐私政策：{app.privacy_policy_url}</p>
                <p style={commerceStyles.itemText}>服务条款：{app.terms_url}</p>
                <p style={commerceStyles.itemText}>支持邮箱：{app.support_email}</p>
                <code style={commerceStyles.itemMeta}>{app.requested_scopes.join(', ')}</code>
                <label>审核说明<textarea value={notes[app.id] ?? ''} onChange={(event) => setNotes((current) => ({ ...current, [app.id]: event.target.value }))} disabled={busy} placeholder="批准时可选；要求修改时必填" /></label>
                <footer style={commerceStyles.itemHeader}>
                  <small style={commerceStyles.itemMeta}>资料审核不签发生产凭据</small>
                  <div style={commerceStyles.headerActions}>
                    <button style={actionStyle('secondary', busy)} type="button" onClick={() => review(app, 'changes_requested')} disabled={busy}><Undo2 size={13} />要求修改</button>
                    <button style={actionStyle('primary', busy)} type="button" onClick={() => review(app, 'approved')} disabled={busy}><Check size={13} />批准资料</button>
                  </div>
                </footer>
              </article>
            )
          })}
          {apps.length === 0 && <p className={base.empty}>当前没有待审核的 App 资料。</p>}
        </div>
        {message && <div style={commerceStyles.message}>{message}</div>}
      </div>
    </section>
  )
}
