import { useState, useEffect, useMemo } from 'react'
import { api } from '../../api/client'
import type {
  ProjectInviteLink,
  ProjectInviteLinksResponse,
  ProjectInviteResponse,
  ProjectRole,
  ProjectRolesResponse,
} from './types'
import { numberOrUndefined, inviteUrl, roleLabel, formatDateTime } from './memberUtils'
import sharedStyles from './ConversationPage.module.css'
import inviteStyles from './InviteDrawer.module.css'

const styles = { ...sharedStyles, ...inviteStyles }

type InviteState = 'active' | 'revoked' | 'expired' | 'full'
type InviteFilter = 'active' | 'all' | InviteState

const INVITE_FILTERS: Array<{ id: InviteFilter; label: string }> = [
  { id: 'active', label: '有效' },
  { id: 'all', label: '全部' },
  { id: 'revoked', label: '撤销' },
  { id: 'expired', label: '过期' },
  { id: 'full', label: '已满' },
]

const DEFAULT_ROLE_OPTIONS: ProjectRole[] = [
  { id: 'admin', name: '管理员', position: 80, builtin: true },
  { id: 'editor', name: '协作者', position: 60, builtin: true },
  { id: 'member', name: '成员', position: 40, builtin: true },
  { id: 'observer', name: '只读成员', position: 20, builtin: true },
]

const INVITE_PRESETS = [
  { id: 'week-member', label: '7天成员', role: 'member', expiresInHours: '168', maxUses: '', temporary: false },
  { id: 'single-observer', label: '一次性观察', role: 'observer', expiresInHours: '24', maxUses: '1', temporary: true },
  { id: 'permanent-member', label: '永久成员', role: 'member', expiresInHours: '', maxUses: '', temporary: false },
  { id: 'collaborator', label: '协作开发', role: 'editor', expiresInHours: '168', maxUses: '10', temporary: false },
]

export function InviteDrawer({
  projectId,
  onClose,
}: {
  projectId: string
  onClose: () => void
}) {
  const [invites, setInvites] = useState<ProjectInviteLink[]>([])
  const [roles, setRoles] = useState<ProjectRole[]>([])
  const [role, setRole] = useState('member')
  const [expiresInHours, setExpiresInHours] = useState('168')
  const [maxUses, setMaxUses] = useState('')
  const [temporary, setTemporary] = useState(false)
  const [filter, setFilter] = useState<InviteFilter>('active')
  const [query, setQuery] = useState('')
  const [message, setMessage] = useState('')
  const [loading, setLoading] = useState(false)
  const [creating, setCreating] = useState(false)
  const [busyCode, setBusyCode] = useState('')
  const [createdInvite, setCreatedInvite] = useState<ProjectInviteLink | null>(null)
  const roleOptions = useMemo(() => inviteRoleOptions(roles), [roles])
  const inviteStats = useMemo(() => inviteStateStats(invites), [invites])
  const visibleInvites = useMemo(
    () => filterInvites(invites, filter, query),
    [filter, invites, query],
  )

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

  async function refreshRoles() {
    try {
      const data = await api.get<ProjectRolesResponse>(`/api/projects/${encodeURIComponent(projectId)}/roles`)
      setRoles(data.roles ?? [])
    } catch {
      setRoles([])
    }
  }

  useEffect(() => {
    refreshInvites()
    refreshRoles()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId])

  async function createInvite() {
    setMessage('创建中…')
    setCreating(true)
    try {
      const data = await api.post<ProjectInviteResponse>(`/api/projects/${encodeURIComponent(projectId)}/invite-links`, {
        role,
        expires_in_hours: numberOrUndefined(expiresInHours),
        max_uses: numberOrUndefined(maxUses),
        temporary,
      })
      if (data.invite) {
        setInvites((items) => [data.invite as ProjectInviteLink, ...items])
        setCreatedInvite(data.invite)
      }
      setMessage('已创建')
      setFilter('active')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '创建失败')
    } finally {
      setCreating(false)
    }
  }

  async function revokeInvite(code: string) {
    setMessage('撤销中…')
    setBusyCode(code)
    try {
      await api.delete<ProjectInviteResponse>(`/api/projects/${encodeURIComponent(projectId)}/invite-links/${encodeURIComponent(code)}`)
      await refreshInvites()
      setCreatedInvite((current) => current?.code === code ? { ...current, revoked_at: new Date().toISOString() } : current)
      setMessage('已撤销')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '撤销失败')
    } finally {
      setBusyCode('')
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

  function applyPreset(preset: typeof INVITE_PRESETS[number]) {
    setRole(preset.role)
    setExpiresInHours(preset.expiresInHours)
    setMaxUses(preset.maxUses)
    setTemporary(preset.temporary)
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.inviteDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>邀请中心</strong>
            <span>{loading ? '同步中…' : message || `${inviteStats.active} 条有效 · ${inviteStats.all} 条总计`}</span>
          </div>
          <div className={styles.drawerHeaderActions}>
            <button className={styles.drawerCloseBtn} onClick={refreshInvites} disabled={loading}>刷新</button>
            <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
          </div>
        </header>
        <div className={styles.drawerBody}>
          <div className={styles.inviteStats}>
            {INVITE_FILTERS.map((item) => (
              <button
                key={item.id}
                type="button"
                data-active={filter === item.id ? 'true' : undefined}
                onClick={() => setFilter(item.id)}
              >
                <strong>{inviteStats[item.id] ?? 0}</strong>
                <span>{item.label}</span>
              </button>
            ))}
          </div>

          {createdInvite && (
            <section className={styles.inviteSpotlight} data-state={inviteState(createdInvite)}>
              <div className={styles.inviteSpotlightMain}>
                <strong>最新邀请链接</strong>
                <span>{inviteUrl(createdInvite.code)}</span>
                <div className={styles.inviteMeta}>
                  <em>{roleLabel(createdInvite.role)}</em>
                  <em>{inviteUseCopy(createdInvite)}</em>
                  <em>{inviteExpiresCopy(createdInvite)}</em>
                  {createdInvite.temporary && <em>临时邀请</em>}
                </div>
              </div>
              <div className={styles.inviteRowActions}>
                <button className={styles.drawerCloseBtn} onClick={() => copyInvite(createdInvite.code)}>复制链接</button>
                <button className={styles.drawerCloseBtn} onClick={() => setCreatedInvite(null)}>收起</button>
              </div>
            </section>
          )}

          <section className={styles.drawerSection}>
            <strong className={styles.sectionTitle}>创建邀请</strong>
            <div className={styles.invitePresetGrid}>
              {INVITE_PRESETS.map((preset) => (
                <button key={preset.id} type="button" onClick={() => applyPreset(preset)}>
                  {preset.label}
                </button>
              ))}
            </div>
            <div className={styles.formGrid}>
              <label className={styles.field}>
                <span>加入角色</span>
                <select value={role} onChange={(event) => setRole(event.target.value)}>
                  {roleOptions.map((option) => (
                    <option key={option.id} value={option.id}>{option.name || roleLabel(option.id)}</option>
                  ))}
                </select>
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
              <button className={styles.primaryBtn} onClick={createInvite} disabled={creating}>创建链接</button>
            </div>
          </section>

          <section className={styles.drawerSection}>
            <div className={styles.inviteToolbar}>
              <strong className={styles.sectionTitle}>邀请列表</strong>
              <input
                className={styles.drawerSearchInput}
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索邀请码或角色"
              />
            </div>
            <div className={styles.inviteList}>
              {visibleInvites.length === 0 && <p className={styles.sideHint}>暂无匹配邀请链接</p>}
              {visibleInvites.map((invite) => {
                const state = inviteState(invite)
                const disabled = state !== 'active' || busyCode === invite.code
                return (
                  <article key={invite.id} className={styles.inviteRow} data-state={state}>
                    <div className={styles.inviteRowMain}>
                      <div className={styles.inviteRowTitle}>
                        <strong>{inviteUrl(invite.code)}</strong>
                        <em className={styles.inviteStatusPill} data-state={state}>{inviteStateLabel(state)}</em>
                      </div>
                      <span>
                        {roleLabel(invite.role)} · {inviteUseCopy(invite)} · {inviteExpiresCopy(invite)}
                      </span>
                      <div className={styles.inviteMeta}>
                        <em>邀请码 {invite.code}</em>
                        {invite.created_at && <em>创建 {formatDateTime(invite.created_at)}</em>}
                        {invite.temporary && <em>临时邀请</em>}
                      </div>
                    </div>
                    <div className={styles.inviteRowActions}>
                      <button className={styles.drawerCloseBtn} onClick={() => copyInvite(invite.code)}>复制</button>
                      <button className={styles.dangerBtn} onClick={() => revokeInvite(invite.code)} disabled={disabled}>撤销</button>
                    </div>
                  </article>
                )
              })}
            </div>
          </section>
        </div>
      </section>
    </div>
  )
}

