import { useState, useEffect } from 'react'
import { api } from '../../api/client'
import { clean } from '../../lib/utils'
import type {
  Channel,
  ChannelCategory,
  ChannelPermissionResponse,
  ChannelPermissions,
  PermissionOption,
  PermissionOverride,
  ProjectMember,
  ProjectRole,
  ProjectRolesResponse,
} from './types'
import {
  CHANNEL_PERMISSION_OPTIONS,
  channelPermissionsChanged,
  memberChannelCanView,
  memberPresenceStatus,
  memberChannelSubtitleForPermissions,
  projectedMemberChannelPermissions,
  memberChannelPermissions,
  memberPrimaryRoleKey,
  memberRoleSummary,
  roleLabel,
} from './memberUtils'
import { compareMembersForPanel, memberAvatarRoleClass } from './MemberPanel'
import styles from './ConversationPage.module.css'

type PermissionEffect = '' | 'allow' | 'deny'

function permissionEffect(override: PermissionOverride, permission: string): PermissionEffect {
  if ((override.deny ?? []).includes(permission)) return 'deny'
  if ((override.allow ?? []).includes(permission)) return 'allow'
  return ''
}

function findOverride(overrides: PermissionOverride[], targetId: string, kind: 'role' | 'member') {
  const key = kind === 'role' ? 'role_id' : 'user_id'
  const altKey = kind === 'role' ? 'roleId' : 'userId'
  return overrides.find((override) => clean(String(override[key as keyof PermissionOverride] ?? override[altKey as keyof PermissionOverride] ?? '')) === targetId) ?? {} as PermissionOverride
}

function updateOverride(
  overrides: PermissionOverride[],
  targetId: string,
  kind: 'role' | 'member',
  permission: string,
  effect: PermissionEffect,
) {
  if (!targetId) return overrides
  const next = overrides.slice()
  const index = next.findIndex((override) => clean(String(kind === 'role' ? (override.role_id ?? override.roleId) : (override.user_id ?? override.userId))) === targetId)
  const current = index >= 0 ? next[index] : (kind === 'role' ? { role_id: targetId } : { user_id: targetId })
  const allow = new Set(current.allow ?? [])
  const deny = new Set(current.deny ?? [])
  allow.delete(permission)
  deny.delete(permission)
  if (effect === 'allow') allow.add(permission)
  if (effect === 'deny') deny.add(permission)
  const updated = { ...current, allow: Array.from(allow), deny: Array.from(deny) }
  if (index >= 0) next[index] = updated
  else next.push(updated)
  return next
}

