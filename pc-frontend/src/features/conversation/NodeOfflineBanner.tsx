/**
 * NodeOfflineBanner — 项目页轻量节点离线提示
 *
 * 只在「节点已注册但未运行」时出现（如电脑重启后）。
 * 节点在线或未注册时自动隐藏。
 */
import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api } from '../../api/client'
import { launchWinClientProtocol, WIN_CLIENT_DOWNLOAD_URL } from '../node/launchWinClient'

const POLL_MS = 30_000

interface NodeInfo {
  node_id: string
  display_name: string
  device_name?: string
  online: boolean
}

interface Props {
  localNodeReady?: boolean
  localNodeId?: string
}

export default function NodeOfflineBanner({ localNodeReady = false, localNodeId = '' }: Props) {
  const navigate = useNavigate()
  const [offlineNode, setOfflineNode] = useState<NodeInfo | null>(null)
  const [dismissed, setDismissed] = useState(false)

  useEffect(() => {
    if (localNodeReady) {
      setOfflineNode(null)
      setDismissed(false)
      return
    }
    check()
    const t = setInterval(check, POLL_MS)
    return () => clearInterval(t)
  }, [localNodeReady, localNodeId])

  async function check() {
    try {
      const data = await api.get<{ nodes?: NodeInfo[] }>('/api/me/nodes')
      const nodes = data.nodes ?? []
      const online  = nodes.find((n) => n.online)
      const offline = nodes.find((n) => !n.online && n.node_id === localNodeId)
        ?? nodes.find((n) => !n.online)
      if (!online && offline) {
        setOfflineNode(offline)
      } else {
        setOfflineNode(null)
        setDismissed(false)
      }
    } catch { /* 静默，保持现状 */ }
  }

  if (!offlineNode || dismissed) return null

  const name = offlineNode.display_name || offlineNode.device_name || '本机节点'

  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 10,
      padding: '9px 16px',
      background: '#2a1f0f',
      borderBottom: '1px solid #6b3c10',
      color: '#e8a460',
      fontSize: 13,
      flexShrink: 0,
    }}>
      <span style={{ fontSize: 16 }}>⚠️</span>
      <span style={{ flex: 1, lineHeight: 1.5 }}>
        账号下的 <strong style={{ color: '#f5c07a' }}>{name}</strong> 当前未在线。
        如果这是这台电脑，请启动 Win 端，并在节点设置里确认开机自启动。
      </span>
      <button
        type="button"
        onClick={launchWinClientProtocol}
        style={{
          background: '#1f6f3d', border: 'none', borderRadius: 5,
          color: '#d9ffe5', padding: '5px 14px',
          cursor: 'pointer', fontSize: 12, fontWeight: 800, flexShrink: 0,
        }}
      >
        启动 Win 端
      </button>
      <button
        type="button"
        onClick={() => navigate('/node')}
        style={{
          background: 'rgba(245, 192, 122, .12)', border: '1px solid rgba(245, 192, 122, .34)', borderRadius: 5,
          color: '#ffe1b2', padding: '5px 14px',
          cursor: 'pointer', fontSize: 12, fontWeight: 800, flexShrink: 0,
        }}
      >
        检查自启动
      </button>
      <a
        href={WIN_CLIENT_DOWNLOAD_URL}
        download
        style={{
          background: '#7a4510', border: 'none', borderRadius: 5,
          color: '#ffd8a8', padding: '5px 14px',
          cursor: 'pointer', fontSize: 12, fontWeight: 700, flexShrink: 0,
          textDecoration: 'none', display: 'inline-block',
          transition: 'background .1s',
        }}
        onMouseEnter={(e) => (e.currentTarget.style.background = '#9a5518')}
        onMouseLeave={(e) => (e.currentTarget.style.background = '#7a4510')}
      >
        重新下载
      </a>
      <button
        type="button"
        onClick={() => setDismissed(true)}
        title="暂时关闭（节点上线后自动消失）"
        style={{
          background: 'none', border: 'none',
          color: '#a07840', cursor: 'pointer',
          fontSize: 18, lineHeight: 1, padding: '2px 4px', flexShrink: 0,
        }}
      >×</button>
    </div>
  )
}
