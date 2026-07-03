import { useEffect, useMemo, useState } from 'react'
import { api } from '../../api/client'
import { clean, formatTime } from '../../lib/utils'
import type { ProjectMemberAuditEntry, ProjectMemberAuditResponse } from './types'
import { roleLabel } from './memberUtils'
import styles from './ConversationPage.module.css'

const ACTION_LABELS: Record<string, string> = {
  create_invite_link: '创建邀请链接',
  revoke_invite_link: '撤销邀请链接',
  join_by_invite_link: '通过邀请加入',
  add_member: '添加成员',
  invite_member: '邀请成员',
  update_member_role: '调整角色',
  update_member_profile: '更新成员资料',
  remove_member: '移除成员',
  create_role: '创建角色',
  update_role: '更新角色',
  delete_role: '删除角色',
  mute_member: '禁言成员',
  unmute_member: '解除禁言',
  ban_member: '封禁成员',
  unban_member: '解封成员',
  update_channel_permission: '更新频道权限',
  update_channel_member_permission: '更新成员频道权限',
  update_channel_role_permission: '更新角色频道权限',
  update_category_permission: '更新分类权限',
  update_category_member_permission: '更新成员分类权限',
  update_category_role_permission: '更新角色分类权限',
}

export function MemberAuditDrawer({
  projectId,
  onClose,
}: {
  projectId: string
  onClose: () => void
}) {
  const [entries, setEntries] = useState<ProjectMemberAuditEntry[]>([])
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')

  async function refreshAudit() {
    setLoading(true)
    try {
      const data = await api.get<ProjectMemberAuditResponse>(`/api/projects/${encodeURIComponent(projectId)}/member-audit?limit=80`)
      setEntries(data.entries ?? [])
      setMessage('')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '成员日志读取失败')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    refreshAudit()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId])

  const visibleEntries = useMemo(() => {
    const needle = clean(query).toLowerCase()
    if (!needle) return entries
    return entries.filter((entry) => {
      const haystack = [
        auditActionLabel(entry.action),
        entry.action,
        entry.actor_account,
        entry.actor_user_id,
        entry.target_account,
        entry.target_user_id,
        entry.old_role,
        entry.new_role,
        entry.note,
        entry.created_at,
      ].join(' ').toLowerCase()
      return haystack.includes(needle)
    })
  }, [entries, query])

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.auditDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>成员日志</strong>
            <span>{loading ? '同步中...' : message || `最近 ${entries.length} 条成员管理记录`}</span>
          </div>
          <div className={styles.drawerHeaderActions}>
            <button className={styles.drawerCloseBtn} onClick={refreshAudit} disabled={loading}>刷新</button>
            <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
          </div>
        </header>
        <div className={styles.drawerBody}>
          <input
            className={styles.drawerSearchInput}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="搜索操作者、成员、动作或备注"
          />
          <div className={styles.auditList}>
            {visibleEntries.length === 0 && (
              <p className={styles.sideHint}>{loading ? '正在读取成员日志...' : '没有匹配的日志'}</p>
            )}
            {visibleEntries.map((entry) => (
              <AuditRow key={entry.id} entry={entry} />
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}

function AuditRow({ entry }: { entry: ProjectMemberAuditEntry }) {
  const actor = clean(entry.actor_account ?? entry.actor_user_id) || '系统'
  const target = clean(entry.target_account ?? entry.target_user_id) || '项目'
  const roleChange = roleChangeLabel(entry)
  const noteParts = auditNoteParts(entry.note)
  return (
    <article className={styles.auditRow}>
      <div className={styles.auditRowMain}>
        <span className={styles.auditAction}>{auditActionLabel(entry.action)}</span>
        <strong title={target}>{target}</strong>
        <em>{formatTime(entry.created_at)}</em>
      </div>
      <div className={styles.auditMeta}>
        <span>操作者：{actor}</span>
        {roleChange && <span>{roleChange}</span>}
      </div>
      {noteParts.length > 0 && (
        <div className={styles.auditNote}>
          {noteParts.map((part) => (
            <span key={part}>{part}</span>
          ))}
        </div>
      )}
    </article>
  )
}

function auditActionLabel(action: string) {
  return ACTION_LABELS[clean(action)] ?? (clean(action) || '成员操作')
}

function roleChangeLabel(entry: ProjectMemberAuditEntry) {
  const oldRole = clean(entry.old_role)
  const newRole = clean(entry.new_role)
  if (!oldRole && !newRole) return ''
  if (oldRole && newRole) return `角色：${roleLabel(oldRole)} -> ${roleLabel(newRole)}`
  if (newRole) return `角色：${roleLabel(newRole)}`
  return `原角色：${roleLabel(oldRole)}`
}

function auditNoteParts(note?: string | null) {
  const value = clean(note)
  if (!value) return []
  const parts = value
    .split(';')
    .map((part) => part.trim())
    .filter(Boolean)
    .map(formatNotePart)
  return parts.length ? parts : [value]
}

function formatNotePart(part: string) {
  const index = part.indexOf('=')
  if (index < 0) return part
  const key = part.slice(0, index)
  const value = part.slice(index + 1)
  const labels: Record<string, string> = {
    channel_id: '频道',
    channel_kind: '频道类型',
    category_id: '分类',
    category: '分类',
    scope: '范围',
    target: '目标',
    allow: '允许',
    deny: '拒绝',
    invite_code: '邀请码',
    temporary: '临时邀请',
    max_uses: '最大次数',
    expires_at: '过期时间',
  }
  return `${labels[key] ?? key}: ${value || '-'}`
}
