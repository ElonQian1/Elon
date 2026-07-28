import SidebarUserStrip from '../shell/SidebarUserStrip'
import MemberConversationList from './MemberConversationList'
import type { MemberConversationEntry } from './memberConversationApi'
import type { Channel, Project } from './types'
import styles from './ConversationChannelPanel.module.css'

interface ConversationChannelPanelProps {
  activeProjectId?: string | null
  activeProject?: Project
  projects: Project[]
  projectsLoaded: boolean
  filteredChannels: Channel[]
  activeChannelId?: string | null
  channelSearch: string
  sessionView: string | 'new' | null
  memberConversations: MemberConversationEntry[]
  hasConversationTarget: boolean
  activeConversationTargetName: string
  isOwnConversationTarget: boolean
  onBackToProjects: () => void
  onOpenProjectHome: () => void | Promise<void>
  onOpenProjectSettings: () => void
  onCreateProject: () => void
  onChannelSearchChange: (value: string) => void
  onSelectChannel: (channelId: string) => void | Promise<void>
  onSelectProject: (projectId: string) => void | Promise<void>
  onOpenSession: (conversationId: string) => void
  onStartNewSession: () => void
  onResetConversationTarget: () => void
}

export default function ConversationChannelPanel({
  activeProjectId,
  activeProject,
  projects,
  projectsLoaded,
  filteredChannels,
  activeChannelId,
  channelSearch,
  sessionView,
  memberConversations,
  hasConversationTarget,
  activeConversationTargetName,
  isOwnConversationTarget,
  onBackToProjects,
  onOpenProjectHome,
  onOpenProjectSettings,
  onCreateProject,
  onChannelSearchChange,
  onSelectChannel,
  onSelectProject,
  onOpenSession,
  onStartNewSession,
  onResetConversationTarget,
}: ConversationChannelPanelProps) {
  const visibleProjects = projects.filter((project) => (
    !channelSearch || project.name.toLowerCase().includes(channelSearch.toLowerCase())
  ))

  return (
    <aside className={styles.channelPanel}>
      <div className={styles.workspaceTitle}>
        {activeProjectId ? (
          <>
            <button
              className={styles.workspaceBackBtn}
              onClick={onBackToProjects}
              title="返回项目列表"
              type="button"
            >←</button>
            <button
              className={styles.workspaceHomeBtn}
              onClick={onOpenProjectHome}
              title="项目首页"
              type="button"
            >
              <strong className={styles.workspaceTitleText}>{activeProject?.name}</strong>
              {activeProject?.description && (
                <span className={styles.workspaceTitleMeta}>{activeProject.description}</span>
              )}
            </button>
            <button
              className={styles.iconBtn}
              onClick={onOpenProjectSettings}
              title="项目设置"
              type="button"
              style={{ fontSize: 14 }}
            >⚙</button>
          </>
        ) : (
          <>
            <div style={{ minWidth: 0, flex: 1 }}>
              <strong className={styles.workspaceTitleText}>我的项目</strong>
            </div>
            <button className={styles.iconBtn} onClick={onCreateProject} title="新建项目" type="button">+</button>
          </>
        )}
      </div>

      <div className={styles.channelSearch}>
        <input
          value={channelSearch}
          onChange={(event) => onChannelSearchChange(event.target.value)}
          placeholder={activeProjectId ? '搜索频道' : '搜索项目'}
        />
      </div>

      <div className={styles.channelList}>
        {activeProjectId ? (
          <>
            {filteredChannels.length === 0 ? (
              <div style={{ padding: '12px 16px', color: 'var(--text-muted)', fontSize: 13 }}>
                还没有频道
              </div>
            ) : (
              filteredChannels.map((channel) => {
                const isDev = channel.kind === 'ai_development'
                return (
                  <button
                    key={channel.id}
                    className={[
                      styles.channelItem,
                      isDev ? styles.devChannel : '',
                      channel.id === activeChannelId ? styles.channelActive : '',
                    ].join(' ')}
                    onClick={() => onSelectChannel(channel.id)}
                    type="button"
                  >
                    <span className={styles.channelGlyph}>{isDev ? '🛠' : '#'}</span>
                    <span className={styles.channelMain}>
                      <strong>{channel.name}</strong>
                      {channel.description && <span>{channel.description}</span>}
                    </span>
                  </button>
                )
              })
            )}

            {hasConversationTarget && (
              <MemberConversationList
                conversations={memberConversations}
                selectedId={sessionView}
                targetName={activeConversationTargetName}
                isOwnTarget={isOwnConversationTarget}
                onOpen={onOpenSession}
                onStartNew={onStartNewSession}
                onResetTarget={onResetConversationTarget}
              />
            )}
          </>
        ) : (
          <>
            {!projectsLoaded && (
              <div style={{ padding: '6px 9px', color: 'var(--text-muted)', fontSize: 13 }}>读取中…</div>
            )}
            {visibleProjects.map((project) => (
              <button
                key={project.id}
                className={styles.channelItem}
                onClick={() => onSelectProject(project.id)}
                type="button"
              >
                <span className={styles.channelGlyph}>
                  {project.icon_data_url || project.icon
                    ? (
                      <img
                        src={project.icon_data_url || project.icon}
                        alt=""
                        style={{ width: 20, height: 20, borderRadius: 4, objectFit: 'cover' }}
                      />
                    )
                    : '📦'
                  }
                </span>
                <span className={styles.channelMain}>
                  <strong>{project.name}</strong>
                  {project.description && <span>{project.description}</span>}
                </span>
              </button>
            ))}
            {projectsLoaded && projects.length === 0 && (
              <div style={{ padding: '6px 9px', color: 'var(--text-muted)', fontSize: 12 }}>
                暂无项目，点击 + 新建
              </div>
            )}
          </>
        )}
      </div>

      <SidebarUserStrip />
    </aside>
  )
}
