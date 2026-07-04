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
    : localNodeOfflineText(localNodeError)
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
      : '未锁定当前电脑节点'

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
              <a href={WIN_CLIENT_DOWNLOAD_URL} download>下载</a>
            </>
          )}
          <button type="button" onClick={() => navigate('/node')}>节点设置</button>
        </div>
      )}
    </div>
  )
}

function localNodeOfflineText(error: string): string {
  const text = clean(error)
  if (!text || /failed to fetch/i.test(text)) {
    return '未检测到本机 Win 端；电脑重启后请先启动节点客户端并保持登录。'
  }
  return text
}
