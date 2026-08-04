import { useCallback, useEffect, useMemo, useState } from 'react'
import { DatabaseZap, Gauge, Plus, RefreshCw, ShieldCheck, TriangleAlert } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import {
  computeSupplyApi,
  type CreateMyComputeCapacityPoolBody,
  type MyComputeCapacityPool,
} from './computeSupplyApi'
import { type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'
import CreateCapacityPoolDialog from './CreateCapacityPoolDialog'
import styles from './ComputeSupplyPage.module.css'

export default function ComputeSupplyPage() {
  const navigate = useNavigate()
  const [providers, setProviders] = useState<MyComputeProvider[]>([])
  const [providerId, setProviderId] = useState('')
  const [pools, setPools] = useState<MyComputeCapacityPool[]>([])
  const [poolId, setPoolId] = useState('')
  const [loading, setLoading] = useState(false)
  const [creating, setCreating] = useState(false)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const selectedProvider = useMemo(() => providers.find((item) => item.provider_id === providerId) ?? null, [providerId, providers])
  const selectedPool = useMemo(() => pools.find((item) => item.pool_id === poolId) ?? null, [poolId, pools])

  const loadProviders = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      const next = await computeSupplyApi.providers()
      setProviders(next)
      setProviderId((current) => next.some((item) => item.provider_id === current) ? current : next[0]?.provider_id ?? '')
    } catch (reason) {
      setError(messageOf(reason, 'Provider 列表读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  const loadPools = useCallback(async () => {
    if (!providerId) { setPools([]); setPoolId(''); return }
    setLoading(true)
    setError('')
    try {
      const next = await computeSupplyApi.pools(providerId)
      setPools(next)
      setPoolId((current) => next.some((item) => item.pool_id === current) ? current : next[0]?.pool_id ?? '')
    } catch (reason) {
      setError(messageOf(reason, 'CapacityPool 读取失败'))
    } finally {
      setLoading(false)
    }
  }, [providerId])

  useEffect(() => { void loadProviders() }, [loadProviders])
  useEffect(() => { void loadPools() }, [loadPools])

  async function createPool(body: CreateMyComputeCapacityPoolBody) {
    if (!providerId || creating) return
    setCreating(true)
    setError('')
    setNotice('')
    try {
      const pool = await computeSupplyApi.createPool(providerId, body)
      await loadPools()
      setPoolId(pool.pool_id)
      setDialogOpen(false)
      setNotice(`CapacityPool ${shortId(pool.pool_id)} 已登记。`)
    } catch (reason) {
      setError(messageOf(reason, 'CapacityPool 登记失败'))
    } finally {
      setCreating(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <div><span className={styles.eyebrow}>容量市场合同</span><h1>算力供给</h1><p>管理 Provider 的资源池与计量边界。</p></div>
        <div className={styles.controls}>
          <label><span>Provider</span><select value={providerId} onChange={(event) => setProviderId(event.target.value)} disabled={loading || providers.length === 0}>{providers.length === 0 && <option value="">暂无 Provider</option>}{providers.map((provider) => <option key={provider.provider_id} value={provider.provider_id}>{provider.display_name}</option>)}</select></label>
          <button type="button" className={styles.iconButton} onClick={() => void loadPools()} disabled={loading || !providerId} aria-label="刷新" title="刷新"><RefreshCw size={16} className={loading ? styles.spinning : ''} /></button>
          <button type="button" className={styles.primaryButton} onClick={() => { setError(''); setDialogOpen(true) }} disabled={!selectedProvider}><Plus size={16} />登记 Pool</button>
        </div>
      </header>
      {error && !dialogOpen && <div className={styles.alert}><TriangleAlert size={15} />{error}</div>}
      {notice && <div className={styles.notice}><ShieldCheck size={15} />{notice}</div>}

      {providers.length === 0 && !loading ? (
        <section className={styles.empty}><Gauge size={25} /><h2>先登记一个 Provider</h2><button type="button" onClick={() => navigate('/my-compute-settlement')}>前往我的算力收益</button></section>
      ) : (
        <section className={styles.workbench}>
          <aside className={styles.poolList}>
            <header><div><strong>CapacityPool</strong><span>{pools.length} 个</span></div></header>
            {pools.map((pool) => <button type="button" key={pool.pool_id} data-active={pool.pool_id === poolId} onClick={() => setPoolId(pool.pool_id)}><DatabaseZap size={16} /><span><strong>{shortId(pool.pool_id)}</strong><small>{pool.region_or_data_zone} · {statusLabel(pool.status)}</small></span></button>)}
            {pools.length === 0 && !loading && <div className={styles.listEmpty}>当前 Provider 尚无 Pool</div>}
          </aside>
          <div className={styles.detail}>
            {selectedPool ? <PoolDetail pool={selectedPool} provider={selectedProvider} /> : <div className={styles.detailEmpty}><DatabaseZap size={24} /><h2>选择或登记 CapacityPool</h2></div>}
          </div>
        </section>
      )}

      {dialogOpen && selectedProvider && <CreateCapacityPoolDialog providerName={selectedProvider.display_name} defaultRegion={selectedProvider.home_region ?? ''} busy={creating} error={error} onClose={() => setDialogOpen(false)} onSubmit={createPool} />}
    </main>
  )
}

function PoolDetail({ pool, provider }: { pool: MyComputeCapacityPool; provider: MyComputeProvider | null }) {
  return <div className={styles.poolDetail}>
    <header><div><span>{provider?.display_name ?? 'Provider'}</span><h2>{pool.pool_id}</h2></div><span className={styles.status}>{statusLabel(pool.status)}</span></header>
    <div className={styles.facts}><div><span>区域</span><strong>{pool.region_or_data_zone}</strong></div><div><span>Epoch</span><strong>{pool.capacity_epoch}</strong></div><div><span>Revision</span><strong>{pool.pool_revision}</strong></div><div><span>创建时间</span><strong>{formatTime(pool.created_at)}</strong></div></div>
    <section className={styles.meters}><header><h3>计量策略</h3><span>{pool.meter_policies.length}</span></header>{pool.meter_policies.map((meter) => <div className={styles.meter} key={meter.meter}><strong>{meter.meter}</strong><span>{meter.meter_mode === 'consumable' ? '消耗型' : '复用型'}</span><span>量子 {meter.quantum_units}</span><code>{shortDigest(meter.policy_digest)}</code></div>)}</section>
    <section className={styles.digests}><h3>合同摘要</h3><div><span>Pool</span><code>{pool.pool_digest}</code></div><div><span>Scope</span><code>{pool.resource_scope_digest}</code></div><div><span>Profile</span><code>{pool.resource_profile_digest}</code></div></section>
  </div>
}

function statusLabel(value: string) { return ({ registering: '登记中', active: '已激活', draining: '排空中', retired: '已退场', quarantined: '已隔离' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function shortDigest(value: string) { return value.length <= 22 ? value : `${value.slice(0, 10)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
