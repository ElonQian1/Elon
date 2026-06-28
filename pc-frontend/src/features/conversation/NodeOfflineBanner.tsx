/**
 * NodeOfflineBanner — 项目页轻量节点离线提示
 *
 * 只在「节点已注册但未运行」时出现（如电脑重启后）。
 * 节点在线或未注册时自动隐藏。
 */
import { useEffect, useState } from 'react'
import { api } from '../../api/client'

const POLL_MS = 30_000

interface NodeInfo {
  node_id: string
  display_name: string
  device_name?: string
  online: boolean
}

export default function NodeOfflineBanner() {
  const [offlineNode, setOfflineNode] = useState<NodeInfo | null>(null)
  const [dismissed, setDismissed] = useState(false)

  useEffect(() => {
    check()
    const t = setInterval(check, POLL_MS)
    return () => clearInterval(t)
  }, [])

  async function check() {
    try {
      const data = await api.get<{ nodes?: NodeInfo[] }>('/api/me/nodes')
      const nodes = data.nodes ?? []
      const online  = nodes.find((n) => n.online)
      const offline = nodes.find((n) => !n.online)
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
        <strong style={{ color: '#f5c07a' }}>{name}</strong> 未运行——
        双击桌面快捷方式「一龙开发平台」重新启动即可。
        找不到快捷方式？
      </span>
      <a
        href="/api/node-agent/download/windows"
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
