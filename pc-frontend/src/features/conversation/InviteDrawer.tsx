import { useState, useEffect } from 'react'
import { api } from '../../api/client'
import type { ProjectInviteLink, ProjectInviteLinksResponse, ProjectInviteResponse } from './types'
import { numberOrUndefined, inviteUrl, roleLabel, formatDateTime } from './memberUtils'
import styles from './ConversationPage.module.css'

export function InviteDrawer({
  projectId,
  onClose,
}: {
  projectId: string
  onClose: () => void
}) {
  const [invites, setInvites] = useState<ProjectInviteLink[]>([])
  const [role, setRole] = useState('member')
  const [expiresInHours, setExpiresInHours] = useState('168')
  const [maxUses, setMaxUses] = useState('')
  const [temporary, setTemporary] = useState(false)
  const [message, setMessage] = useState('')
  const [loading, setLoading] = useState(false)

  async function refreshInvites() {
    setLoading(true)
    try {
      const data = await api.get<ProjectInviteLinksResponse>(`/api/projects/${encodeURIComponent(projectId)}/invite-links`)
      setInvites(data.invites ?? [])
      setMessage('')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '邀请链接读取失败')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    refreshInvites()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId])

  async function createInvite() {
    setMessage('创建中…')
    try {
      const data = await api.post<ProjectInviteResponse>(`/api/projects/${encodeURIComponent(projectId)}/invite-links`, {
        role,
        expires_in_hours: numberOrUndefined(expiresInHours),
        max_uses: numberOrUndefined(maxUses),
        temporary,
      })
      if (data.invite) setInvites((items) => [data.invite as ProjectInviteLink, ...items])
      setMessage('已创建')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '创建失败')
    }
  }

  async function revokeInvite(code: string) {
    setMessage('撤销中…')
    try {
      await api.delete<ProjectInviteResponse>(`/api/projects/${encodeURIComponent(projectId)}/invite-links/${encodeURIComponent(code)}`)
      await refreshInvites()
      setMessage('已撤销')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '撤销失败')
    }
  }

  async function copyInvite(code: string) {
    try {
      await navigator.clipboard.writeText(inviteUrl(code))
      setMessage('已复制邀请链接')
    } catch {
      setMessage(inviteUrl(code))
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.inviteDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>邀请链接</strong>
            <span>{loading ? '同步中…' : message}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>
        <div className={styles.drawerBody}>
          <section className={styles.drawerSection}>
            <div className={styles.formGrid}>
              <label className={styles.field}>
                <span>加入角色</span>
                <input value={role} onChange={(event) => setRole(event.target.value)} placeholder="member" />
              </label>
              <label className={styles.field}>
                <span>有效小时</span>
                <input value={expiresInHours} onChange={(event) => setExpiresInHours(event.target.value)} inputMode="numeric" placeholder="空为永久" />
              </label>
              <label className={styles.field}>
                <span>最大次数</span>
                <input value={maxUses} onChange={(event) => setMaxUses(event.target.value)} inputMode="numeric" placeholder="空为不限" />
              </label>
              <label className={styles.checkField}>
                <input type="checkbox" checked={temporary} onChange={(event) => setTemporary(event.target.checked)} />
                <span>临时邀请</span>
              </label>
            </div>
            <div className={styles.actionRow}>
              <button className={styles.primaryBtn} onClick={createInvite}>创建链接</button>
            </div>
          </section>

          <section className={styles.drawerSection}>
            <strong className={styles.sectionTitle}>已创建</strong>
            <div className={styles.inviteList}>
              {invites.length === 0 && <p className={styles.sideHint}>暂无邀请链接</p>}
              {invites.map((invite) => (
                <article key={invite.id} className={styles.inviteRow}>
                  <div>
                    <strong>{inviteUrl(invite.code)}</strong>
                    <span>
                      {roleLabel(invite.role)} · {invite.use_count}/{invite.max_uses ?? '不限'} · {invite.revoked_at ? '已撤销' : invite.expires_at ? `过期 ${formatDateTime(invite.expires_at)}` : '永久'}
                    </span>
                  </div>
                  <button className={styles.drawerCloseBtn} onClick={() => copyInvite(invite.code)}>复制</button>
                  <button className={styles.dangerBtn} onClick={() => revokeInvite(invite.code)} disabled={!!invite.revoked_at}>撤销</button>
                </article>
              ))}
            </div>
          </section>
        </div>
      </section>
    </div>
  )
}