/* ── PermissionDrawer ── */
export function PermissionDrawer({
  projectId,
  activeChannelId,
  channels,
  categories,
  members,
  onClose,
  onSaved,
}: {
  projectId: string
  activeChannelId: string
  channels: Channel[]
  categories: ChannelCategory[]
  members: ProjectMember[]
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [roles, setRoles] = useState<ProjectRole[]>([])
  const [permissionOptions, setPermissionOptions] = useState<PermissionOption[]>(CHANNEL_PERMISSION_OPTIONS)
  const activeChannel = channels.find((channel) => channel.id === activeChannelId) ?? channels[0]
  const [categoryId, setCategoryId] = useState(activeChannel?.category_id ?? categories[0]?.id ?? '')
  const [channelId, setChannelId] = useState(activeChannel?.id ?? '')
  const [categoryRoleOverrides, setCategoryRoleOverrides] = useState<PermissionOverride[]>([])
  const [categoryMemberOverrides, setCategoryMemberOverrides] = useState<PermissionOverride[]>([])
  const [channelRoleOverrides, setChannelRoleOverrides] = useState<PermissionOverride[]>([])
  const [channelMemberOverrides, setChannelMemberOverrides] = useState<PermissionOverride[]>([])
  const [memberId, setMemberId] = useState(members[0]?.user_id ?? '')
  const [loading, setLoading] = useState(false)
  const [status, setStatus] = useState('')

  useEffect(() => {
    api.get<ProjectRolesResponse>(`/api/projects/${encodeURIComponent(projectId)}/roles`)
      .then((data) => {
        setRoles(data.roles ?? [])
      })
      .catch((err: { message?: string }) => setStatus(err.message ?? '角色加载失败'))
  }, [projectId])

  useEffect(() => {
    if (!categoryId) return
    setLoading(true)
    api.get<ChannelPermissionResponse>(`/api/projects/${encodeURIComponent(projectId)}/channel-categories/${encodeURIComponent(categoryId)}/permissions`)
      .then((data) => {
        setCategoryRoleOverrides(data.overrides ?? [])
        setCategoryMemberOverrides(data.member_overrides ?? data.memberOverrides ?? [])
        if (data.permissions?.length) setPermissionOptions(data.permissions)
      })
      .catch((err: { message?: string }) => setStatus(err.message ?? '分类权限加载失败'))
      .finally(() => setLoading(false))
  }, [projectId, categoryId])

  useEffect(() => {
    if (!channelId) return
    setLoading(true)
    api.get<ChannelPermissionResponse>(`/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/permissions`)
      .then((data) => {
        setChannelRoleOverrides(data.overrides ?? [])
        setChannelMemberOverrides(data.member_overrides ?? data.memberOverrides ?? [])
        if (data.permissions?.length) setPermissionOptions(data.permissions)
      })
      .catch((err: { message?: string }) => setStatus(err.message ?? '频道权限加载失败'))
      .finally(() => setLoading(false))
  }, [projectId, channelId])

  function changeChannel(nextChannelId: string) {
    const channel = channels.find((item) => item.id === nextChannelId)
    setChannelId(nextChannelId)
    if (channel?.category_id) setCategoryId(channel.category_id)
  }

  async function saveRole(scope: 'category' | 'channel', roleId: string) {
    const overrides = scope === 'category' ? categoryRoleOverrides : channelRoleOverrides
    const targetId = scope === 'category' ? categoryId : channelId
    if (!targetId) return
    const override = findOverride(overrides, roleId, 'role')
    await savePermissions(scope, targetId, { role_id: roleId, allow: override.allow ?? [], deny: override.deny ?? [] })
  }

  async function saveMember(scope: 'category' | 'channel') {
    if (!memberId) return
    const overrides = scope === 'category' ? categoryMemberOverrides : channelMemberOverrides
    const targetId = scope === 'category' ? categoryId : channelId
    if (!targetId) return
    const override = findOverride(overrides, memberId, 'member')
    await savePermissions(scope, targetId, { member_id: memberId, allow: override.allow ?? [], deny: override.deny ?? [] })
  }

  async function savePermissions(scope: 'category' | 'channel', targetId: string, body: unknown) {
    setStatus('保存中…')
    const base = scope === 'category'
      ? `/api/projects/${encodeURIComponent(projectId)}/channel-categories/${encodeURIComponent(targetId)}/permissions`
      : `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(targetId)}/permissions`
    try {
      const data = await api.patch<ChannelPermissionResponse>(base, body)
      if (scope === 'category') {
        setCategoryRoleOverrides(data.overrides ?? [])
        setCategoryMemberOverrides(data.member_overrides ?? data.memberOverrides ?? [])
      } else {
        setChannelRoleOverrides(data.overrides ?? [])
        setChannelMemberOverrides(data.member_overrides ?? data.memberOverrides ?? [])
      }
      setStatus('已保存')
      await onSaved()
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '保存失败')
    }
  }

  const selectedMember = members.find((member) => member.user_id === memberId)
  const selectedChannel = channels.find((channel) => channel.id === channelId)

  return (
    <div className={styles.drawerBackdrop}>
      <section className={styles.permissionDrawer} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>成员权限</strong>
            <span>{loading ? '同步中…' : status}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>

        <div className={styles.permissionColumns}>
          <section className={styles.permissionBlock}>
            <div className={styles.permissionToolbar}>
              <strong>分类权限</strong>
              <select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}>
                {categories.map((category) => (
                  <option key={category.id} value={category.id}>{category.name || category.kind || category.id}</option>
                ))}
              </select>
            </div>
            <PermissionRoleGrid
              roles={roles}
              options={permissionOptions}
              overrides={categoryRoleOverrides}
              onChange={(roleId, permission, effect) => setCategoryRoleOverrides(updateOverride(categoryRoleOverrides, roleId, 'role', permission, effect))}
              onSave={(roleId) => saveRole('category', roleId)}
            />
            <PermissionMemberGrid
              member={selectedMember}
              members={members}
              memberId={memberId}
              options={permissionOptions}
              overrides={categoryMemberOverrides}
              onMemberChange={setMemberId}
              onChange={(permission, effect) => setCategoryMemberOverrides(updateOverride(categoryMemberOverrides, memberId, 'member', permission, effect))}
              onSave={() => saveMember('category')}
            />
          </section>

          <section className={styles.permissionBlock}>
            <div className={styles.permissionToolbar}>
              <strong>频道覆盖</strong>
              <select value={channelId} onChange={(event) => changeChannel(event.target.value)}>
                {channels.map((channel) => (
                  <option key={channel.id} value={channel.id}>{channel.name}</option>
                ))}
              </select>
            </div>
            <ChannelMemberPermissionPreview
              channel={selectedChannel}
              channelId={channelId}
              categoryId={categoryId}
              roles={roles}
              members={members}
              categoryRoleOverrides={categoryRoleOverrides}
              categoryMemberOverrides={categoryMemberOverrides}
              channelRoleOverrides={channelRoleOverrides}
              channelMemberOverrides={channelMemberOverrides}
            />
            <PermissionRoleGrid
              roles={roles}
              options={permissionOptions}
              overrides={channelRoleOverrides}
              onChange={(roleId, permission, effect) => setChannelRoleOverrides(updateOverride(channelRoleOverrides, roleId, 'role', permission, effect))}
              onSave={(roleId) => saveRole('channel', roleId)}
            />
            <PermissionMemberGrid
              member={selectedMember}
              members={members}
              memberId={memberId}
              options={permissionOptions}
              overrides={channelMemberOverrides}
              onMemberChange={setMemberId}
              onChange={(permission, effect) => setChannelMemberOverrides(updateOverride(channelMemberOverrides, memberId, 'member', permission, effect))}
              onSave={() => saveMember('channel')}
            />
          </section>
        </div>
      </section>
    </div>
  )
}

