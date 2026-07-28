import { clean } from '../../lib/utils'
import NodeOfflineBanner from './NodeOfflineBanner'
import { shortNodeId } from './conversationPageUtils'
import type { Channel, Project } from './types'
import type { LocalNodeStatus } from './useLocalNodeStatus'
import styles from './ConversationStatusStack.module.css'

interface ConversationStatusStackProps {
  activeProjectId?: string | null
  activeProject?: Project
  activeChannel?: Channel
  activeProjectRoleLabel: string
  localNode: LocalNodeStatus | null
  localNodeId: string
  localNodeReady: boolean
  localNodeError: string
  localBindStatus: string
  projectBoundToLocalNode: boolean
  activeChannelBlocksAi: boolean
  activeChannelIsNotAi: boolean
  sessionView: string | 'new' | null
}

export default function ConversationStatusStack({
  activeProjectId,
  activeProject,
  activeChannel,
  activeProjectRoleLabel,
  localNode,
  localNodeId,
  localNodeReady,
  localNodeError,
  localBindStatus,
  projectBoundToLocalNode,
  activeChannelBlocksAi,
  activeChannelIsNotAi,
  sessionView,
}: ConversationStatusStackProps) {
  return (
    <div className={styles.chatStatusStack}>
      {activeProjectId && (
        <>
          <NodeOfflineBanner />
          <div className={[
            styles.localNodeNotice,
            !localNodeReady ? styles.localNodeNoticeWarn : projectBoundToLocalNode ? styles.localNodeNoticeOk : styles.localNodeNoticeInfo,
          ].join(' ')}>
            <strong>
              {localNodeReady
                ? projectBoundToLocalNode ? '当前电脑节点已锁定' : '当前电脑节点优先'
                : '未锁定当前电脑节点'}
            </strong>
            <span>
              {localNodeReady
                ? `${clean(localNode?.device_name) || '本机'} · ${localNodeId}${localBindStatus ? ` · ${localBindStatus}` : ''}`
                : localNodeError || '请确认 Windows 节点助手正在运行并已登录当前账号'}
            </span>
          </div>
          <div className={styles.projectRouteNotice}>
            <span>
              <strong>当前项目</strong>
              {activeProject?.name ?? activeProjectId} · {activeProjectRoleLabel}
              {activeChannel ? ` · ${activeChannel.name}` : ' · 默认 AI开发频道'}
            </span>
            <span>
              {projectBoundToLocalNode
                ? '会使用本机节点'
                : activeProject?.node_id
                  ? `项目记录绑定 ${shortNodeId(activeProject.node_id)}`
                  : '项目尚未记录节点'}
            </span>
          </div>
          {(activeChannelBlocksAi || (!sessionView && activeChannelIsNotAi)) && (
            <div className={styles.permissionNotice}>
              {activeChannelIsNotAi
                ? '当前输入会通过 AI开发 频道发起项目 AI 对话。'
                : '当前角色不能在这个频道发起 AI 开发。'}
            </div>
          )}
        </>
      )}
    </div>
  )
}
