import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { ArrowRightLeft, CircleDollarSign, Cpu, HardDrive, Server, Users } from 'lucide-react'
import {
  fetchMarketNodes,
  fetchNodeBalance,
  fetchNodeUsage,
  nodeCanAcceptProject,
  nodeId,
  nodeName,
} from './nodeHelpers'
import { useAuthStore } from '../../store/auth'
import type { NodeBalanceResponse, NodeComputeRun, NodeSummary, NodeUsageResponse } from './types'
import styles from './NodeMarketPanel.module.css'

interface NodeMarketPanelProps {
  myNodes: NodeSummary[]
  onOpenMyNode: (nodeId: string) => void
}

export default function NodeMarketPanel({ myNodes, onOpenMyNode }: NodeMarketPanelProps) {
  const user = useAuthStore((s) => s.user)
  const [nodes, setNodes] = useState<NodeSummary[]>([])
  const [balance, setBalance] = useState<NodeBalanceResponse | null>(null)
  const [usage, setUsage] = useState<NodeUsageResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [refreshTick, setRefreshTick] = useState(0)

  useEffect(() => {
    let alive = true
    setLoading(true)
    setError('')
    Promise.all([fetchMarketNodes(), fetchNodeBalance(), fetchNodeUsage()])
      .then(([marketNodes, nodeBalance, nodeUsage]) => {
        if (!alive) return
        setNodes(marketNodes)
        setBalance(nodeBalance)
        setUsage(nodeUsage)
      })
      .catch((err) => {
        if (alive) setError((err as Error).message || '节点市场加载失败')
      })
      .finally(() => {
        if (alive) setLoading(false)
      })
    return () => { alive = false }
  }, [refreshTick])

  const myNodeIds = useMemo(() => new Set(myNodes.map(nodeId).filter(Boolean)), [myNodes])
  const visibleMarketNodes = useMemo(
    () => nodes.filter((node) => !isOwnNode(node, user?.id, myNodeIds)),
    [nodes, user?.id, myNodeIds],
  )
  const borrowableNodes = visibleMarketNodes.filter((node) => node.online && nodeCanAcceptProject(node))
  const publicOwnNodes = nodes.filter((node) => isOwnNode(node, user?.id, myNodeIds))
  const nodeNames = useMemo(() => {
    const map = new Map<string, string>()
    for (const node of [...nodes, ...myNodes]) {
      const id = nodeId(node)
      if (id) map.set(id, nodeName(node))
    }
    return map
  }, [nodes, myNodes])

  return (
    <div className={styles.marketPage}>
      <div className={styles.hero}>
        <div>
          <div className={styles.kicker}>节点市场</div>
          <h2>共享算力网络</h2>
          <p>在线节点、可用开发环境、使用账本和收益结算在同一处汇总。</p>
        </div>
        <button className={styles.refreshBtn} type="button" onClick={() => setRefreshTick((value) => value + 1)}>
          {loading ? '同步中' : '刷新'}
        </button>
      </div>

      {error && <div className={styles.error}>{error}</div>}
      <div className={styles.statsGrid}>
        <StatCard icon={<Server size={16} />} label="市场在线" value={visibleMarketNodes.length} />
        <StatCard icon={<Cpu size={16} />} label="可接项目" value={borrowableNodes.length} />
        <StatCard icon={<HardDrive size={16} />} label="我的在线节点" value={publicOwnNodes.length} />
        <StatCard
          icon={<CircleDollarSign size={16} />}
          label="可提现收益"
          value={formatFen(balance?.balance_fen)}
        />
      </div>

      <section className={styles.panel}>
        <div className={styles.panelHead}>
          <div>
            <span>Discovery</span>
            <h3>可用市场节点</h3>
          </div>
          <strong>{loading ? '同步中' : `${borrowableNodes.length}/${visibleMarketNodes.length}`}</strong>
        </div>
        {visibleMarketNodes.length > 0 ? (
          <div className={styles.nodeGrid}>
            {visibleMarketNodes.map((node) => (
              <MarketNodeCard key={nodeId(node)} node={node} />
            ))}
          </div>
        ) : (
          <div className={styles.empty}>当前没有其他用户在线节点。</div>
        )}
      </section>

      <section className={styles.panel}>
        <div className={styles.panelHead}>
          <div>
            <span>Settlement</span>
            <h3>收益结算</h3>
          </div>
          <strong>{formatShare(balance?.provider_revenue_share_x1000)}</strong>
        </div>
        <div className={styles.moneyGrid}>
          <MoneyCell label="累计收益" value={formatFen(balance?.lifetime_earned_fen)} />
          <MoneyCell label="冻结提现" value={formatFen(balance?.pending_payout_fen)} />
          <MoneyCell label="最低提现" value={formatFen(balance?.payout_min_fen)} />
        </div>
      </section>

      <div className={styles.ledgerGrid}>
        <UsagePanel
          title="我使用的节点"
          icon={<ArrowRightLeft size={16} />}
          runs={usage?.consuming ?? []}
          nodeNames={nodeNames}
          empty="暂无共享使用记录。"
        />
        <UsagePanel
          title="我的节点被使用"
          icon={<Users size={16} />}
          runs={usage?.providing ?? []}
          nodeNames={nodeNames}
          empty="暂无提供记录。"
          providerView
        />
      </div>

      {myNodes.length > 0 && (
        <section className={styles.panel}>
          <div className={styles.panelHead}>
            <div>
              <span>Owned</span>
              <h3>我的节点</h3>
            </div>
          </div>
          <div className={styles.ownedList}>
            {myNodes.map((node) => {
              const id = nodeId(node)
              return (
                <button key={id} type="button" onClick={() => onOpenMyNode(id)}>
                  <strong>{nodeName(node)}</strong>
                  <span>{node.online ? '在线' : '离线'} · {node.capacity_label ?? node.short_id ?? id}</span>
                </button>
              )
            })}
          </div>
        </section>
      )}
    </div>
  )
}