function inviteRoleOptions(roles: ProjectRole[]) {
  const options = [...DEFAULT_ROLE_OPTIONS, ...roles]
  const seen = new Set<string>()
  return options
    .filter((role) => role.id !== 'owner')
    .sort((left, right) => (right.position ?? 0) - (left.position ?? 0) || left.id.localeCompare(right.id))
    .filter((role) => {
      const id = role.id.trim()
      if (!id || seen.has(id)) return false
      seen.add(id)
      return true
    })
}

function inviteState(invite: ProjectInviteLink): InviteState {
  if (invite.revoked_at) return 'revoked'
  if (invite.expires_at) {
    const expiresAt = Date.parse(invite.expires_at)
    if (!Number.isNaN(expiresAt) && expiresAt <= Date.now()) return 'expired'
  }
  if (typeof invite.max_uses === 'number' && invite.use_count >= invite.max_uses) return 'full'
  return 'active'
}

function inviteStateLabel(state: InviteState) {
  const labels: Record<InviteState, string> = {
    active: '有效',
    revoked: '已撤销',
    expired: '已过期',
    full: '已用满',
  }
  return labels[state]
}

function inviteStateStats(invites: ProjectInviteLink[]) {
  const stats: Record<InviteFilter, number> = {
    active: 0,
    all: invites.length,
    revoked: 0,
    expired: 0,
    full: 0,
  }
  invites.forEach((invite) => {
    stats[inviteState(invite)] += 1
  })
  return stats
}

function filterInvites(invites: ProjectInviteLink[], filter: InviteFilter, query: string) {
  const q = query.trim().toLowerCase()
  return invites.filter((invite) => {
    const state = inviteState(invite)
    if (filter !== 'all' && state !== filter) return false
    if (!q) return true
    const haystack = [
      invite.code,
      invite.role,
      roleLabel(invite.role),
      inviteUrl(invite.code),
      inviteStateLabel(state),
    ].join(' ').toLowerCase()
    return haystack.includes(q)
  })
}

function inviteUseCopy(invite: ProjectInviteLink) {
  return `${invite.use_count}/${invite.max_uses ?? '不限'} 次`
}

function inviteExpiresCopy(invite: ProjectInviteLink) {
  if (!invite.expires_at) return '永久有效'
  return `过期 ${formatDateTime(invite.expires_at)}`
}
