import { useMemo } from 'react'
import { createPortal } from 'react-dom'
import type { User } from '../../store/auth'
import { clean } from '../../lib/utils'
import AgentRunsPanel from '../dev/AgentRunsPanel'
import {
  channelPermissionSummary,
  membersHaveChannelPermissionMap,
  membersForChannel,
} from './memberUtils'
import {
  MemberSearch,
  MemberContextMenu,
  MemberProfilePopover,
  MemberContextSummary,
  MemberLoadingRows,
  memberPresenceAvatarClass,
} from './MemberPanel'
import type { MemberMenuRequest, MemberModerationAction } from './MemberPanel'
import { normalizeOwnPresenceStatus, ownPresenceSummary } from './conversationPageUtils'
import type { Channel, Project, ProjectMember, UserPresenceSettings } from './types'
import panelStyles from './ConversationMemberPanel.module.css'
import sharedStyles from './ConversationPage.module.css'

export type MemberPanelScope = 'channel' | 'project'

interface ConversationMemberPanelProps {
  activeProjectId?: string | null
  activeChannelId?: string | null
  activeProject?: Project
  activeChannel?: Channel
  channels: Channel[]
  user: User | null
  myPresence: UserPresenceSettings | null
  members: ProjectMember[]
  spaceLoading: boolean
  spaceError: string
  memberPanelScope: MemberPanelScope
  memberMenu: MemberMenuRequest | null
  selectedMember: ProjectMember | null
  memberPopoverY: number
  isDevChannel: boolean
  activeWorkspacePath: string
  activeConversationMemberId?: string | null
  canModerateMembers: boolean
  canManageMembers: boolean
  canInviteMembers: boolean
  canViewMemberAudit: boolean
  canUseRoleManager: boolean
  canManagePermissions: boolean
  onSetMemberPanelScope: (scope: MemberPanelScope) => void
  onOpenPresence: () => void
  onOpenDirectory: () => void
  onOpenMembersPage: () => void
  onOpenInvites: () => void
  onOpenModerationCenter: () => void
  onOpenRoleManager: () => void
  onOpenAudit: () => void
  onOpenPermissionDrawer: () => void
  onCloseMemberMenu: () => void
  onCloseSelectedMember: () => void
  onOpenProfile: (member: ProjectMember, y: number) => void
  onOpenDetails: (member: ProjectMember) => void
  onOpenConversations: (member: ProjectMember) => void
  onOpenPermissions: (member: ProjectMember) => void
  onOpenRoles: (member: ProjectMember) => void
  onModerate: (member: ProjectMember, action: MemberModerationAction, durationMinutes?: number) => Promise<void>
  onRemove: (member: ProjectMember) => Promise<boolean | void>
  onSelectMember: (member: ProjectMember, y: number) => void
  onOpenMemberMenu: (request: MemberMenuRequest) => void
}

