import { useMemo, useRef, useState, type ChangeEvent, type MouseEvent } from 'react'
import { createPortal } from 'react-dom'
import {
  ChevronRight,
  Clipboard,
  ExternalLink,
  FolderCog,
  ImagePlus,
  MoreHorizontal,
  Settings,
  UserPlus,
} from 'lucide-react'
import type { User } from '../../store/auth'
import { popoverAnchorFromRect, type PopoverAnchor } from '../../lib/popoverPosition'
import WorkspacePanelResizeHandle from './WorkspacePanelResizeHandle'
import type { WorkspacePanels } from './useWorkspacePanels'
import type { Channel, Project, ProjectMember } from './types'
import {
  MemberContextMenu,
  MemberProfilePopover,
} from './MemberPanel'
import type { MemberMenuRequest, MemberModerationAction } from './MemberPanel'
import {
  memberPresenceStatus,
  memberPrimaryRoleKey,
  memberRoleSummary,
  presenceLabel,
  roleLabel,
} from './memberUtils'
import { useProjectContextSidebar } from './useProjectContextSidebar'
import styles from './ProjectContextSidebar.module.css'

export type ProjectMemberScope = 'channel' | 'project'

interface Props {
  workspacePanels: WorkspacePanels
  project: Project | null
  activeProjectRole: string
  activeChannelId?: string | null
  activeChannel?: Channel
  channels: Channel[]
  panelMembers: ProjectMember[]
  spaceMembers: ProjectMember[]
  spaceLoading: boolean
  spaceError: string
  panelUsesChannelScope: boolean
  memberMenu: MemberMenuRequest | null
  selectedMember: ProjectMember | null
  memberPopoverAnchor: PopoverAnchor
  user: User | null
  canInviteMembers: boolean
  canUseRoleManager: boolean
  canManagePermissions: boolean
  canModerateMembers: boolean
  canManageMembers: boolean
  localNodeId: string
  localNodeName: string
  localNodeReady: boolean
  onProjectChanged: () => Promise<void> | void
  onOpenProjectSettings: () => void
  onOpenWorkspaceSettings: () => void
  onOpenMembersPage: () => void
  onShowInvites: () => void
  onCloseMemberMenu: () => void
  onOpenMemberProfile: (member: ProjectMember, anchor: PopoverAnchor) => void
  onOpenMemberDetails: (member: ProjectMember) => void
  onOpenMemberConversations: (member: ProjectMember) => void
  onOpenMemberPermissions: (member: ProjectMember) => void
  onOpenMemberRoles: (member: ProjectMember) => void
  onModerateMember: (member: ProjectMember, action: MemberModerationAction, durationMinutes?: number) => Promise<void>
  onRemoveMember: (member: ProjectMember) => Promise<boolean | void>
  onCloseSelectedMember: () => void
  onSetMemberPanelScope: (scope: ProjectMemberScope) => void
  onSelectMember: (member: ProjectMember, anchor: PopoverAnchor) => void
  onOpenMemberMenu: (request: MemberMenuRequest) => void
}