/* ── ChannelMemberPermissionPreview ── */
function ChannelMemberPermissionPreview({
  channel,
  channelId,
  categoryId,
  roles,
  members,
  categoryRoleOverrides,
  categoryMemberOverrides,
  channelRoleOverrides,
  channelMemberOverrides,
}: {
  channel?: Channel
  channelId: string
  categoryId?: string
  roles: ProjectRole[]
  members: ProjectMember[]
  categoryRoleOverrides: PermissionOverride[]
  categoryMemberOverrides: PermissionOverride[]
  channelRoleOverrides: PermissionOverride[]
  channelMemberOverrides: PermissionOverride[]
}) {
  const entries = members.map((member) => {
    const permissions = projectedMemberChannelPermissions({
      member,
      channel,
      categoryId,
      roles,
      categoryRoleOverrides,
      categoryMemberOverrides,
      channelRoleOverrides,
      channelMemberOverrides,
    })
    const savedPermissions = memberChannelPermissions(member, channelId)
    return {
      member,
      permissions,
      changed: channelPermissionsChanged(permissions, savedPermissions),
    }
  })
  const visibleEntries = entries
    .filter((entry) => memberChannelCanView(entry.permissions))
    .sort((left, right) => compareMembersForPanel(left.member, right.member))
  const hiddenEntries = entries
    .filter((entry) => !memberChannelCanView(entry.permissions))
    .sort((left, right) => compareMembersForPanel(left.member, right.member))
  const onlineCount = visibleEntries.filter((entry) => memberPresenceStatus(entry.member) !== 'offline').length
  const changedCount = entries.filter((entry) => entry.changed).length
  const previewEntries = visibleEntries.slice(0, 8)
  const hiddenPreview = hiddenEntries.slice(0, 3)

  return (
    <article className={styles.permissionPreview}>
      <div className={styles.permissionPreviewHead}>
        <div>
          <strong>频道成员预览</strong>
          <span>{channel?.name ?? '当前频道'}</span>
        </div>
        <div className={styles.permissionPreviewStats}>
          <em>预览可见 {visibleEntries.length}</em>
          <em>隐藏 {hiddenEntries.length}</em>
          <em>在线 {onlineCount}</em>
          {changedCount > 0 && <em>未保存 {changedCount}</em>}
        </div>
      </div>
      <span className={styles.permissionPreviewNote}>根据当前表单实时预估，保存后会与服务端权限重新同步。</span>
      <div className={styles.permissionPreviewList}>
        {previewEntries.map((entry) => (
          <ChannelMemberPreviewRow
            key={entry.member.user_id}
            member={entry.member}
            permissions={entry.permissions}
            changed={entry.changed}
          />
        ))}
        {hiddenPreview.map((entry) => (
          <ChannelMemberPreviewRow
            key={`hidden-${entry.member.user_id}`}
            member={entry.member}
            permissions={entry.permissions}
            changed={entry.changed}
            hidden
          />
        ))}
        {previewEntries.length === 0 && hiddenPreview.length === 0 && (
          <p className={styles.sideHint}>暂无成员</p>
        )}
      </div>
    </article>
  )
}