export default function ConversationMemberPanel({
  activeProjectId,
  activeChannelId,
  activeProject,
  activeChannel,
  channels,
  user,
  myPresence,
  members,
  spaceLoading,
  spaceError,
  memberPanelScope,
  memberMenu,
  selectedMember,
  memberPopoverY,
  isDevChannel,
  activeWorkspacePath,
  activeConversationMemberId,
  canModerateMembers,
  canManageMembers,
  canInviteMembers,
  canViewMemberAudit,
  canUseRoleManager,
  canManagePermissions,
  onSetMemberPanelScope,
  onOpenPresence,
  onOpenDirectory,
  onOpenMembersPage,
  onOpenInvites,
  onOpenModerationCenter,
  onOpenRoleManager,
  onOpenAudit,
  onOpenPermissionDrawer,
  onCloseMemberMenu,
  onCloseSelectedMember,
  onOpenProfile,
  onOpenDetails,
  onOpenConversations,
  onOpenPermissions,
  onOpenRoles,
  onModerate,
  onRemove,
  onSelectMember,
  onOpenMemberMenu,
}: ConversationMemberPanelProps) {
  const hasChannelMemberPermissions = !!activeChannelId && membersHaveChannelPermissionMap(members, activeChannelId)
  const activeMemberPanelScope: MemberPanelScope = activeChannelId && memberPanelScope === 'channel' ? 'channel' : 'project'
  const panelUsesChannelScope = activeMemberPanelScope === 'channel'
  const panelUsesChannelPermissions = panelUsesChannelScope && hasChannelMemberPermissions
  const panelMembers = useMemo(
    () => panelUsesChannelScope ? membersForChannel(members, activeChannelId ?? undefined) : members,
    [members, activeChannelId, panelUsesChannelScope],
  )
  const memberPanelTitle = panelUsesChannelScope ? '频道成员' : activeProjectId ? '项目大厅' : '工作台'
  const memberPanelContext = panelUsesChannelScope
    ? activeChannel?.name ?? '当前频道'
    : activeProject?.name ?? '我的项目'
  const memberPanelCount = activeProjectId ? panelMembers.length : (user ? 1 : 0)
  const memberPanelSummary = panelUsesChannelScope && activeChannel
    ? panelUsesChannelPermissions
      ? channelPermissionSummary(activeChannel, panelMembers.length, members.length, true)
      : `${activeChannel.name} · 当前频道未设置成员级可见限制，显示项目内全部成员`
    : activeProjectId
      ? `项目大厅显示 ${members.length} 位项目成员，适合查看全局在线、角色和管理状态`
      : '个人 AI 工作台'

  const ownPresenceStatus = normalizeOwnPresenceStatus(myPresence?.status ?? user?.status ?? 'online')
  const ownPresenceAvatarStatus = ownPresenceStatus === 'invisible' ? 'offline' : ownPresenceStatus
  const ownPresenceSubtitle = ownPresenceSummary(myPresence, ownPresenceStatus)
  const ownAvatarUrl = clean(user?.avatar_data_url ?? '')
  const ownDisplayName = user ? (user.nickname ?? user.account) : ''
  const ownInitial = ownDisplayName ? ownDisplayName[0].toUpperCase() : '?'

  return (
    <aside className={panelStyles.memberPanel}>
      <div className={panelStyles.memberTitle}>
        <div className={panelStyles.memberTitleCopy}>
          <strong>{memberPanelTitle}{memberPanelCount > 0 ? ` — ${memberPanelCount}` : ''}</strong>
          <span>{memberPanelContext}</span>
        </div>
        <div className={panelStyles.memberActions}>
          <button className={panelStyles.memberInviteBtn} type="button" onClick={onOpenPresence}>状态</button>
          {activeProjectId && <button className={panelStyles.memberInviteBtn} type="button" onClick={onOpenDirectory}>目录</button>}
          {activeProjectId && <button className={panelStyles.memberInviteBtn} type="button" onClick={onOpenMembersPage}>成员页</button>}
          {activeProjectId && canInviteMembers && <button className={panelStyles.memberInviteBtn} type="button" onClick={onOpenInvites}>邀请</button>}
          {activeProjectId && <button className={panelStyles.memberInviteBtn} type="button" onClick={onOpenModerationCenter}>管理</button>}
          {activeProjectId && canUseRoleManager && <button className={panelStyles.memberInviteBtn} type="button" onClick={onOpenRoleManager}>角色</button>}
          {activeProjectId && canViewMemberAudit && <button className={panelStyles.memberInviteBtn} type="button" onClick={onOpenAudit}>日志</button>}
          {activeProjectId && activeChannelId && canManagePermissions && (
            <button className={panelStyles.memberInviteBtn} type="button" onClick={onOpenPermissionDrawer}>权限</button>
          )}
        </div>
      </div>
      <div className={panelStyles.memberList}>
        {memberMenu && createPortal(
          <MemberContextMenu
            member={memberMenu.member}
            x={memberMenu.x}
            y={memberMenu.y}
            canModerate={canModerateMembers && memberMenu.member.user_id !== user?.id}
            canRemove={canManageMembers && memberMenu.member.user_id !== user?.id}
            onClose={onCloseMemberMenu}
            onOpenProfile={onOpenProfile}
            onOpenDetails={onOpenDetails}
            onOpenConversations={onOpenConversations}
            onOpenPermissions={activeProjectId && activeChannelId && canManagePermissions ? onOpenPermissions : undefined}
            onOpenRoles={activeProjectId && canUseRoleManager ? onOpenRoles : undefined}
            onModerate={onModerate}
            onRemove={onRemove}
          />,
          document.body,
        )}
        {selectedMember && createPortal(
          <MemberProfilePopover
            member={selectedMember}
            anchorY={memberPopoverY}
            projectId={activeProjectId ?? undefined}
            channels={channels}
            channel={activeChannel}
            canModerate={canModerateMembers && selectedMember.user_id !== user?.id}
            canRemove={canManageMembers && selectedMember.user_id !== user?.id}
            onClose={onCloseSelectedMember}
            onOpenDetails={onOpenDetails}
            onOpenConversations={onOpenConversations}
            onOpenRoles={canUseRoleManager ? onOpenRoles : undefined}
            onModerate={onModerate}
            onRemove={onRemove}
          />,
          document.body,
        )}
        {activeProjectId && isDevChannel && activeWorkspacePath && (
          <div className={panelStyles.agentRunsSlot}>
            <AgentRunsPanel workspacePath={activeWorkspacePath} />
          </div>
        )}
        {activeProjectId && (
          <div className={panelStyles.memberScopeSwitch} role="group" aria-label="成员列表范围">
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
        {activeProjectId && (
          <MemberContextSummary
            title={panelUsesChannelScope ? '当前频道' : '项目大厅'}
            label={memberPanelSummary}
            members={panelMembers}
            channel={panelUsesChannelScope ? activeChannel : undefined}
            projectTotal={members.length}
            usingChannelPermissions={panelUsesChannelPermissions}
          />
        )}
        {activeProjectId && spaceLoading && panelMembers.length === 0 && (
          <MemberLoadingRows />
        )}
        {activeProjectId && !spaceLoading && spaceError && (
          <p className={sharedStyles.sideHint}>{spaceError}</p>
        )}
        {activeProjectId && panelMembers.length > 0 && (
          <MemberSearch
            members={panelMembers}
            onSelect={onSelectMember}
            onOpenConversations={onOpenConversations}
            onOpenMenu={onOpenMemberMenu}
            activeConversationMemberId={activeConversationMemberId}
            placeholder={panelUsesChannelScope ? '搜索频道成员' : '搜索项目成员'}
            channelId={panelUsesChannelScope ? activeChannelId ?? undefined : undefined}
          />
        )}
        {activeProjectId && !spaceLoading && !spaceError && panelMembers.length === 0 && (
          <p className={sharedStyles.sideHint}>{panelUsesChannelScope ? '暂无可见频道成员' : '暂无项目成员'}</p>
        )}
        {!activeProjectId && user && (
          <>
            <div className={sharedStyles.memberSection}>当前账号</div>
            <div className={sharedStyles.memberItem}>
              <div className={[sharedStyles.memberAvatar, memberPresenceAvatarClass(ownPresenceAvatarStatus)].join(' ')}>
                {ownAvatarUrl
                  ? <img src={ownAvatarUrl} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
                  : ownInitial
                }
              </div>
              <div className={sharedStyles.memberCopy}>
                <div className={sharedStyles.memberLine}>
                  <strong className={sharedStyles.memberItemName}>{ownDisplayName}</strong>
                </div>
                <span className={sharedStyles.memberSub}>{ownPresenceSubtitle}</span>
              </div>
            </div>
          </>
        )}
      </div>
    </aside>
  )
}
