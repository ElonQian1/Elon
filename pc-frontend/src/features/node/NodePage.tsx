import { useState, useEffect, useRef, useCallback } from 'react'
import { nodeApi, probeLocalNode } from './localNodeApi'
import { fetchMyNodes, fetchNodeAgentVersion, nodeId, nodeName, nodeSummaryLine } from './nodeHelpers'
import { safeNodeAdminUrl } from '../../lib/utils'
import type { NodeSummary, LocalNodeStatus } from './types'
import styles from './NodePage.module.css'

const DOWNLOAD_URL = '/api/node-agent/download/windows-client'
const LAUNCH_URL = 'elon-node://open'

export default function NodePage() {
  const [nodes, setNodes] = useState<NodeSummary[]>([])
  const [selectedNodeId, setSelectedNodeId] = useState('')
  const adminUrl = safeNodeAdminUrl()

  useEffect(() => {
    fetchMyNodes().then(setNodes).catch(() => {})
  }, [])

  const selected = nodes.find((n) => nodeId(n) === selectedNodeId)

  return (
    <div className={styles.layout}>
      <aside className={styles.sidebar}>
        <div className={styles.sideSection}>本机</div>
        <button
          className={[styles.sideBtn, !selectedNodeId ? styles.sideActive : ''].join(' ')}
          onClick={() => setSelectedNodeId('')}
        >
          <span className={styles.sideIcon}>🖥️</span>
          <span className={styles.sideMeta}>
            <strong>分享算力</strong>
            <small>下载、启动和注册</small>
          </span>
        </button>

        <div className={styles.sideSection}>我的节点</div>
        {nodes.length === 0 && <p className={styles.sideEmpty}>暂无节点</p>}
        {nodes.map((n) => {
          const id = nodeId(n)
          return (
            <button
              key={id}
              className={[styles.sideBtn, id === selectedNodeId ? styles.sideActive : ''].join(' ')}
              onClick={() => setSelectedNodeId(id)}
            >
              <span className={styles.sideIcon}>{n.online ? '●' : '○'}</span>
              <span className={styles.sideMeta}>
                <strong>{nodeName(n)}</strong>
                <small>{nodeSummaryLine(n)}</small>
              </span>
            </button>
          )
        })}

        <div className={styles.sideSection}>状态</div>
        <div className={styles.sideNote}>
          {nodes.filter((n) => n.online).length}/{nodes.length} 台在线
        </div>
      </aside>

      <main className={styles.main}>
        {!selectedNodeId
          ? <LocalNodePanel adminUrl={adminUrl} />
          : selected
            ? <NodeDetailPanel node={selected} onBack={() => setSelectedNodeId('')} adminUrl={adminUrl} />
            : <p className={styles.notFound}>节点不存在</p>
        }
      </main>
    </div>
  )
}

/* ── 本机面板 ── */
function LocalNodePanel({ adminUrl }: { adminUrl: string }) {
  const [probeStatus, setProbeStatus] = useState<'checking' | 'online' | 'offline'>('checking')
  const [localStatus, setLocalStatus] = useState<LocalNodeStatus | null>(null)
  const [version, setVersion] = useState('')
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const doProbe = useCallback(async (quiet = false) => {
    if (!quiet) setProbeStatus('checking')
    try {
      const status = await probeLocalNode(adminUrl) as LocalNodeStatus
      setLocalStatus(status)
      setProbeStatus('online')
      if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null }
    } catch {
      setProbeStatus('offline')
    }
  }, [adminUrl])

  useEffect(() => {
    doProbe()
    fetchNodeAgentVersion()
      .then((d) => {
        const size = d.windowsClientFileSize
          ? ` · ${(d.windowsClientFileSize / 1024 / 1024).toFixed(1)} MB` : ''
        setVersion(`最新 Win 端：v${d.version ?? 'latest'}${size}`)
      })
      .catch(() => setVersion('Win 端下载包暂时无法读取版本。'))
    return () => { if (pollRef.current) clearInterval(pollRef.current) }
  }, [doProbe])

  function launchWinClient() {
    const iframe = document.createElement('iframe')
    iframe.style.display = 'none'
    iframe.src = LAUNCH_URL
    document.body.appendChild(iframe)
    setTimeout(() => iframe.remove(), 2000)
    if (!pollRef.current) {
      let attempts = 0
      pollRef.current = setInterval(() => {
        attempts++
        if (attempts > 24) { clearInterval(pollRef.current!); pollRef.current = null; return }
        doProbe(true)
      }, 3500)
    }
  }

  return (
    <div className={styles.localPage}>
      <div className={styles.hero}>
        <div>
          <div className={styles.kicker}>一龙 Win 端</div>
          <h2>分享这台电脑算力</h2>
          <p>首次使用需要下载并安装；安装后点击"启动 Win 端"，浏览器会拉起本机程序。</p>
        </div>
        <span className={[styles.chip, styles[probeStatus]].join(' ')}>
          {{ checking: '检测中', online: '已连接', offline: '未连接' }[probeStatus]}
        </span>
      </div>

      <div className={styles.actions}>
        <a className={[styles.btn, styles.primary].join(' ')} href={DOWNLOAD_URL} download>下载 Win 端</a>
        <button className={styles.btn} onClick={launchWinClient}>
          {probeStatus === 'online' ? '打开高级本机页' : '启动 Win 端'}
        </button>
        <button className={styles.btn} onClick={() => doProbe()}>重新检测</button>
      </div>

      {version && <p className={styles.versionLine}>{version}</p>}

      {probeStatus === 'online' && localStatus
        ? <NodeAdminPanel adminUrl={adminUrl} initialStatus={localStatus} />
        : probeStatus === 'offline' && (
          <div className={styles.setupCard}>
            <h3>还没有可用的本机节点</h3>
            <div className={styles.stepList}>
              <div><strong>1</strong><span>下载 Win 端压缩包并解压。</span></div>
              <div><strong>2</strong><span>双击「一龙PC节点.exe」，它会自动安装并注册网页一键唤起。</span></div>
              <div><strong>3</strong><span>安装后点击"启动 Win 端"，在本机页面登录并注册 PC 节点。</span></div>
            </div>
          </div>
        )}
    </div>
  )
}