function MarketNodeCard({ node }: { node: NodeSummary }) {
  const capabilities = [
    node.cli_project_ready ? 'CLI 项目' : '',
    node.route_a_ready ? '本机 AI' : '',
    node.api_runtime_ready ? 'API Key' : '',
    node.server_runtime_ready ? '服务器模型' : '',
    node.public_dev_enabled ? '公开开发' : '',
  ].filter(Boolean)
  return (
    <article className={styles.nodeCard}>
      <div className={styles.cardHead}>
        <div>
          <h4>{nodeName(node)}</h4>
          <small>{node.short_id ?? nodeId(node)}</small>
        </div>
        <span className={node.online ? styles.online : styles.offline}>
          {node.online ? '在线' : '离线'}
        </span>
      </div>
      <p>{node.hardware_summary || hardwareLine(node)}</p>
      <div className={styles.metaGrid}>
        <div><span>容量</span><strong>{node.capacity_label ?? '未上报'}</strong></div>
        <div><span>项目</span><strong>{capacityLine(node)}</strong></div>
        <div><span>硬盘</span><strong>{node.storage_repo_url_configured ? '跨 PC' : node.storage_ready ? '本机' : '未配置'}</strong></div>
        <div><span>共享</span><strong>{publicDevHandshakeText(node)}</strong></div>
      </div>
      <div className={styles.pills}>
        {(capabilities.length ? capabilities : ['待检测']).map((item) => <span key={item}>{item}</span>)}
      </div>
    </article>
  )
}

