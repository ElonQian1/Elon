import { useEffect, useMemo, useRef, useState } from 'react'
import { useProjectStore } from './useProjectStore'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { DevTaskMessage } from '../dev/DevTaskCard'
import { buildContext } from '../dev/devTaskUtils'
import { CreateProjectModal } from '../projects/CreateProjectModal'
import { api } from '../../api/client'
import { formatTime, clean } from '../../lib/utils'
import type {
  Channel,
  ChannelCategory,
  ChannelPermissionResponse,
  Message,
  PermissionOption,
  PermissionOverride,
  ProjectMember,
  ProjectRole,
  ProjectRolesResponse,
} from './types'
import styles from './ConversationPage.module.css'

export default function ConversationPage() {
  const user = useAuthStore((s) => s.user)
  const {
    projects, projectsLoaded, activeProjectId, channels, categories, members, activeChannelId,
    messages, messagesLoading, sendingMessage,
    loadProjects, selectProject, reloadProjectSpace, selectChannel, sendMessage, cancelTask, approveTool,
  } = useProjectStore()
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const [input, setInput] = useState('')
  const [sendError, setSendError] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const [showPermissions, setShowPermissions] = useState(false)
  const feedRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    loadProjects()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user?.id])

  useEffect(() => {
    // 自动滚到底
    if (feedRef.current) feedRef.current.scrollTop = feedRef.current.scrollHeight
  }, [messages])

  async function handleSend(e: React.FormEvent) {
    e.preventDefault()
    const text = input.trim()
    if (!text || sendingMessage) return
    setInput('')
    setSendError('')
    try {
      await sendMessage(text, selectedAgent || null)
    } catch (err) {
      setSendError((err as { message?: string }).message ?? '发送失败')
    }
  }

  const activeProject = projects.find((p) => p.id === activeProjectId)
  const activeChannel = channels.find((c) => c.id === activeChannelId)
  const isDevChannel = activeChannel?.kind === 'ai_development'
  const groupedChannels = useMemo(() => groupChannels(channels, categories), [channels, categories])
  const canManagePermissions = channels.some(channelCanManage)

  // 构建 dev task 上下文（一次性，messages 变化时重算）
  const taskContext = buildContext(messages as Parameters<typeof buildContext>[0])

  return (
    <div className={styles.layout}>
      {/* 项目列表 */}
      <aside className={styles.projectSidebar}>
        <div className={styles.sideHeader}>
          <span>我的项目</span>
          <button className={styles.newProjectBtn} onClick={() => setShowCreate(true)} title="新建项目">+</button>
        </div>
        {!projectsLoaded && <p className={styles.sideHint}>读取中…</p>}
        {projectsLoaded && projects.length === 0 && (
          <p className={styles.sideHint}>暂无项目，点击 + 新建</p>
        )}
        {projects.map((p) => (
          <button
            key={p.id}
            className={[styles.projectBtn, p.id === activeProjectId ? styles.projectActive : ''].join(' ')}
            onClick={() => selectProject(p.id)}
          >
            <span className={styles.projectIcon}>{p.name?.[0]?.toUpperCase() ?? '?'}</span>
            <span className={styles.projectMeta}>
              <strong>{p.name}</strong>
              {p.description && <small>{p.description}</small>}
            </span>
          </button>
        ))}
      </aside>

      {/* 频道列表 */}
      {activeProjectId && (
        <aside className={styles.channelSidebar}>
          <div className={styles.sideHeader}>
            <span>{activeProject?.name ?? '频道'}</span>
          </div>
          {groupedChannels.map((group) => (
            <div key={group.id} className={styles.channelGroup}>
              <div className={styles.channelGroupTitle}>{group.name}</div>
              {group.channels.map((c) => (
                <button
                  key={c.id}
                  className={[styles.channelBtn, c.id === activeChannelId ? styles.channelActive : ''].join(' ')}
                  onClick={() => selectChannel(c.id)}
                >
                  <span className={styles.channelIcon}>
                    {channelIcon(c)}
                  </span>
                  <span className={styles.channelName}>{c.name}</span>
                </button>
              ))}
            </div>
          ))}
        </aside>
      )}

      {/* 消息区域 */}
      <div className={styles.main}>
        {!activeProjectId && (
          <div className={styles.placeholder}>
            <h2>选择一个项目</h2>
            <p>从左侧选择项目，或点击 + 新建</p>
            <button className={styles.bigCreateBtn} onClick={() => setShowCreate(true)}>+ 新建项目</button>
          </div>
        )}

        {activeProjectId && !activeChannelId && (
          <div className={styles.placeholder}>
            <h2>{activeProject?.name}</h2>
            <p>从频道列表选择一个频道开始对话</p>
          </div>
        )}

        {activeChannelId && (
          <>
            <header className={styles.channelHeader}>
              <div>
                <span className={styles.channelHeaderName}>{activeChannel?.name ?? activeChannelId}</span>
                {activeChannel?.description && (
                  <span className={styles.channelHeaderDesc}>{activeChannel.description}</span>
                )}
              </div>
              <span className={styles.channelKindBadge}>
                {activeChannel?.kind === 'ai_development' ? 'AI 开发' : activeChannel?.kind ?? '频道'}
              </span>
            </header>

            <div className={styles.feed} ref={feedRef}>
              {messagesLoading && messages.length === 0 && (
                <p className={styles.feedHint}>正在读取消息…</p>
              )}
              {!messagesLoading && messages.length === 0 && (
                <p className={styles.feedHint}>还没有消息，发送第一条！</p>
              )}
              {messages.map((msg) => (
                <MessageItem
                  key={msg.id}
                  message={msg}
                  isDevChannel={isDevChannel}
                  taskContext={taskContext}
                  user={user}
                  onCancel={cancelTask}
                  onApprove={approveTool}
                />
              ))}
            </div>

            <form className={styles.composer} onSubmit={handleSend}>
              <input
                className={styles.composerInput}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                placeholder={isDevChannel ? '描述开发需求，AI 会帮你实现…' : '发送消息…'}
                disabled={sendingMessage}
              />
              <button
                className={styles.sendBtn}
                type="submit"
                disabled={!input.trim() || sendingMessage}
              >
                {sendingMessage ? '…' : '发送'}
              </button>
            </form>
            {sendError && <p className={styles.sendError}>{sendError}</p>}
          </>
        )}
      </div>

      {activeProjectId && (
        <aside className={styles.memberSidebar}>
          <div className={styles.sideHeader}>
            <span>成员</span>
            {canManagePermissions && (
              <button className={styles.memberToolBtn} onClick={() => setShowPermissions(true)}>权限</button>
            )}
          </div>
          <div className={styles.memberCount}>{onlineCount(members)} 在线 / {members.length} 成员</div>
          <div className={styles.memberList}>
            {members.length === 0 && <p className={styles.sideHint}>暂无成员</p>}
            {members.map((member) => (
              <div key={member.user_id} className={styles.memberRow}>
                <span className={styles.memberAvatar}>{memberInitial(member)}</span>
                <span className={styles.memberInfo}>
                  <strong>{member.account || member.user_id}</strong>
                  <small>{memberRoleSummary(member)}</small>
                </span>
                <span className={[styles.memberDot, member.is_online ? styles.memberOnline : ''].join(' ')} />
              </div>
            ))}
          </div>
        </aside>
      )}

      {showCreate && (
        <CreateProjectModal
          quickMode
          onClose={() => setShowCreate(false)}
          onCreated={async (p) => {
            setShowCreate(false)
            await loadProjects()
            if (p.id) await selectProject(p.id)
          }}
        />
      )}

      {showPermissions && activeProjectId && (
        <PermissionDrawer
          projectId={activeProjectId}
          activeChannelId={activeChannelId}
          channels={channels}
          categories={categories}
          members={members}
          onClose={() => setShowPermissions(false)}
          onSaved={reloadProjectSpace}
        />
      )}
    </div>
  )
}

