import { useMemo, type MouseEvent } from 'react'
import { createPortal } from 'react-dom'
import { ChevronRight } from 'lucide-react'
import type { User } from '../../store/auth'
import AgentRunsPanel from '../dev/AgentRunsPanel'
import type { Channel, ProjectMember } from './types'
import {
  MemberContextMenu,
  MemberContextSummary,
  MemberLoadingRows,
  MemberProfilePopover,
  MemberSearch,
  memberPresenceAvatarClass,
} from './MemberPanel'
import type { MemberMenuRequest, MemberModerationAction } from './MemberPanel'
import { popoverAnchorFromRect, type PopoverAnchor } from '../../lib/popoverPosition'
import { memberPresenceStatus, presenceLabel } from './memberUtils'
import WorkspacePanelResizeHandle from './WorkspacePanelResizeHandle'
import type { WorkspacePanels } from './useWorkspacePanels'
import styles from './ConversationPage.module.css'

export type MemberPanelScope = 'channel' | 'project'

interface ConversationMemberSidebarProps {
  workspacePanels: WorkspacePanels
  title: string
  count: number
  context: string
  activeProjectId?: string | null
  activeChannelId?: string | null
  activeChannel?: Channel
  channels: Channel[]
  canInviteMembers: boolean
  canUseRoleManager: boolean
  canViewMemberAudit: boolean
  canManagePermissions: boolean
  canModerateMembers: boolean
  canManageMembers: boolean
  panelUsesChannelScope: boolean
  panelUsesChannelPermissions: boolean
  memberPanelSummary: string
  selectionMode: boolean
  onSelectionModeChange: (value: boolean) => void
  panelMembers: ProjectMember[]
  spaceMembers: ProjectMember[]
  spaceLoading: boolean
  spaceError: string
  memberMenu: MemberMenuRequest | null
  selectedMember: ProjectMember | null
  memberPopoverAnchor: PopoverAnchor
  isDevChannel: boolean
  activeWorkspacePath: string
  isAssistingMember: boolean
  activeConversationTargetId: string | null
  user: User | null
  ownPresenceAvatarStatus: string
  ownAvatarUrl: string
  ownInitial: string
  ownDisplayName: string
  ownPresenceSubtitle: string
  onShowPresence: () => void
  onShowDirectory: () => void
  onOpenMembersPage: () => void
  onShowInvites: () => void
  onOpenModeration: () => void
  onOpenRoleManager: () => void
  onShowAudit: () => void
  onOpenPermissionManager: () => void
  onCloseMemberMenu: () => void
  onOpenMemberProfile: (member: ProjectMember, anchor: PopoverAnchor) => void
  onOpenMemberDetails: (member: ProjectMember) => void
  onOpenMemberConversations: (member: ProjectMember) => void
  onOpenMemberPermissions: (member: ProjectMember) => void
  onOpenMemberRoles: (member: ProjectMember) => void
  onModerateMember: (member: ProjectMember, action: MemberModerationAction, durationMinutes?: number) => Promise<void>
  onRemoveMember: (member: ProjectMember) => Promise<boolean | void>
  onCloseSelectedMember: () => void
  onSetMemberPanelScope: (scope: MemberPanelScope) => void
  onSelectMember: (member: ProjectMember, anchor: PopoverAnchor) => void
  onOpenMemberMenu: (request: MemberMenuRequest) => void
}