/* ── 本机已连接时的管理面板（精简版） ── */
function NodeAdminPanel({ adminUrl, initialStatus }: { adminUrl: string; initialStatus: LocalNodeStatus }) {
  const [status, setStatus] = useState(initialStatus)
  const [result, setResult] = useState('')
  const [error, setError] = useState('')

  async function login() {
    setResult('绑定中…'); setError('')
    try {
      await nodeApi(adminUrl, '/api/login', { method: 'POST', body: JSON.stringify({ token: '' }) })
      setResult('本机节点已绑定当前账号。')
    } catch (err) { setError((err as Error).message) }
  }

  async function logout() {
    setResult('登出中…'); setError('')
    try {
      await nodeApi(adminUrl, '/api/logout', { method: 'POST' })
      setResult('本机节点已登出。')
      const s = await nodeApi<LocalNodeStatus>(adminUrl, '/api/status')
      setStatus(s)
    } catch (err) { setError((err as Error).message) }
  }

  return (
    <div className={styles.adminPanel}>
      <div className={styles.adminRow}>
        <div>
          <div className={styles.kicker}>本机节点</div>
          <h3>{status.device_name ?? '这台电脑'}</h3>
        </div>
        <span className={[styles.chip, status.connected ? styles.online : styles.checking].join(' ')}>
          {status.connected ? '云端在线' : '等待云端'}
        </span>
      </div>
      <div className={styles.kvGrid}>
        {[
          ['登录', status.logged_in ? '已登录' : '未登录'],
          ['节点 ID', status.agent_id ?? '登录后自动生成'],
          ['版本', status.version ?? '未知'],
        ].map(([k, v]) => (
          <div key={k}><span>{k}</span><strong>{v}</strong></div>
        ))}
      </div>
      <div className={styles.actions}>
        <button className={[styles.btn, styles.primary].join(' ')} onClick={login}>
          {status.logged_in ? '重新绑定当前账号' : '用当前账号注册节点'}
        </button>
        <button className={styles.btn} disabled={!status.logged_in} onClick={logout}>登出本机节点</button>
        <button className={styles.btn} onClick={() => window.open(adminUrl, '_blank', 'noopener')}>
          高级本机页
        </button>
      </div>
      {result && <p className={styles.resultOk}>{result}</p>}
      {error && <p className={styles.resultErr}>{error}</p>}
    </div>
  )
}

/* ── 远程节点详情 ── */
function NodeDetailPanel({ node, onBack, adminUrl: _adminUrl }: { node: NodeSummary; onBack: () => void; adminUrl: string }) {
  const hw = node.hardware ?? {}
  const runtime = node.dev_runtime ?? {}
  const warnings = [...(node.capacity_warnings ?? []), ...(runtime.issues ?? [])].filter(Boolean)
  const models = node.models ?? []

  const rows = [
    ['显示名称', nodeName(node)],
    ['节点 ID', nodeId(node) || '未知'],
    ['短 ID', node.short_id ?? '未知'],
    ['在线', node.online ? '是' : '否'],
    ['项目', `${node.project_count ?? 0}/${node.project_limit ?? '?'}`],
    ['系统', String(hw.os ?? '未知')],
    ['CPU', hw.cpu_brand ? `${hw.cpu_brand} · ${hw.cpu_cores ?? '?'} 核` : '未上报'],
    ['内存', hw.memory_total_bytes ? `${(hw.memory_total_bytes / 1024 ** 3).toFixed(1)} GB` : '未上报'],
    ['显卡', (hw.gpu_names ?? []).join('、') || '未上报'],
    ['工作区', runtime.workspace_root_path ?? '未配置'],
    ['Git', runtime.git_ready ? '可用' : '未就绪'],
    ['AI Agent', node.ai_cli_ready ? '就绪' : '未就绪'],
  ]

  return (
    <div className={styles.detailPage}>
      <div className={styles.hero}>
        <div>
          <div className={styles.kicker}>我的节点</div>
          <h2>{nodeName(node)}</h2>
          <p className={styles.nodeIdText}>{nodeId(node)}</p>
        </div>
        <span className={[styles.chip, node.online ? styles.online : styles.offline].join(' ')}>
          {node.online ? '在线' : '离线'}
        </span>
      </div>

      {warnings.length > 0 && (
        <div className={styles.warnings}>
          {warnings.map((w, i) => <div key={i}>{w}</div>)}
        </div>
      )}

      <div className={styles.kvGrid}>
        {rows.map(([k, v]) => <div key={k}><span>{k}</span><strong>{v}</strong></div>)}
      </div>

      {models.length > 0 && (
        <div className={styles.section}>
          <h4>模型能力</h4>
          <div className={styles.pills}>
            {models.map((m, i) => (
              <span key={i} className={styles.pill}>
                {m.display_name ?? m.model_id} {m.provider ? `· ${m.provider}` : ''}
              </span>
            ))}
          </div>
        </div>
      )}

      <button className={styles.btn} onClick={onBack}>← 回到分享算力</button>
    </div>
  )
}