interface ChannelGroup {
  id: string
  name: string
  channels: Channel[]
}

const CHANNEL_PERMISSION_OPTIONS: PermissionOption[] = [
  { key: 'view_channel', label: '查看频道' },
  { key: 'send_messages', label: '发送消息' },
  { key: 'start_ai_tasks', label: '发起 AI 任务' },
  { key: 'manage_channel', label: '管理频道权限' },
]

function groupChannels(channels: Channel[], categories: ChannelCategory[]): ChannelGroup[] {
  if (!categories.length) {
    return [{ id: 'all', name: '频道', channels }]
  }
  const groups: ChannelGroup[] = []
  const used = new Set<string>()
  categories
    .slice()
    .sort((a, b) => Number(a.position ?? 0) - Number(b.position ?? 0))
    .forEach((category) => {
      const items = channels.filter((channel) => clean(channel.category_id ?? '') === category.id)
      if (!items.length) return
      items.forEach((item) => used.add(item.id))
      groups.push({ id: category.id, name: category.name || category.kind || '分类', channels: items })
    })
  const rest = channels.filter((channel) => !used.has(channel.id))
  if (rest.length) groups.push({ id: 'other', name: '其他', channels: rest })
  return groups
}

function channelIcon(channel: Channel) {
  if (channel.kind === 'ai_development') return 'AI'
  if (channel.kind === 'builds') return '包'
  if (channel.kind === 'announcements') return '告'
  if (channel.kind === 'docs') return '文'
  return '#'
}

