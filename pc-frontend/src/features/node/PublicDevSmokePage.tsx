import { useMemo, useState } from 'react'
import { ArrowRightLeft, CheckCircle2, PlayCircle, RefreshCw, ShieldCheck, XCircle } from 'lucide-react'
import styles from './PublicDevSmokePage.module.css'

const TOKEN_KEY = 'elon.publicDevSmoke.adminToken'

interface SmokeResponse {
  ok: boolean
  execute: boolean
  cli_name: string
  generated_at: string
  directions: SmokeDirection[]
}

interface SmokeDirection {
  label: string
  status: string
  consumer: SmokeUser
  provider: SmokeUser
  node: SmokeNode
  preflight: {
    authorized: boolean
    ready: boolean
    cli_allowed_by_share: boolean
    cli_reported_by_node: boolean
    route: string
    notes: string[]
  }
  result?: {
    outcome: string
    done_message?: string | null
    model_used?: string | null
    done_node_id?: string | null
    event_count: number
    event_preview: Array<{ event_type: string; text: string }>
    compute_run?: SmokeComputeRun | null
  } | null
  error?: string | null
}

interface SmokeUser {
  id: string
  account: string
  nickname?: string | null
}

interface SmokeNode {
  node_id: string
  display_name: string
  short_id: string
  public_dev_handshake_ready: boolean
  public_dev_handshake_status: string
  online: boolean
  cli_connected: boolean
  allowed_clis: string[]
  last_handshake_at?: string | null
  agent_version?: string | null
}

interface SmokeComputeRun {
  compute_call_id: string
  status: string
  prompt_tokens: number
  completion_tokens: number
  billed_cost_rmb_fen: number
  provider_earned_fen: number
  settlement_status?: string | null
  duration_ms?: number | null
}

interface HandshakeResponse {
  ok: boolean
  public_dev_handshake?: {
    generated_at: string
    summary: {
      total_nodes: number
      public_dev_enabled: number
      online_public_dev: number
      ready_public_dev: number
      pending_online_public_dev: number
      offline_public_dev: number
    }
    nodes: HandshakeNode[]
  }
}

interface HandshakeNode {
  node_id: string
  owner_user_id: string
  owner_account?: string | null
  owner_nickname?: string | null
  display_name: string
  short_id: string
  public_dev_enabled: boolean
  public_dev_handshake_ready: boolean
  public_dev_handshake_status: string
  online: boolean
  cli_connected: boolean
  allowed_clis: string[]
  last_handshake_allowed_clis: string[]
  agent_version?: string | null
}