export default function ProjectContextSidebar({
  workspacePanels,
  project,
  activeProjectRole,
  activeChannelId,
  activeChannel,
  channels,
  panelMembers,
  spaceMembers,
  spaceLoading,
  spaceError,
  panelUsesChannelScope,
  memberMenu,
  selectedMember,
  memberPopoverAnchor,
  user,
  canInviteMembers,
  canUseRoleManager,
  canManagePermissions,
  canModerateMembers,
  canManageMembers,
  localNodeId,
  localNodeName,
  localNodeReady,
  onProjectChanged,
  onOpenProjectSettings,
  onOpenWorkspaceSettings,
  onOpenMembersPage,
  onShowInvites,
  onCloseMemberMenu,
  onOpenMemberProfile,
  onOpenMemberDetails,
  onOpenMemberConversations,
  onOpenMemberPermissions,
  onOpenMemberRoles,
  onModerateMember,
  onRemoveMember,
  onCloseSelectedMember,
  onSetMemberPanelScope,
  onSelectMember,
  onOpenMemberMenu,
}: Props) {
  const logoInputRef = useRef<HTMLInputElement>(null)
  const [memberQuery, setMemberQuery] = useState('')
  const {
    tab,
    setTab,
    health,
    healthLoading,
    healthError,
    feedback,
    logoBusy,
    copyText,
    updateLogo,
  } = useProjectContextSidebar({ project, onProjectChanged })

  const projectId = project?.id ?? ''
  const displayName = project?.display_name?.trim() || project?.name || '当前项目'
  const iconSource = project?.icon_data_url || project?.icon || ''
  const workspacePath = health?.project?.workspace_path?.trim()
    || project?.workspace_path?.trim()
    || project?.storage_worktree_path?.trim()
    || ''
  const boundNodeId = health?.project?.node_id?.trim() || project?.node_id?.trim() || ''
  const nodeIsLocal = !!boundNodeId && boundNodeId === localNodeId
  const nodeName = health?.node?.device_name?.trim()
    || (nodeIsLocal ? localNodeName : '')
    || shortId(boundNodeId)
    || '尚未绑定'
  const nodeOnline = health?.node?.online ?? health?.node?.cli_connected ?? (nodeIsLocal ? localNodeReady : undefined)
  const nodeState = !boundNodeId ? '未绑定' : nodeOnline === true ? '在线' : nodeOnline === false ? '离线' : '已绑定'
  const owner = spaceMembers.find((member) => memberPrimaryRoleKey(member) === 'owner')
  const ownerName = owner?.member_display_name || owner?.account || owner?.global_account || '未标注'
  const defaultChannel = channels.find((channel) => channel.kind === 'ai_development')
  const canUpdateLogo = activeProjectRole === 'owner'
  const visibleMembers = useMemo(() => {
    const query = memberQuery.trim().toLocaleLowerCase('zh-CN')
    return [...panelMembers]
      .filter((member) => !query || memberSearchText(member).includes(query))
      .sort(compareMembers)
  }, [memberQuery, panelMembers])
  const onlineCount = panelMembers.filter((member) => memberPresenceStatus(member) !== 'offline').length

  if (workspacePanels.memberCollapsed) return null

  function selectLogo() {
    if (canUpdateLogo && !logoBusy) logoInputRef.current?.click()
  }

  function handleLogoChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) void updateLogo(file)
  }

  return (
    <aside className={styles.panel} aria-label="项目详情侧栏">
      <WorkspacePanelResizeHandle side="member" panels={workspacePanels} />
      <header className={styles.header}>
        <div>
          <strong>项目详情</strong>
          <span>{displayName}</span>
        </div>
        <button
          className={styles.iconButton}
          type="button"
          title="收起项目详情"
          aria-label="收起项目详情"
          onClick={workspacePanels.toggleMemberPanel}
        >
          <ChevronRight size={17} aria-hidden="true" />
        </button>
      </header>

      <div className={styles.tabs} role="tablist" aria-label="项目详情">
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'project'}
          data-active={tab === 'project' ? 'true' : undefined}
          onClick={() => setTab('project')}
        >
          项目
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'members'}
          data-active={tab === 'members' ? 'true' : undefined}
          onClick={() => setTab('members')}
        >
          成员 <span>{spaceMembers.length}</span>
        </button>
      </div>

      <div className={styles.content}>
        {memberMenu && createPortal(
          <MemberContextMenu
            member={memberMenu.member}
            x={memberMenu.x}
            y={memberMenu.y}
            canModerate={canModerateMembers && memberMenu.member.user_id !== user?.id}
            canRemove={canManageMembers && memberMenu.member.user_id !== user?.id}
            onClose={onCloseMemberMenu}
            onOpenProfile={onOpenMemberProfile}
            onOpenDetails={onOpenMemberDetails}
            onOpenConversations={onOpenMemberConversations}
            onOpenPermissions={activeChannelId && canManagePermissions ? onOpenMemberPermissions : undefined}
            onOpenRoles={canUseRoleManager ? onOpenMemberRoles : undefined}
            onModerate={onModerateMember}
            onRemove={onRemoveMember}
          />,
          document.body,
        )}
        {selectedMember && createPortal(
          <MemberProfilePopover
            member={selectedMember}
            anchor={memberPopoverAnchor}
            projectId={projectId || undefined}
            channels={channels}
            channel={activeChannel}
            canModerate={canModerateMembers && selectedMember.user_id !== user?.id}
            canRemove={canManageMembers && selectedMember.user_id !== user?.id}
            onClose={onCloseSelectedMember}
            onOpenDetails={onOpenMemberDetails}
            onOpenConversations={onOpenMemberConversations}
            onOpenRoles={canUseRoleManager ? onOpenMemberRoles : undefined}
            onModerate={onModerateMember}
            onRemove={onRemoveMember}
          />,
          document.body,
        )}

        {tab === 'project' ? (
          <>
            <section className={styles.identity}>
              <button
                className={styles.logoButton}
                type="button"
                onClick={selectLogo}
                disabled={!canUpdateLogo || logoBusy}
                title={canUpdateLogo ? '更换项目 Logo' : '只有项目创建者可以修改 Logo'}
              >
                {iconSource
                  ? <img src={iconSource} alt="" />
                  : <span>{projectInitial(displayName)}</span>}
                {canUpdateLogo && <em><ImagePlus size={15} aria-hidden="true" />{logoBusy ? '处理中' : '更换'}</em>}
              </button>
              <input
                ref={logoInputRef}
                className={styles.hiddenInput}
                type="file"
                accept="image/png,image/jpeg,image/webp"
                onChange={handleLogoChange}
              />
              <div>
                <strong>{displayName}</strong>
                <span>{roleLabel(activeProjectRole)} · {spaceMembers.length} 位成员</span>
                {project?.description && <p>{project.description}</p>}
              </div>
            </section>

            <section className={styles.section}>
              <div className={styles.sectionTitle}>
                <strong>开发位置</strong>
                <button type="button" onClick={onOpenWorkspaceSettings}>工作区设置</button>
              </div>
              <div className={styles.detailRow}>
                <span className={styles.statusDot} data-state={nodeState} />
                <div>
                  <span>项目节点</span>
                  <strong title={boundNodeId}>{healthLoading ? '正在确认节点…' : nodeName}</strong>
                  <small>{nodeState}{boundNodeId ? ` · ${shortId(boundNodeId)}` : ''}</small>
                </div>
                {boundNodeId && (
                  <button
                    className={styles.rowAction}
                    type="button"
                    title="复制节点 ID"
                    aria-label="复制节点 ID"
                    onClick={() => void copyText(boundNodeId, '节点 ID 已复制')}
                  >
                    <Clipboard size={15} aria-hidden="true" />
                  </button>
                )}
              </div>
              <div className={styles.detailRow}>
                <FolderCog className={styles.rowIcon} size={18} aria-hidden="true" />
                <div>
                  <span>工作目录</span>
                  <strong title={workspacePath}>{workspacePath || '尚未设置工作目录'}</strong>
                  <small>{workspacePath ? '当前项目代码位置' : '请在工作区设置中绑定目录'}</small>
                </div>
                {workspacePath && (
                  <button
                    className={styles.rowAction}
                    type="button"
                    title="复制完整目录"
                    aria-label="复制完整目录"
                    onClick={() => void copyText(workspacePath, '工作目录已复制')}
                  >
                    <Clipboard size={15} aria-hidden="true" />
                  </button>
                )}
              </div>
              {healthError && <p className={styles.inlineNotice}>{healthError}</p>}
              {health?.recommended_action && <p className={styles.inlineNotice}>{health.recommended_action}</p>}
            </section>

            <section className={styles.section}>
              <div className={styles.sectionTitle}><strong>项目信息</strong></div>
              <InfoRow label="所有者" value={ownerName} />
              <InfoRow label="默认频道" value={defaultChannel?.name || 'AI开发'} />
              <InfoRow label="项目 ID" value={projectId || '-'} copy={() => void copyText(projectId, '项目 ID 已复制')} />
              {project?.repo_url && <InfoRow label="代码仓库" value={project.repo_url} />}
              {project?.branch && <InfoRow label="默认分支" value={project.branch} />}
            </section>

            {feedback && <p className={styles.feedback} role="status">{feedback}</p>}
            <button className={styles.fullSettingsButton} type="button" onClick={onOpenProjectSettings}>
              <Settings size={16} aria-hidden="true" />
              完整项目设置
              <ExternalLink size={14} aria-hidden="true" />
            </button>
          </>
        ) : (
          <section className={styles.membersPane}>
            <div className={styles.memberToolbar}>
              <div>
                <strong>{spaceMembers.length} 位成员</strong>
                <span>{onlineCount} 人在线</span>
              </div>
              {canInviteMembers && (
                <button type="button" onClick={onShowInvites}>
                  <UserPlus size={15} aria-hidden="true" />邀请成员
                </button>
              )}
            </div>
            {activeChannelId && (
              <div className={styles.scopeSwitch} role="group" aria-label="成员范围">
                <button type="button" data-active={!panelUsesChannelScope ? 'true' : undefined} onClick={() => onSetMemberPanelScope('project')}>项目成员</button>
                <button type="button" data-active={panelUsesChannelScope ? 'true' : undefined} onClick={() => onSetMemberPanelScope('channel')}>当前频道</button>
              </div>
            )}
            {panelMembers.length > 6 && (
              <input
                className={styles.memberSearch}
                value={memberQuery}
                onChange={(event) => setMemberQuery(event.target.value)}
                placeholder="搜索成员"
                aria-label="搜索成员"
              />
            )}
            {spaceLoading && panelMembers.length === 0 && <p className={styles.empty}>正在读取项目成员…</p>}
            {!spaceLoading && spaceError && <p className={styles.empty}>{spaceError}</p>}
            {!spaceLoading && !spaceError && visibleMembers.length === 0 && <p className={styles.empty}>没有匹配的成员</p>}
            <div className={styles.memberList}>
              {visibleMembers.map((member) => (
                <MemberRow
                  key={member.user_id}
                  member={member}
                  onSelect={onSelectMember}
                  onOpenConversations={onOpenMemberConversations}
                  onOpenMenu={onOpenMemberMenu}
                />
              ))}
            </div>
            <button className={styles.fullSettingsButton} type="button" onClick={onOpenMembersPage}>
              <Settings size={16} aria-hidden="true" />成员与权限管理<ExternalLink size={14} aria-hidden="true" />
            </button>
          </section>
        )}
      </div>
    </aside>
  )
}