function channelCanManage(channel: Channel) {
  const permissions = channel.permissions ?? {}
  return !!(permissions.can_manage || permissions.canManage)
}

function onlineCount(members: ProjectMember[]) {
  return members.filter((member) => member.is_online).length
}

function memberInitial(member: ProjectMember) {
  return clean(member.account ?? member.user_id).slice(0, 1).toUpperCase() || '员'
}

function memberRoleSummary(member: ProjectMember) {
  const roles = member.roles ?? []
  if (roles.length) return roles.map((role) => role.name || role.id).join(' / ')
  return roleLabel(member.role ?? 'member')
}

function roleLabel(role: string) {
  const labels: Record<string, string> = {
    owner: '拥有者',
    admin: '管理员',
    editor: '协作者',
    developer: '开发者',
    maintainer: '维护者',
    member: '成员',
    observer: '只读成员',
  }
  return labels[role] ?? role
}

function PermissionDrawer({
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

type PermissionEffect = '' | 'allow' | 'deny'

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

function permissionEffect(override: PermissionOverride, permission: string): PermissionEffect {
  if ((override.deny ?? []).includes(permission)) return 'deny'
  if ((override.allow ?? []).includes(permission)) return 'allow'
  return ''
}

function findOverride(overrides: PermissionOverride[], targetId: string, kind: 'role' | 'member') {
  const key = kind === 'role' ? 'role_id' : 'user_id'
  const altKey = kind === 'role' ? 'roleId' : 'userId'
  return overrides.find((override) => clean(String(override[key] ?? override[altKey] ?? '')) === targetId) ?? {}
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

/* ── 单条消息渲染 ── */
function MessageItem({ message, isDevChannel, taskContext, user, onCancel, onApprove }: {
  message: Message
  isDevChannel: boolean
  taskContext: ReturnType<typeof buildContext>
  user: { nickname?: string; account?: string } | null
  onCancel: (taskId: string) => Promise<void>
  onApprove: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
}) {
  const kind = clean(message.kind ?? message.role ?? '').toLowerCase()

  // Dev task 消息用 DevTaskCard 渲染
  if (isDevChannel && ['ai_task', 'ai_progress', 'ai_result'].includes(kind)) {
    return (
      <div className={styles.msgRow}>
        <DevTaskMessage
          message={message}
          context={taskContext}
          onCancel={onCancel}
          onApprove={onApprove}
        />
      </div>
    )
  }

  // 普通聊天消息
  const isUser = kind === 'user' || kind === 'human'
  const content = clean(message.content ?? message.text ?? '')
  const time = message.created_at ? formatTime(message.created_at) : ''
  const displayName = isUser ? (user?.nickname ?? user?.account ?? '我') : 'AI'

  return (
    <div className={[styles.msgRow, isUser ? styles.userMsg : styles.aiMsg].join(' ')}>
      <div className={styles.msgAvatar}>
        {isUser ? ((user?.nickname ?? user?.account)?.[0]?.toUpperCase() ?? '我') : 'AI'}
      </div>
      <div className={styles.msgBody}>
        <div className={styles.msgMeta}>
          <strong>{displayName}</strong>
          {time && <span>{time}</span>}
        </div>
        <div className={styles.msgContent}>{content}</div>
      </div>
    </div>
  )
}
