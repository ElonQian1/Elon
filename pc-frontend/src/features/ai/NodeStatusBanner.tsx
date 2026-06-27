/**
 * NodeStatusBanner — 本机 PC 节点状态检测 + 引导组件
 *
 * 策略：
 *  - 每 6s 轮询 /api/me/nodes
 *  - 无节点：展示"下载 → 安装 → 启动"3 步引导 + 下载按钮
 *  - 节点在线：绿色已就绪，提供"进入本机开发频道"按钮（这是真正能执行 CLI 的地方）
 *  - 不影响当前对话内容，以浮动横幅形式挂在聊天区顶部
 */
import { useEffect, useState, useRef } from 'react'
import { api } from '../../api/client'
import styles from './NodeStatusBanner.module.css'

interface NodeInfo {
  node_id: string
  display_name: string
  device_name?: string
  online: boolean
  ai_cli_ready: boolean
  allowed_clis?: string[]
}

type Status = 'loading' | 'no_node' | 'offline' | 'online'

const DOWNLOAD_URL = '/api/node-agent/download/windows'
const POLL_INTERVAL_MS = 6000

export default function NodeStatusBanner() {
  const [status, setStatus] = useState<Status>('loading')
  const [node, setNode] = useState<NodeInfo | null>(null)
  const [expanded, setExpanded] = useState(false)
  const [step, setStep] = useState<1 | 2 | 3>(1)
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    checkNodes()
    pollRef.current = setInterval(checkNodes, POLL_INTERVAL_MS)
    return () => { if (pollRef.current) clearInterval(pollRef.current) }
  }, []) // eslint-disable-line

  async function checkNodes() {
    try {
      const data = await api.get<{ nodes?: NodeInfo[] }>('/api/me/nodes')
      const nodes = data.nodes ?? []
      const online = nodes.find((n) => n.online)
      if (online) {
        setNode(online)
        setStatus('online')
        setExpanded(false)
      } else if (nodes.length > 0) {
        setNode(nodes[0])
        setStatus('offline')
      } else {
        setNode(null)
        setStatus('no_node')
      }
    } catch {
      // 网络错误时保持当前状态
    }
  }

  function openDevChannel() {
    // 跳到项目对话页（用户的 PC 节点项目开发频道）
    window.location.href = '/pc/'
  }

  // 在线且折叠时只显示一行状态 pill
  if (status === 'online' && !expanded) {
    return (
      <div className={styles.pill} title={`本机 CLI 已就绪：${node?.display_name ?? node?.device_name ?? ''}`}>
        <span className={styles.pillDot} />
        <span className={styles.pillText}>本机 CLI 已就绪</span>
        <button className={styles.pillAction} type="button" onClick={openDevChannel}>
          进入开发频道 →
        </button>
        <button className={styles.pillClose} type="button" onClick={() => setExpanded(true)} title="展开">
          ⋯
        </button>
      </div>
    )
  }

  if (status === 'loading') return null

  // 无节点或离线：展示引导卡
  return (
    <div className={styles.banner}>
      <div className={styles.bannerHeader}>
        <span className={[styles.statusDot, status === 'offline' ? styles.dotOffline : styles.dotNone].join(' ')} />
        <strong className={styles.bannerTitle}>
          {status === 'offline'
            ? `本机节点未运行（${node?.display_name ?? node?.device_name ?? '未知设备'}）`
            : '连接你的 Windows 电脑，让 AI 真正帮你干活'}
        </strong>
        {status === 'online' && (
          <button className={styles.closeBtn} type="button" onClick={() => setExpanded(false)}>×</button>
        )}
      </div>

      <p className={styles.bannerDesc}>
        {status === 'offline'
          ? '节点已注册但当前未运行。请在你的 Windows 电脑上启动「一龙 PC 节点」，AI 就能访问本机文件和执行命令行。'
          : '一龙 PC 节点（约 11 MB）运行在你的 Windows 电脑上，连接后 AI 可以完整访问命令行、读写文件、自动开发和打包应用。'}
      </p>

      <div className={styles.steps}>
        <div className={[styles.step, step >= 1 ? styles.stepActive : ''].join(' ')}>
          <span className={styles.stepNum}>1</span>
          <div className={styles.stepBody}>
            <strong>下载安装包</strong>
            <small>Windows 64位，约 11 MB</small>
          </div>
          <a
            className={styles.stepBtn}
            href={DOWNLOAD_URL}
            download="elon-pc-node.exe"
            onClick={() => setStep(2)}
          >
            下载
          </a>
        </div>

        <div className={[styles.step, step >= 2 ? styles.stepActive : styles.stepDimmed].join(' ')}>
          <span className={styles.stepNum}>2</span>
          <div className={styles.stepBody}>
            <strong>运行安装包</strong>
            <small>双击 elon-pc-node.exe → 按提示完成安装</small>
          </div>
          <button className={styles.stepBtn} type="button" onClick={() => setStep(3)}>
            已安装
          </button>
        </div>

        <div className={[styles.step, step >= 3 ? styles.stepActive : styles.stepDimmed].join(' ')}>
          <span className={styles.stepNum}>3</span>
          <div className={styles.stepBody}>
            <strong>等待自动连接</strong>
            <small>
              {status === 'online'
                ? '✓ 节点已连接！'
                : '安装后节点会自动连接，此页面会实时更新…'}
            </small>
          </div>
          <span className={styles.stepStatus}>
            {status === 'online' ? '✓ 已就绪' : '等待中…'}
          </span>
        </div>
      </div>

      {status === 'online' && (
        <div className={styles.readyBar}>
          <span>✓ 节点已连接：{node?.display_name ?? node?.device_name}</span>
          <button className={styles.readyBtn} type="button" onClick={openDevChannel}>
            进入本机开发频道（执行命令行）
          </button>
        </div>
      )}
    </div>
  )
}