function UsagePanel({
  title,
  icon,
  runs,
  nodeNames,
  empty,
  providerView = false,
}: {
  title: string
  icon: ReactNode
  runs: NodeComputeRun[]
  nodeNames: Map<string, string>
  empty: string
  providerView?: boolean
}) {
  return (
    <section className={styles.panel}>
      <div className={styles.panelHead}>
        <div className={styles.titleWithIcon}>{icon}<h3>{title}</h3></div>
        <strong>{runs.length}</strong>
      </div>
      {runs.length ? (
        <div className={styles.runList}>
          {runs.slice(0, 8).map((run) => (
            <div key={run.id ?? run.compute_call_id} className={styles.runRow}>
              <div>
                <strong>{nodeNames.get(String(run.node_id ?? '')) ?? shortId(run.node_id)}</strong>
                <span>{providerView ? shortId(run.consumer_user_id) : shortId(run.provider_user_id)} · {formatDate(run.started_at)}</span>
              </div>
              <div>
                <strong>{formatTokens(run)}</strong>
                <span className={styles[runTone(run.status)]}>{run.status ?? 'unknown'}</span>
              </div>
              <div>
                <strong>{formatFen(providerView ? run.provider_earned_fen : run.billed_cost_rmb_fen)}</strong>
                <span>{run.settlement_status ?? run.usage_mode ?? ''}</span>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className={styles.empty}>{empty}</div>
      )}
    </section>
  )
}

function StatCard({ icon, label, value }: { icon: ReactNode; label: string; value: ReactNode }) {
  return (
    <div className={styles.statCard}>
      <span>{icon}</span>
      <div>
        <strong>{value}</strong>
        <small>{label}</small>
      </div>
    </div>
  )
}

function MoneyCell({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><strong>{value}</strong></div>
}

function publicDevHandshakeText(node: NodeSummary) {
  if (node.public_dev_handshake_ready) return '握手就绪'
  const status = node.public_dev_handshake_status ?? ''
  const labels: Record<string, string> = {
    sharing_disabled: '未开放',
    offline: '离线',
    waiting_for_handshake: '等握手',
    version_reconnected_waiting_capabilities: '等能力',
    no_allowed_cli: 'CLI 不符',
    runtime_not_ready: '未就绪',
    ready: '握手就绪',
  }
  return labels[status] ?? (node.public_dev_enabled ? '待确认' : '未开放')
}

function isOwnNode(node: NodeSummary, userId: string | undefined, myNodeIds: Set<string>) {
  const id = nodeId(node)
  return (!!userId && node.owner_user_id === userId) || (!!id && myNodeIds.has(id))
}

function capacityLine(node: NodeSummary) {
  const count = Number(node.project_count ?? 0)
  const limit = Number(node.project_limit ?? 0)
  return limit > 0 ? `${count}/${limit}` : `${count}`
}

function hardwareLine(node: NodeSummary) {
  const hw = node.hardware ?? {}
  const gpu = hw.gpu_names?.filter(Boolean).join(' / ')
  if (gpu) return `GPU ${gpu}`
  if (hw.cpu_brand) return String(hw.cpu_brand)
  return '硬件未知'
}

function formatFen(value?: number | null) {
  const fen = Number(value ?? 0)
  if (!Number.isFinite(fen) || fen <= 0) return '¥0.00'
  return `¥${(fen / 100).toFixed(2)}`
}

function formatShare(value?: number) {
  const share = Number(value ?? 0)
  return share > 0 ? `分账 ${(share / 10).toFixed(0)}%` : '分账未配置'
}

function formatTokens(run: NodeComputeRun) {
  const tokens = Number(run.prompt_tokens ?? 0) + Number(run.completion_tokens ?? 0)
  if (!Number.isFinite(tokens) || tokens <= 0) return '0'
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}K`
  return `${Math.round(tokens)}`
}

function formatDate(value?: string | null) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return String(value).slice(0, 16)
  return date.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false })
}

function shortId(value?: string | null) {
  const text = String(value ?? '').trim()
  if (!text) return 'unknown'
  return text.length > 16 ? `...${text.slice(-12)}` : text
}

function runTone(status?: string) {
  if (status === 'settled' || status === 'settled_no_provider' || status === 'deduplicated') return 'good'
  if (status === 'started' || status === 'settlement_skipped' || status === 'released_no_usage') return 'warn'
  return 'bad'
}