export default function ConversationMemberSidebar({
  workspacePanels,
  title,
  count,
  context,
  activeProjectId,
  activeChannelId,
  activeChannel,
  channels,
  canInviteMembers,
  canUseRoleManager,
  canViewMemberAudit,
  canManagePermissions,
  canModerateMembers,
  canManageMembers,
  panelUsesChannelScope,
  panelUsesChannelPermissions,
  memberPanelSummary,
  selectionMode,
  onSelectionModeChange,
  panelMembers,
  spaceMembers,
  spaceLoading,
  spaceError,
  memberMenu,
  selectedMember,
  memberPopoverAnchor,
  isDevChannel,
  activeWorkspacePath,
  isAssistingMember,
  activeConversationTargetId,
  user,
  ownPresenceAvatarStatus,
  ownAvatarUrl,
  ownInitial,
  ownDisplayName,
  ownPresenceSubtitle,
  onShowPresence,
  onShowDirectory,
  onOpenMembersPage,
  onShowInvites,
  onOpenModeration,
  onOpenRoleManager,
  onShowAudit,
  onOpenPermissionManager,
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
}: ConversationMemberSidebarProps) {
  const hasProject = !!activeProjectId
  const onlineCount = useMemo(
    () => panelMembers.filter((member) => memberPresenceStatus(member) !== 'offline').length,
    [panelMembers],
  )

  if (workspacePanels.memberCollapsed) return null

  return (
    <aside
      className={styles.memberPanel}
      aria-label="右侧项目大厅"
    >
      <WorkspacePanelResizeHandle side="member" panels={workspacePanels} />
      <div className={styles.memberTitle}>
        <div className={styles.memberTitleCopy}>
          <strong>{title}{count > 0 ? ` — ${count}` : ''}</strong>
          <span>{context}</span>
        </div>
        <div className={styles.memberActions}>
          <button
            className={styles.memberIconBtn}
            type="button"
            title="隐藏右侧栏"
            aria-label="隐藏右侧栏"
            onClick={workspacePanels.toggleMemberPanel}
          >
            <ChevronRight size={14} aria-hidden="true" />
          </button>
          <button className={styles.memberInviteBtn} type="button" onClick={onShowPresence}>状态</button>
          {hasProject && (
            <button className={styles.memberInviteBtn} type="button" onClick={() => onSelectionModeChange(!selectionMode)}>
              {selectionMode ? '在线' : '选择'}
            </button>
          )}
          {hasProject && <button className={styles.memberInviteBtn} type="button" onClick={onShowDirectory}>目录</button>}
          {hasProject && <button className={styles.memberInviteBtn} type="button" onClick={onOpenMembersPage}>成员页</button>}
          {hasProject && canInviteMembers && <button className={styles.memberInviteBtn} type="button" onClick={onShowInvites}>邀请</button>}
          {hasProject && <button className={styles.memberInviteBtn} type="button" onClick={onOpenModeration}>管理</button>}
          {hasProject && canUseRoleManager && <button className={styles.memberInviteBtn} type="button" onClick={onOpenRoleManager}>角色</button>}
          {hasProject && canViewMemberAudit && <button className={styles.memberInviteBtn} type="button" onClick={onShowAudit}>日志</button>}
          {hasProject && activeChannelId && canManagePermissions && (
            <button className={styles.memberInviteBtn} type="button" onClick={onOpenPermissionManager}>权限</button>
          )}
        </div>
      </div>
      <div className={styles.memberList}>
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
            onOpenPermissions={hasProject && activeChannelId && canManagePermissions ? onOpenMemberPermissions : undefined}
            onOpenRoles={hasProject && canUseRoleManager ? onOpenMemberRoles : undefined}
            onModerate={onModerateMember}
            onRemove={onRemoveMember}
          />,
          document.body
        )}
        {selectedMember && createPortal(
          <MemberProfilePopover
            member={selectedMember}
            anchor={memberPopoverAnchor}
            projectId={activeProjectId ?? undefined}
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
          document.body
        )}
        {hasProject && isDevChannel && activeWorkspacePath && (
          <div className={styles.agentRunsSlot}>
            <AgentRunsPanel workspacePath={activeWorkspacePath} />
          </div>
        )}
        {hasProject && (
          <div className={styles.memberScopeSwitch} role="group" aria-label="成员列表范围">
            {activeChannelId && (
              <button
                type="button"
                data-active={panelUsesChannelScope ? 'true' : undefined}
                onClick={() => onSetMemberPanelScope('channel')}
              >
                当前频道
              </button>
            )}
            <button
              type="button"
              data-active={!panelUsesChannelScope ? 'true' : undefined}
              onClick={() => onSetMemberPanelScope('project')}
            >
              项目大厅
            </button>
          </div>
        )}
        {hasProject && (
          <MemberContextSummary
            title={panelUsesChannelScope ? '当前频道' : '项目大厅'}
            label={selectionMode ? memberPanelSummary : `在线 ${onlineCount} 人 · ${memberPanelSummary}`}
            members={panelMembers}
            channel={panelUsesChannelScope ? activeChannel : undefined}
            projectTotal={spaceMembers.length}
            usingChannelPermissions={panelUsesChannelPermissions}
          />
        )}
        {hasProject && spaceLoading && panelMembers.length === 0 && <MemberLoadingRows />}
        {hasProject && !spaceLoading && spaceError && <p className={styles.sideHint}>{spaceError}</p>}
        {hasProject && !selectionMode && panelMembers.length > 0 && (
          <MemberOnlineRoster
            members={panelMembers}
            onSelect={onSelectMember}
            onOpenConversations={onOpenMemberConversations}
            activeConversationMemberId={isAssistingMember ? activeConversationTargetId : null}
          />
        )}
        {hasProject && selectionMode && panelMembers.length > 0 && (
          <MemberSearch
            members={panelMembers}
            onSelect={onSelectMember}
            onOpenConversations={onOpenMemberConversations}
            onOpenMenu={onOpenMemberMenu}
            activeConversationMemberId={isAssistingMember ? activeConversationTargetId : null}
            placeholder={panelUsesChannelScope ? '搜索频道成员' : '搜索项目成员'}
            channelId={panelUsesChannelScope ? activeChannelId ?? undefined : undefined}
          />
        )}
        {hasProject && !spaceLoading && !spaceError && panelMembers.length === 0 && (
          <p className={styles.sideHint}>{panelUsesChannelScope ? '暂无可见频道成员' : '暂无项目成员'}</p>
        )}
        {!hasProject && user && (
          <>
            <div className={styles.memberSection}>当前账号</div>
            <div className={styles.memberItem}>
              <div className={[styles.memberAvatar, memberPresenceAvatarClass(ownPresenceAvatarStatus)].join(' ')}>
                {ownAvatarUrl
                  ? <img src={ownAvatarUrl} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
                  : ownInitial
                }
              </div>
              <div className={styles.memberCopy}>
                <div className={styles.memberLine}>
                  <strong className={styles.memberItemName}>{ownDisplayName}</strong>
                </div>
                <span className={styles.memberSub}>{ownPresenceSubtitle}</span>
              </div>
            </div>
          </>
        )}
      </div>
    </aside>
  )
}