export default function PublicDevSmokePage() {
  const [adminToken, setAdminToken] = useState(() => sessionStorage.getItem(TOKEN_KEY) ?? '')
  const [leftOwner, setLeftOwner] = useState('钱一龙')
  const [leftNode, setLeftNode] = useState('一龙4060')
  const [rightOwner, setRightOwner] = useState('夜云')
  const [rightNode, setRightNode] = useState('志伟4060')
  const [cliName, setCliName] = useState('codex')
  const [prompt, setPrompt] = useState('请只回复一行：public-dev-smoke-ok。不要改文件，不要运行命令。')
  const [busy, setBusy] = useState<'preflight' | 'execute' | ''>('')
  const [error, setError] = useState('')
  const [report, setReport] = useState<SmokeResponse | null>(null)
  const [handshakeNodes, setHandshakeNodes] = useState<HandshakeNode[]>([])
  const [handshakeGeneratedAt, setHandshakeGeneratedAt] = useState('')
  const [handshakeLoading, setHandshakeLoading] = useState(false)
  const [handshakeError, setHandshakeError] = useState('')

  const summary = useMemo(() => {
    if (!report) return '等待检测'
    if (report.ok && report.execute) return '双向实测通过'
    if (report.ok) return '双向预检通过'
    return '存在阻塞'
  }, [report])

  const publicNodes = useMemo(
    () => handshakeNodes.filter((node) => node.public_dev_enabled),
    [handshakeNodes],
  )
  const readyNodes = useMemo(
    () => publicNodes.filter((node) => node.public_dev_handshake_ready && node.online && node.cli_connected),
    [publicNodes],
  )
  const selectedLeftKey = selectedNodeKey(handshakeNodes, leftOwner, leftNode)
  const selectedRightKey = selectedNodeKey(handshakeNodes, rightOwner, rightNode)

  async function loadHandshake() {
    const token = adminToken.trim()
    if (!token) {
      setHandshakeError('需要 ADMIN_TOKEN 才能同步节点握手')
      return
    }
    sessionStorage.setItem(TOKEN_KEY, token)
    setHandshakeLoading(true)
    setHandshakeError('')
    try {
      const response = await fetch('/api/admin/nodes/public-dev-handshake', {
        headers: { Authorization: `Bearer ${token}` },
      })
      const data = await response.json().catch(() => ({}))
      if (!response.ok) throw new Error(data.error || `HTTP ${response.status}`)
      const payload = data as HandshakeResponse
      setHandshakeNodes(payload.public_dev_handshake?.nodes ?? [])
      setHandshakeGeneratedAt(payload.public_dev_handshake?.generated_at ?? '')
    } catch (err) {
      setHandshakeError((err as Error).message || '同步节点握手失败')
    } finally {
      setHandshakeLoading(false)
    }
  }

  function selectNode(side: 'left' | 'right', key: string) {
    const node = handshakeNodes.find((item) => nodeKey(item) === key)
    if (!node) return
    const owner = node.owner_user_id || node.owner_account || node.owner_nickname || ''
    const nodeMatch = node.node_id || node.display_name || node.short_id
    if (side === 'left') {
      setLeftOwner(owner)
      setLeftNode(nodeMatch)
    } else {
      setRightOwner(owner)
      setRightNode(nodeMatch)
    }
  }

  async function runSmoke(execute: boolean) {
    const token = adminToken.trim()
    if (!token) {
      setError('需要 ADMIN_TOKEN')
      return
    }
    sessionStorage.setItem(TOKEN_KEY, token)
    setBusy(execute ? 'execute' : 'preflight')
    setError('')
    try {
      const response = await fetch('/api/admin/nodes/public-dev-mutual-smoke', {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          execute,
          left_owner: leftOwner,
          left_node: leftNode,
          right_owner: rightOwner,
          right_node: rightNode,
          cli_name: cliName,
          prompt,
        }),
      })
      const data = await response.json().catch(() => ({}))
      if (!response.ok) throw new Error(data.error || `HTTP ${response.status}`)
      setReport(data as SmokeResponse)
    } catch (err) {
      setError((err as Error).message || '互测失败')
    } finally {
      setBusy('')
    }
  }

  return (
    <div className={styles.page} data-smoke-status={report?.ok ? 'ok' : error ? 'error' : 'idle'}>
      <header className={styles.header}>
        <div>
          <span className={styles.kicker}>Public Dev Smoke</span>
          <h1>公开开发节点互用实测</h1>
        </div>
        <div className={[styles.summary, report?.ok ? styles.good : error ? styles.bad : ''].join(' ')}>
          {report?.ok ? <CheckCircle2 size={18} /> : error ? <XCircle size={18} /> : <ShieldCheck size={18} />}
          <strong>{summary}</strong>
        </div>
      </header>

      <section className={styles.controls}>
        <label>
          <span>ADMIN_TOKEN</span>
          <input
            value={adminToken}
            onChange={(event) => setAdminToken(event.target.value)}
            type="password"
            autoComplete="off"
            placeholder="Bearer token"
          />
        </label>
        <div className={styles.nodePicker}>
          <div className={styles.nodePickerHeader}>
            <div>
              <strong>公开节点选择</strong>
              <span>
                {handshakeNodes.length
                  ? `${readyNodes.length}/${publicNodes.length} 个公开节点已就绪`
                  : '先同步握手，再选择任意两台公开节点'}
              </span>
            </div>
            <button type="button" onClick={() => loadHandshake()} disabled={Boolean(busy) || handshakeLoading}>
              <RefreshCw size={15} />
              {handshakeLoading ? '同步中' : '同步节点'}
            </button>
          </div>
          {handshakeError && <p className={styles.miniError}>{handshakeError}</p>}
          <div className={styles.selectorGrid}>
            <label>
              <span>左侧节点</span>
              <select value={selectedLeftKey} onChange={(event) => selectNode('left', event.target.value)}>
                <option value="">手动输入或选择节点</option>
                {publicNodes.map((node) => (
                  <option key={nodeKey(node)} value={nodeKey(node)}>
                    {nodeOptionLabel(node)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>右侧节点</span>
              <select value={selectedRightKey} onChange={(event) => selectNode('right', event.target.value)}>
                <option value="">手动输入或选择节点</option>
                {publicNodes.map((node) => (
                  <option key={nodeKey(node)} value={nodeKey(node)}>
                    {nodeOptionLabel(node)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {handshakeGeneratedAt && (
            <p className={styles.hint}>握手快照 {new Date(handshakeGeneratedAt).toLocaleString()}</p>
          )}
        </div>
        <div className={styles.pairGrid}>
          <label><span>左侧账号</span><input value={leftOwner} onChange={(event) => setLeftOwner(event.target.value)} /></label>
          <label><span>左侧节点</span><input value={leftNode} onChange={(event) => setLeftNode(event.target.value)} /></label>
          <label><span>右侧账号</span><input value={rightOwner} onChange={(event) => setRightOwner(event.target.value)} /></label>
          <label><span>右侧节点</span><input value={rightNode} onChange={(event) => setRightNode(event.target.value)} /></label>
        </div>
        <label>
          <span>CLI</span>
          <input value={cliName} onChange={(event) => setCliName(event.target.value)} />
        </label>
        <label>
          <span>Prompt</span>
          <textarea value={prompt} onChange={(event) => setPrompt(event.target.value)} rows={3} />
        </label>
        <div className={styles.actions}>
          <button type="button" onClick={() => runSmoke(false)} disabled={Boolean(busy)}>
            <RefreshCw size={16} />
            {busy === 'preflight' ? '预检中' : '预检'}
          </button>
          <button className={styles.primary} type="button" onClick={() => runSmoke(true)} disabled={Boolean(busy)}>
            <PlayCircle size={16} />
            {busy === 'execute' ? '执行中' : '执行双向互测'}
          </button>
        </div>
      </section>

      {error && <div className={styles.error}>{error}</div>}

      {report && (
        <section className={styles.results}>
          <div className={styles.resultHead}>
            <ArrowRightLeft size={17} />
            <strong>{report.execute ? '执行结果' : '预检结果'}</strong>
            <span>{new Date(report.generated_at).toLocaleString()}</span>
          </div>
          <div className={styles.directionGrid}>
            {report.directions.map((direction) => (
              <DirectionCard key={direction.label} direction={direction} />
            ))}
          </div>
        </section>
      )}
    </div>
  )
}

function DirectionCard({ direction }: { direction: SmokeDirection }) {
  const passed = direction.status === 'passed' || direction.status === 'ready'
  const run = direction.result?.compute_run
  return (
    <article className={[styles.direction, passed ? styles.goodBorder : styles.badBorder].join(' ')}>
      <div className={styles.directionTitle}>
        <div>
          <span>{direction.status}</span>
          <h2>{direction.label}</h2>
        </div>
        {passed ? <CheckCircle2 size={20} /> : <XCircle size={20} />}
      </div>
      <div className={styles.routeLine}>
        <strong>{displayUser(direction.consumer)}</strong>
        <span>使用</span>
        <strong>{direction.node.display_name}</strong>
        <span>{displayUser(direction.provider)}</span>
      </div>
      <div className={styles.flags}>
        <Flag ok={direction.preflight.authorized} label="授权" />
        <Flag ok={direction.preflight.ready} label="就绪" />
        <Flag ok={direction.preflight.cli_reported_by_node} label="CLI" />
        <Flag ok={direction.node.online} label="在线" />
      </div>
      {direction.error && <p className={styles.directionError}>{direction.error}</p>}
      {direction.preflight.notes.length > 0 && (
        <ul className={styles.notes}>
          {direction.preflight.notes.map((note) => <li key={note}>{note}</li>)}
        </ul>
      )}
      {direction.result?.done_message && (
        <p className={styles.reply}>{direction.result.done_message}</p>
      )}
      {run && (
        <div className={styles.ledger}>
          <span>账本</span>
          <strong>{run.status}</strong>
          <span>{formatTokens(run)} tokens</span>
          <span>收益 {formatFen(run.provider_earned_fen)}</span>
        </div>
      )}
      {direction.result?.event_preview?.length ? (
        <div className={styles.events}>
          {direction.result.event_preview.slice(0, 4).map((event, index) => (
            <div key={`${event.event_type}-${index}`}>
              <span>{event.event_type}</span>
              <p>{event.text || '空事件'}</p>
            </div>
          ))}
        </div>
      ) : null}
    </article>
  )
}

function Flag({ ok, label }: { ok: boolean; label: string }) {
  return <span className={ok ? styles.flagOk : styles.flagBad}>{label}</span>
}

function displayUser(user: SmokeUser) {
  return user.nickname || user.account || user.id
}

function nodeKey(node: HandshakeNode) {
  return `${node.owner_user_id}::${node.node_id}`
}

function selectedNodeKey(nodes: HandshakeNode[], owner: string, nodeMatch: string) {
  const ownerText = owner.trim()
  const nodeText = nodeMatch.trim()
  const found = nodes.find((node) => node.owner_user_id === ownerText && node.node_id === nodeText)
  return found ? nodeKey(found) : ''
}

function nodeOptionLabel(node: HandshakeNode) {
  const owner = node.owner_nickname || node.owner_account || shortText(node.owner_user_id)
  const status = node.public_dev_handshake_ready ? '就绪' : handshakeStatusText(node.public_dev_handshake_status)
  const cli = [...(node.allowed_clis ?? []), ...(node.last_handshake_allowed_clis ?? [])]
    .filter(Boolean)
    .slice(0, 3)
    .join('/')
  return `${owner} · ${node.display_name} · ${status}${cli ? ` · ${cli}` : ''}`
}

function handshakeStatusText(status: string) {
  const labels: Record<string, string> = {
    sharing_disabled: '未开放',
    offline: '离线',
    waiting_for_handshake: '等握手',
    version_reconnected_waiting_capabilities: '等能力',
    no_allowed_cli: 'CLI 不符',
    runtime_not_ready: '未就绪',
    ready: '就绪',
  }
  return labels[status] ?? (status || '未知')
}

function shortText(value?: string | null) {
  const text = String(value ?? '').trim()
  if (!text) return 'unknown'
  return text.length > 14 ? `${text.slice(0, 6)}...${text.slice(-4)}` : text
}

function formatTokens(run: SmokeComputeRun) {
  return Math.max(0, Number(run.prompt_tokens ?? 0) + Number(run.completion_tokens ?? 0))
}

function formatFen(fen: number) {
  const value = Number(fen ?? 0)
  return `￥${(value / 100).toFixed(4)}`
}
