/**
 * NodeOfflineBanner — 项目页轻量节点离线提示
 *
 * 只在「节点已注册但未运行」时出现（如电脑重启后），
 * 提供一键下载启动脚本，可关闭。
 * 节点在线或未注册时自动隐藏。
 */
import { useEffect, useState } from 'react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'

const SERVER_URL = 'http://43.139.149.158:8080'
const WS_URL    = 'ws://43.139.149.158:8080/agent/ws'
const POLL_MS   = 30_000

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
        // 节点上线后自动取消关闭状态，重新可见
      } else {
        setOfflineNode(null)
        setDismissed(false)
      }
    } catch { /* 静默，保持现状 */ }
  }

  function downloadBat() {
    const token = useAuthStore.getState().token ?? ''
    // bat 脚本：下载节点 exe 并注入用户 token 启动（已有则覆盖，保证最新版本）
    const bat = [
      '@echo off',
      'title 一龙开发平台 — 重新启动',
      'echo.',
      'echo =============================================',
      'echo   一龙 PC 节点  重新启动脚本',
      'echo =============================================',
      'echo.',
      'echo 正在下载最新节点程序...',
      `curl -L --progress-bar -o "%TEMP%\\elon-pc-node.exe" "${SERVER_URL}/api/node-agent/download/windows"`,
      'if errorlevel 1 (',
      '  echo.',
      '  echo [错误] 下载失败，请检查网络后重试。',
      '  pause',
      '  exit /b 1',
      ')',
      'echo.',
      'echo 启动节点，正在连接到你的账号...',
      `set NODE_USER_TOKEN=${token}`,
      `set NODE_CLOUD_URL=${WS_URL}`,
      '"%TEMP%\\elon-pc-node.exe"',
      'echo.',
      'echo 节点已退出。',
      'pause',
    ].join('\r\n')

    const blob = new Blob([bat], { type: 'text/plain;charset=gbk' })
    const url  = URL.createObjectURL(blob)
    const a    = document.createElement('a')
    a.href     = url
    a.download = '重启一龙开发平台.bat'
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
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
        <strong style={{ color: '#f5c07a' }}>{name}</strong> 节点未运行——
        电脑重启后需要重新启动节点，AI 才能访问本机命令行和文件。
      </span>
      <button
        type="button"
        onClick={downloadBat}
        style={{
          background: '#7a4510', border: 'none', borderRadius: 5,
          color: '#ffd8a8', padding: '5px 14px',
          cursor: 'pointer', fontSize: 12, fontWeight: 700, flexShrink: 0,
          transition: 'background .1s',
        }}
        onMouseEnter={(e) => (e.currentTarget.style.background = '#9a5518')}
        onMouseLeave={(e) => (e.currentTarget.style.background = '#7a4510')}
      >
        下载重启脚本 .bat
      </button>
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
