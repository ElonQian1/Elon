import { useEffect, useMemo, useState } from 'react'
import { api } from '../../api/client'
import { clean } from '../../lib/utils'
import type { PermissionOption, ProjectMember, ProjectRole, ProjectRolesResponse } from './types'
import { filterMembers, memberInitial, memberRoleSummary, roleLabel } from './memberUtils'
import styles from './ConversationPage.module.css'

type RoleResponse = {
  role?: ProjectRole
  roles?: ProjectRole[]
}

const DEFAULT_ROLE_COLOR = '#5865f2'

export function RoleManagementDrawer({
  projectId,
  members,
  currentUserId,
  initialMemberId,
  canManageRoles = true,
  canManageMembers = true,
  onClose,
  onSaved,
}: {
  projectId: string
  members: ProjectMember[]
  currentUserId?: string
  initialMemberId?: string
  canManageRoles?: boolean
  canManageMembers?: boolean
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [roles, setRoles] = useState<ProjectRole[]>([])
  const [permissions, setPermissions] = useState<PermissionOption[]>([])
  const [selectedRoleId, setSelectedRoleId] = useState('')
  const [roleName, setRoleName] = useState('')
  const [roleColor, setRoleColor] = useState(DEFAULT_ROLE_COLOR)
  const [rolePosition, setRolePosition] = useState('30')
  const [rolePermissions, setRolePermissions] = useState<string[]>([])
  const [newRoleName, setNewRoleName] = useState('')
  const [memberQuery, setMemberQuery] = useState('')
  const [selectedMemberId, setSelectedMemberId] = useState(members[0]?.user_id ?? '')
  const [selectedMemberRoles, setSelectedMemberRoles] = useState<string[]>([])
  const [loading, setLoading] = useState(false)
  const [status, setStatus] = useState('')

  async function refreshRoles() {
    setLoading(true)
    try {
      const data = await api.get<ProjectRolesResponse>(`/api/projects/${encodeURIComponent(projectId)}/roles`)
      const nextRoles = data.roles ?? []
      setRoles(nextRoles)
      setPermissions(data.permissions ?? [])
      setSelectedRoleId((current) => nextRoles.some((role) => role.id === current) ? current : (nextRoles.find((role) => !role.builtin)?.id ?? nextRoles[0]?.id ?? ''))
      setStatus('')
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '角色读取失败')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    refreshRoles()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId])

  const selectedRole = roles.find((role) => role.id === selectedRoleId)
  const selectedMember = members.find((member) => member.user_id === selectedMemberId)
  const visibleMembers = useMemo(() => filterMembers(members, memberQuery), [members, memberQuery])

  useEffect(() => {
    if (initialMemberId && members.some((member) => member.user_id === initialMemberId)) {
      setSelectedMemberId(initialMemberId)
      setMemberQuery('')
    }
  }, [initialMemberId, members])

  useEffect(() => {
    if (!selectedRole) {
      setRoleName('')
      setRoleColor(DEFAULT_ROLE_COLOR)
      setRolePosition('30')
      setRolePermissions([])
      return
    }
    setRoleName(selectedRole.name || roleLabel(selectedRole.id))
    setRoleColor(selectedRole.color || DEFAULT_ROLE_COLOR)
    setRolePosition(String(selectedRole.position ?? 30))
    setRolePermissions(selectedRole.permissions ?? [])
  }, [selectedRole])

  useEffect(() => {
    if (!selectedMemberId || !members.some((member) => member.user_id === selectedMemberId)) {
      setSelectedMemberId(members[0]?.user_id ?? '')
    }
  }, [members, selectedMemberId])

  useEffect(() => {
    const member = members.find((item) => item.user_id === selectedMemberId)
    setSelectedMemberRoles(memberRoleIds(member))
  }, [members, selectedMemberId])

  function toggleRolePermission(permission: string) {
    setRolePermissions((current) =>
      current.includes(permission)
        ? current.filter((item) => item !== permission)
        : [...current, permission],
    )
  }

  function toggleMemberRole(roleId: string) {
    setSelectedMemberRoles((current) =>
      current.includes(roleId)
        ? current.filter((item) => item !== roleId)
        : [...current, roleId],
    )
  }

  async function createRole() {
    if (!canManageRoles) {
      setStatus('当前角色无权管理角色定义')
      return
    }
    const name = clean(newRoleName)
    if (!name) return
    setStatus('创建中...')
    try {
      const data = await api.post<RoleResponse>(`/api/projects/${encodeURIComponent(projectId)}/roles`, {
        name,
        color: DEFAULT_ROLE_COLOR,
        position: 30,
        permissions: ['view_members', 'send_messages'],
      })
      setNewRoleName('')
      await refreshRoles()
      if (data.role?.id) setSelectedRoleId(data.role.id)
      setStatus('已创建')
      await onSaved()
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '创建失败')
    }
  }

  async function saveRole() {
    if (!selectedRole || selectedRole.builtin) return
    if (!canManageRoles) {
      setStatus('当前角色无权管理角色定义')
      return
    }
    setStatus('保存角色中...')
    try {
      const data = await api.patch<RoleResponse>(`/api/projects/${encodeURIComponent(projectId)}/roles/${encodeURIComponent(selectedRole.id)}`, {
        name: roleName,
        color: roleColor || null,
        position: Number(rolePosition) || selectedRole.position || 30,
        permissions: rolePermissions,
      })
      await refreshRoles()
      if (data.role?.id) setSelectedRoleId(data.role.id)
      setStatus('角色已保存')
      await onSaved()
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '保存失败')
    }
  }

  async function deleteRole() {
    if (!selectedRole || selectedRole.builtin) return
    if (!canManageRoles) {
      setStatus('当前角色无权管理角色定义')
      return
    }
    if (!window.confirm(`确认删除角色「${selectedRole.name || selectedRole.id}」？`)) return
    setStatus('删除中...')
    try {
      await api.delete<RoleResponse>(`/api/projects/${encodeURIComponent(projectId)}/roles/${encodeURIComponent(selectedRole.id)}`)
      setSelectedRoleId('')
      await refreshRoles()
      setStatus('角色已删除')
      await onSaved()
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '删除失败')
    }
  }

  async function saveMemberRoles() {
    if (!selectedMember) return
    if (!canManageMembers) {
      setStatus('当前角色无权修改成员角色')
      return
    }
    if (selectedMemberRoles.length === 0) {
      setStatus('成员至少需要一个角色')
      return
    }
    setStatus('保存成员角色中...')
    try {
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(selectedMember.user_id)}`, {
        roles: selectedMemberRoles,
      })
      setStatus('成员角色已保存')
      await onSaved()
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '保存失败')
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.roleDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>角色管理</strong>
            <span>{loading ? '同步中...' : status || '管理项目角色、权限和成员角色'}</span>
          </div>
          <div className={styles.drawerHeaderActions}>
            <button className={styles.drawerCloseBtn} onClick={refreshRoles} disabled={loading}>刷新</button>
            <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
          </div>
        </header>

        <div className={styles.roleManagerBody}>
          <section className={styles.roleManagerColumn}>
            <div className={styles.roleCreateRow}>
              <input value={newRoleName} onChange={(event) => setNewRoleName(event.target.value)} placeholder="新角色名称" />
              <button className={styles.primaryBtn} onClick={createRole} disabled={!canManageRoles || !clean(newRoleName)}>创建</button>
            </div>
            <div className={styles.roleList}>
              {roles.map((role) => (
                <button
                  key={role.id}
                  className={[styles.roleListItem, role.id === selectedRoleId ? styles.roleListItemActive : ''].join(' ')}
                  type="button"
                  onClick={() => setSelectedRoleId(role.id)}
                >
                  <span className={styles.roleSwatch} style={{ background: role.color || '#747f8d' }} />
                  <strong>{role.name || roleLabel(role.id)}</strong>
                  <em>{role.member_count ?? 0}</em>
                </button>
              ))}
            </div>
            <RoleEditor
              role={selectedRole}
              permissions={permissions}
              roleName={roleName}
              roleColor={roleColor}
              rolePosition={rolePosition}
              rolePermissions={rolePermissions}
              onNameChange={setRoleName}
              onColorChange={setRoleColor}
              onPositionChange={setRolePosition}
              onTogglePermission={toggleRolePermission}
              onSave={saveRole}
              onDelete={deleteRole}
              canManageRoles={canManageRoles}
            />
          </section>

          <section className={styles.roleManagerColumn}>
            <input
              className={styles.drawerSearchInput}
              value={memberQuery}
              onChange={(event) => setMemberQuery(event.target.value)}
              placeholder="搜索成员"
            />
            <div className={styles.roleMemberList}>
              {visibleMembers.map((member) => (
                <button
                  key={member.user_id}
                  className={[styles.roleMemberItem, member.user_id === selectedMemberId ? styles.roleMemberItemActive : ''].join(' ')}
                  type="button"
                  onClick={() => setSelectedMemberId(member.user_id)}
                >
                  <span className={styles.memberAvatar}>{memberInitial(member)}</span>
                  <span>
                    <strong>{member.account || member.user_id}</strong>
                    <em>{memberRoleSummary(member)}</em>
                  </span>
                </button>
              ))}
              {visibleMembers.length === 0 && <p className={styles.sideHint}>没有匹配成员</p>}
            </div>
            <div className={styles.roleAssignmentPanel}>
              <div className={styles.roleAssignmentHead}>
                <strong>{selectedMember ? selectedMember.account || selectedMember.user_id : '选择成员'}</strong>
                <span>{selectedMember ? memberRoleSummary(selectedMember) : '从上方成员列表选择'}</span>
              </div>
              <div className={styles.roleCheckGrid}>
                {roles.map((role) => (
                  <label key={role.id} className={styles.roleCheckItem}>
                    <input
                      type="checkbox"
                      checked={selectedMemberRoles.includes(role.id)}
                      onChange={() => toggleMemberRole(role.id)}
                      disabled={!canManageMembers || !selectedMember || selectedMember.user_id === currentUserId}
                    />
                    <span className={styles.roleSwatch} style={{ background: role.color || '#747f8d' }} />
                    <strong>{role.name || roleLabel(role.id)}</strong>
                  </label>
                ))}
              </div>
              <div className={styles.actionRow}>
                <button className={styles.primaryBtn} onClick={saveMemberRoles} disabled={!canManageMembers || !selectedMember || selectedMember.user_id === currentUserId}>
                  保存成员角色
                </button>
              </div>
              {!canManageMembers && (
                <p className={styles.sideHint}>当前角色只能查看成员角色，不能修改。</p>
              )}
              {selectedMember?.user_id === currentUserId && (
                <p className={styles.sideHint}>不能在这里修改自己的角色。</p>
              )}
            </div>
          </section>
        </div>
      </section>
    </div>
  )
}

function RoleEditor({
  role,
  permissions,
  roleName,
  roleColor,
  rolePosition,
  rolePermissions,
  onNameChange,
  onColorChange,
  onPositionChange,
  onTogglePermission,
  onSave,
  onDelete,
  canManageRoles,
}: {
  role?: ProjectRole
  permissions: PermissionOption[]
  roleName: string
  roleColor: string
  rolePosition: string
  rolePermissions: string[]
  onNameChange: (value: string) => void
  onColorChange: (value: string) => void
  onPositionChange: (value: string) => void
  onTogglePermission: (permission: string) => void
  onSave: () => void
  onDelete: () => void
  canManageRoles: boolean
}) {
  if (!role) return <p className={styles.sideHint}>暂无角色</p>
  const readOnly = !!role.builtin || !canManageRoles
  return (
    <div className={styles.roleEditor}>
      <div className={styles.roleEditorHead}>
        <strong>{readOnly ? '内置角色' : '编辑角色'}</strong>
        <span>{role.builtin ? '内置角色不能直接编辑，可用频道权限覆盖。' : !canManageRoles ? '当前角色只能查看角色定义，不能编辑。' : '调整名称、颜色、排序和项目权限。'}</span>
      </div>
      <div className={styles.formGrid}>
        <label className={styles.field}>
          <span>名称</span>
          <input value={roleName} onChange={(event) => onNameChange(event.target.value)} disabled={readOnly} />
        </label>
        <label className={styles.field}>
          <span>颜色</span>
          <input value={roleColor} onChange={(event) => onColorChange(event.target.value)} disabled={readOnly} />
        </label>
        <label className={styles.field}>
          <span>排序</span>
          <input value={rolePosition} onChange={(event) => onPositionChange(event.target.value)} disabled={readOnly} inputMode="numeric" />
        </label>
      </div>
      <div className={styles.rolePermissionGrid}>
        {permissions.map((permission) => (
          <label key={permission.key} className={styles.rolePermissionItem}>
            <input
              type="checkbox"
              checked={rolePermissions.includes(permission.key)}
              onChange={() => onTogglePermission(permission.key)}
              disabled={readOnly}
            />
            <span>{permission.label}</span>
          </label>
        ))}
      </div>
      <div className={styles.actionRow}>
        <button className={styles.dangerBtn} onClick={onDelete} disabled={readOnly}>删除角色</button>
        <button className={styles.primaryBtn} onClick={onSave} disabled={readOnly || !clean(roleName)}>保存角色</button>
      </div>
    </div>
  )
}

function memberRoleIds(member?: ProjectMember) {
  if (!member) return []
  const ids = member.roles?.length ? member.roles.map((role) => role.id) : [member.role || 'member']
  return Array.from(new Set(ids.map((id) => clean(id)).filter(Boolean)))
}