function InfoRow({ label, value, copy }: { label: string; value: string; copy?: () => void }) {
  return (
    <div className={styles.infoRow}>
      <span>{label}</span>
      <strong title={value}>{value}</strong>
      {copy && <button type="button" title={`复制${label}`} aria-label={`复制${label}`} onClick={copy}><Clipboard size={14} aria-hidden="true" /></button>}
    </div>
  )
}

function MemberRow({
  member,
  onSelect,
  onOpenConversations,
  onOpenMenu,
}: {
  member: ProjectMember
  onSelect: (member: ProjectMember, anchor: PopoverAnchor) => void
  onOpenConversations: (member: ProjectMember) => void
  onOpenMenu: (request: MemberMenuRequest) => void
}) {
  const status = memberPresenceStatus(member)
  const name = member.member_display_name || member.account || member.global_account || member.user_id
  const role = memberRoleSummary(member)

  function openProfile(event: MouseEvent<HTMLElement>) {
    onSelect(member, popoverAnchorFromRect(event.currentTarget.getBoundingClientRect()))
  }

  function openMenu(event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation()
    onOpenMenu({ member, x: event.clientX, y: event.clientY })
  }

  return (
    <div className={styles.memberRow}>
      <button className={styles.avatarButton} type="button" onClick={() => onOpenConversations(member)} title={`打开 ${name} 的会话`}>
        <span className={styles.memberAvatar} data-status={status}>
          {member.avatar_data_url ? <img src={member.avatar_data_url} alt="" /> : projectInitial(name)}
        </span>
      </button>
      <button className={styles.memberCopy} type="button" onClick={openProfile}>
        <strong>{name}</strong>
        <span>{role} · {presenceLabel(status)}</span>
      </button>
      <button className={styles.moreButton} type="button" onClick={openMenu} title={`管理 ${name}`} aria-label={`管理 ${name}`}>
        <MoreHorizontal size={16} aria-hidden="true" />
      </button>
    </div>
  )
}

function compareMembers(left: ProjectMember, right: ProjectMember): number {
  const leftOwner = memberPrimaryRoleKey(left) === 'owner' ? 0 : 1
  const rightOwner = memberPrimaryRoleKey(right) === 'owner' ? 0 : 1
  if (leftOwner !== rightOwner) return leftOwner - rightOwner
  const leftOnline = memberPresenceStatus(left) === 'offline' ? 1 : 0
  const rightOnline = memberPresenceStatus(right) === 'offline' ? 1 : 0
  if (leftOnline !== rightOnline) return leftOnline - rightOnline
  return memberSearchText(left).localeCompare(memberSearchText(right), 'zh-CN')
}

function memberSearchText(member: ProjectMember): string {
  return [member.member_display_name, member.account, member.global_account, member.user_id, memberRoleSummary(member)]
    .filter(Boolean)
    .join(' ')
    .toLocaleLowerCase('zh-CN')
}

function projectInitial(value: string): string {
  return value.trim().slice(0, 1).toUpperCase() || '项'
}

function shortId(value: string): string {
  if (!value) return ''
  return value.length > 18 ? `${value.slice(0, 10)}…${value.slice(-6)}` : value
}