/* ── ChannelMemberPreviewRow ── */
function ChannelMemberPreviewRow({
  member,
  permissions,
  changed,
  hidden,
}: {
  member: ProjectMember
  permissions: ChannelPermissions
  changed?: boolean
  hidden?: boolean
}) {
  const roleKey = memberPrimaryRoleKey(member)
  const name = member.account ?? member.user_id
  const avatarCls = [
    styles.memberAvatar,
    memberAvatarRoleClass(roleKey),
    memberPresenceStatus(member) === 'offline' ? styles.memberAvatarOffline : styles.memberAvatarOnline,
  ].filter(Boolean).join(' ')
  return (
    <div className={[styles.permissionPreviewRow, hidden ? styles.permissionPreviewRowHidden : ''].filter(Boolean).join(' ')}>
      <div className={avatarCls}>
        {member.avatar_data_url
          ? <img src={member.avatar_data_url} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
          : name[0]?.toUpperCase()
        }
      </div>
      <div>
        <strong>{name}{changed && <em>未保存</em>}</strong>
        <span>{memberChannelSubtitleForPermissions(member, permissions)}</span>
      </div>
    </div>
  )
}

/* ── PermissionRoleGrid ── */
function PermissionRoleGrid({
  roles,
  options,
  overrides,
  onChange,
  onSave,
}: {
  roles: ProjectRole[]
  options: PermissionOption[]
  overrides: PermissionOverride[]
  onChange: (roleId: string, permission: string, effect: PermissionEffect) => void
  onSave: (roleId: string) => void
}) {
  return (
    <div className={styles.permissionCards}>
      {roles.map((role) => (
        <article key={role.id} className={styles.permissionCard}>
          <div className={styles.permissionCardHead}>
            <span className={styles.roleSwatch} style={{ background: role.color ?? '#747f8d' }} />
            <strong>{role.name || roleLabel(role.id)}</strong>
          </div>
          <PermissionGrid
            options={options}
            override={findOverride(overrides, role.id, 'role')}
            onChange={(permission, effect) => onChange(role.id, permission, effect)}
          />
          <button className={styles.savePermissionBtn} onClick={() => onSave(role.id)}>保存</button>
        </article>
      ))}
    </div>
  )
}

/* ── PermissionMemberGrid ── */
function PermissionMemberGrid({
  member,
  members,
  memberId,
  options,
  overrides,
  onMemberChange,
  onChange,
  onSave,
}: {
  member?: ProjectMember
  members: ProjectMember[]
  memberId: string
  options: PermissionOption[]
  overrides: PermissionOverride[]
  onMemberChange: (memberId: string) => void
  onChange: (permission: string, effect: PermissionEffect) => void
  onSave: () => void
}) {
  return (
    <article className={styles.permissionCard}>
      <div className={styles.permissionToolbar}>
        <strong>成员覆盖</strong>
        <select value={memberId} onChange={(event) => onMemberChange(event.target.value)}>
          {members.map((item) => (
            <option key={item.user_id} value={item.user_id}>{item.account || item.user_id}</option>
          ))}
        </select>
      </div>
      {member && <small className={styles.permissionMemberName}>{memberRoleSummary(member)}</small>}
      <PermissionGrid
        options={options}
        override={findOverride(overrides, memberId, 'member')}
        onChange={onChange}
      />
      <button className={styles.savePermissionBtn} onClick={onSave} disabled={!memberId}>保存</button>
    </article>
  )
}

/* ── PermissionGrid ── */
function PermissionGrid({
  options,
  override,
  onChange,
}: {
  options: PermissionOption[]
  override: PermissionOverride
  onChange: (permission: string, effect: PermissionEffect) => void
}) {
  return (
    <div className={styles.permissionGrid}>
      {options.map((option) => (
        <label key={option.key}>
          <span>{option.label}</span>
          <select value={permissionEffect(override, option.key)} onChange={(event) => onChange(option.key, event.target.value as PermissionEffect)}>
            <option value="">继承</option>
            <option value="allow">允许</option>
            <option value="deny">拒绝</option>
          </select>
        </label>
      ))}
    </div>
  )
}
