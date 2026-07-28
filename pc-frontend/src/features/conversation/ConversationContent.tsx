import type { RefObject } from 'react'
import type { User } from '../../store/auth'
import type { TaskContext } from '../dev/types'
import ProjectLandingView from './ProjectLanding'
import ConversationFeed from './ConversationFeed'
import type { MessageGroup } from './messageFlow'
import type { Channel, Message, Project, ProjectLanding as ProjectLandingData } from './types'
import styles from './ConversationPage.module.css'

interface ConversationContentProps {
  activeProjectId?: string | null
  activeChannelId?: string | null
  sessionView: string | 'new' | null
  activeProject?: Project
  channels: Channel[]
  landing?: ProjectLandingData | null
  isAssistingMember: boolean
  activeConversationTargetName: string
  feedRef: RefObject<HTMLDivElement>
  feedLoading: boolean
  displayMessages: Message[]
  messageGroups: MessageGroup[]
  taskContext: TaskContext
  isDevChannel: boolean
  user: User | null
  hasRunningTask: boolean
  sendingMessage: boolean
  showNewMsg: boolean
  onCreateProject: () => void
  onSelectLandingChannel: (channelId: string) => void | Promise<void>
  onFeedScroll: () => void
  onScrollToBottom: () => void
  onCancelTask: (id: string) => Promise<void>
  onApproveTool: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
}

export default function ConversationContent({
  activeProjectId,
  activeChannelId,
  sessionView,
  activeProject,
  channels,
  landing,
  isAssistingMember,
  activeConversationTargetName,
  feedRef,
  feedLoading,
  displayMessages,
  messageGroups,
  taskContext,
  isDevChannel,
  user,
  hasRunningTask,
  sendingMessage,
  showNewMsg,
  onCreateProject,
  onSelectLandingChannel,
  onFeedScroll,
  onScrollToBottom,
  onCancelTask,
  onApproveTool,
}: ConversationContentProps) {
  return (
    <>
      {sessionView === null ? (
        <div className={styles.messageList}>
          {!activeProjectId ? (
            <div className={styles.emptyState}>
              <strong>欢迎使用一龙工作台</strong>
              <p>从左侧选择一个项目，或新建一个开始开发。</p>
              <button className={styles.bigCreateBtn} onClick={onCreateProject}>+ 新建项目</button>
            </div>
          ) : (
            isAssistingMember ? (
              <div className={styles.emptyState}>
                <strong>{activeConversationTargetName} 的项目会话</strong>
                <p>从左侧选择一个公开会话后，你可以用自己的账号继续协助他。</p>
              </div>
            ) : activeProject && (
              <ProjectLandingView
                project={activeProject}
                channels={channels}
                landing={landing ?? null}
                onSelectChannel={onSelectLandingChannel}
              />
            )
          )}
        </div>
      ) : (
        <ConversationFeed
          sessionView={sessionView}
          feedRef={feedRef}
          feedLoading={feedLoading}
          displayMessages={displayMessages}
          messageGroups={messageGroups}
          taskContext={taskContext}
          isDevChannel={isDevChannel}
          user={user}
          hasRunningTask={hasRunningTask}
          sendingMessage={sendingMessage}
          onScroll={onFeedScroll}
          onCancelTask={onCancelTask}
          onApproveTool={onApproveTool}
        />
      )}
      {showNewMsg && activeChannelId && (
        <button className={styles.newMsgBtn} onClick={onScrollToBottom} type="button">
          ↓ 新消息
        </button>
      )}
    </>
  )
}
