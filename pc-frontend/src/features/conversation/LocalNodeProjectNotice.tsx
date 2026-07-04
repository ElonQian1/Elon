import { useNavigate } from 'react-router-dom'
import { launchWinClientProtocol, WIN_CLIENT_DOWNLOAD_URL } from '../node/launchWinClient'
import { useWinClientUpdateCheck } from '../node/useWinClientUpdateCheck'
import { clean } from '../../lib/utils'
import type { LocalNodeStatus } from './localPcRuntime'
import styles from './ConversationPage.module.css'

interface Props {
  localNode: LocalNodeStatus | null
  localNodeReady: boolean
  localNodeId: string
  localBindStatus: string
  localNodeError: string
  projectBoundToLocalNode: boolean
}

export default function LocalNodeProjectNotice({
  localNode,
  localNodeReady,
  localNodeId,
  localBindStatus,
  localNodeError,
  projectBoundToLocalNode,
}: Props) {
  const navigate = useNavigate()
  const winClientUpdate = useWinClientUpdateCheck(localNode?.version, localNodeReady)
  const needsUpdate = localNodeReady && winClientUpdate.kind === 'update'
  const localNodeLine = `${clean(localNode?.device_name) || '本机'} · ${localNodeId}${localBindStatus ? ` · ${localBindStatus}` : ''}`
  const statusText = localNodeReady
    ? needsUpdate ? `${localNodeLine} · ${winClientUpdate.title}` : localNodeLine
    : localNodeOfflineText(localNodeError, localNode)
  const noticeClass = !localNodeReady
    ? styles.localNodeNoticeWarn
    : needsUpdate
      ? styles.localNodeNoticeUpdate
      : projectBoundToLocalNode
        ? styles.localNodeNoticeOk
        : styles.localNodeNoticeInfo
  const label = needsUpdate
    ? 'Win 端可更新'
    : localNodeReady
      ? projectBoundToLocalNode ? '当前电脑节点已锁定' : '当前电脑节点优先'
      : '本机 Win 端未就绪'

  return (
    <div className={[styles.localNodeNotice, noticeClass].join(' ')}>
      <strong>{label}</strong>
      <span>{statusText}</span>
      {(!localNodeReady || needsUpdate) && (
        <div className={styles.localNodeActions}>
          {needsUpdate ? (
            <>
              <button type="button" onClick={() => navigate('/node')}>更新 Win 端</button>
              <a href={WIN_CLIENT_DOWNLOAD_URL} download>下载新版</a>
            </>
          ) : (
            <>
              <button type="button" onClick={launchWinClientProtocol}>启动 Win 端</button>
              <button type="button" onClick={() => navigate('/node')}>检查自启动</button>
              <a href={WIN_CLIENT_DOWNLOAD_URL} download>下载</a>
            </>
          )}
          {needsUpdate && <button type="button" onClick={() => navigate('/node')}>节点设置</button>}
        </div>
      )}
    </div>
  )
}

function localNodeOfflineText(error: string, localNode: LocalNodeStatus | null): string {
  const text = clean(error)
  if (localNode) {
    if (localNode.connected === false) {
      return `本机 Win 端已启动，但尚未连上云端${localNode.last_event ? `：${localNode.last_event}` : '。'}`
    }
    if (localNode.codex_cli?.available === false) {
      return '本机 Win 端已启动，但 Codex CLI 未就绪；请到节点设置修复。'
    }
    return '本机 Win 端已启动，但还未绑定当前网页账号；请在节点设置重新绑定当前账号。'
  }
  if (!text || /failed to fetch/i.test(text)) {
    return '未检测到本机 Win 端；请启动客户端，启动后可在节点设置确认开机自启动。'
  }
  return text
}