function MemberOnlineRoster({
  members,
  onSelect,
  onOpenConversations,
  activeConversationMemberId,
}: {
  members: ProjectMember[]
  onSelect: (member: ProjectMember, anchor: PopoverAnchor) => void
  onOpenConversations: (member: ProjectMember) => void
  activeConversationMemberId?: string | null
}) {
  const online = members.filter((member) => memberPresenceStatus(member) !== 'offline')
  const offline = members.filter((member) => memberPresenceStatus(member) === 'offline')
  return (
    <>
      <div className={styles.memberSection}>在线 · {online.length}</div>
      {online.length === 0 && <p className={styles.sideHint}>当前没有在线成员</p>}
      {online.map((member) => (
        <MemberRosterRow
          key={`online-${member.user_id}`}
          member={member}
          active={activeConversationMemberId === member.user_id}
          onSelect={onSelect}
          onOpenConversations={onOpenConversations}
        />
      ))}
      {offline.length > 0 && (
        <>
          <div className={styles.memberSection}>离线 · {offline.length}</div>
          {offline.map((member) => (
            <MemberRosterRow
              key={`offline-${member.user_id}`}
              member={member}
              active={activeConversationMemberId === member.user_id}
              onSelect={onSelect}
              onOpenConversations={onOpenConversations}
            />
          ))}
        </>
      )}
    </>
  )
}

function MemberRosterRow({
  member,
  active,
  onSelect,
  onOpenConversations,
}: {
  member: ProjectMember
  active: boolean
  onSelect: (member: ProjectMember, anchor: PopoverAnchor) => void
  onOpenConversations: (member: ProjectMember) => void
}) {
  const status = memberPresenceStatus(member)
  const name = member.member_display_name || member.account || member.global_account || member.user_id
  const subtitle = member.activity || member.custom_status || presenceLabel(status)
  function openProfile(event: MouseEvent<HTMLElement>) {
    onSelect(member, popoverAnchorFromRect(event.currentTarget.getBoundingClientRect()))
  }
  return (
    <div className={[styles.memberItem, active ? styles.memberItemActive : ''].join(' ')}>
      <button
        className={styles.memberAvatarButton}
        type="button"
        onClick={() => onOpenConversations(member)}
        title={`打开 ${name} 的会话`}
        aria-label={`打开 ${name} 的会话`}
      >
        <span className={[styles.memberAvatar, memberPresenceAvatarClass(status)].join(' ')}>
          {member.avatar_data_url
            ? <img src={member.avatar_data_url} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
            : name[0]?.toUpperCase() ?? '?'}
        </span>
      </button>
      <button className={styles.memberInfoButton} type="button" onClick={openProfile}>
        <span className={styles.memberCopy}>
          <span className={styles.memberLine}>
            <strong className={styles.memberItemName}>{name}</strong>
            <em className={[styles.memberPresencePill, status === 'online' ? styles.memberPresencePillOnline : ''].join(' ')}>
              {presenceLabel(status)}
            </em>
          </span>
          <span className={styles.memberSub}>{subtitle}</span>
        </span>
      </button>
    </div>
  )
}
