import { useCallback, useEffect, useMemo, useState } from 'react'
import { DatabaseZap, Gauge, Plus, RefreshCw, ShieldCheck, TriangleAlert } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import {
  computeSupplyApi,
  type ChangeCapacitySupplyBody,
  type CreateMyComputeCapacityBucketBody,
  type CreateMyComputeCapacityPoolBody,
  type MyComputeCapacityBucket,
  type MyComputeCapacityPool,
} from './computeSupplyApi'
import { type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'
import BucketSupplyDialog, { type SupplyAction } from './BucketSupplyDialog'
import CapacityPoolDetail from './CapacityPoolDetail'
import CreateCapacityBucketDialog from './CreateCapacityBucketDialog'
import CreateCapacityPoolDialog from './CreateCapacityPoolDialog'
import styles from './ComputeSupplyPage.module.css'

export default function ComputeSupplyPage() {
  const navigate = useNavigate()
  const [providers, setProviders] = useState<MyComputeProvider[]>([])
  const [providerId, setProviderId] = useState('')
  const [pools, setPools] = useState<MyComputeCapacityPool[]>([])
  const [poolId, setPoolId] = useState('')
  const [buckets, setBuckets] = useState<MyComputeCapacityBucket[]>([])
  const [bucketId, setBucketId] = useState('')
  const [loading, setLoading] = useState(false)
  const [loadingBuckets, setLoadingBuckets] = useState(false)
  const [creating, setCreating] = useState(false)
  const [poolDialogOpen, setPoolDialogOpen] = useState(false)
  const [bucketDialogOpen, setBucketDialogOpen] = useState(false)
  const [supplyDialog, setSupplyDialog] = useState<{ bucket: MyComputeCapacityBucket; action: SupplyAction } | null>(null)
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

  const loadBuckets = useCallback(async () => {
    if (!providerId || !poolId) { setBuckets([]); setBucketId(''); return }
    setLoadingBuckets(true)
    setError('')
    try {
      const next = await computeSupplyApi.buckets(providerId, poolId)
      setBuckets(next)
      setBucketId((current) => next.some((item) => item.balance.binding.bucket_id === current) ? current : next[0]?.balance.binding.bucket_id ?? '')
    } catch (reason) {
      setError(messageOf(reason, 'CapacityBucket 读取失败'))
    } finally {
      setLoadingBuckets(false)
    }
  }, [poolId, providerId])

  useEffect(() => { void loadProviders() }, [loadProviders])
  useEffect(() => { void loadPools() }, [loadPools])
  useEffect(() => { void loadBuckets() }, [loadBuckets])

  async function createPool(body: CreateMyComputeCapacityPoolBody) {
    if (!providerId || creating) return
    setCreating(true)
    setError('')
    setNotice('')
    try {
      const pool = await computeSupplyApi.createPool(providerId, body)
      await loadPools()
      setPoolId(pool.pool_id)
      setPoolDialogOpen(false)
      setNotice(`CapacityPool ${shortId(pool.pool_id)} 已登记。`)
    } catch (reason) {
      setError(messageOf(reason, 'CapacityPool 登记失败'))
    } finally {
      setCreating(false)
    }
  }

  async function createBucket(body: CreateMyComputeCapacityBucketBody) {
    if (!providerId || !poolId || creating) return
    setCreating(true); setError(''); setNotice('')
    try {
      const bucket = await computeSupplyApi.createBucket(providerId, poolId, body)
      await loadBuckets()
      setBucketId(bucket.balance.binding.bucket_id)
      setBucketDialogOpen(false)
      setNotice(`交付窗口 ${shortId(bucket.balance.binding.delivery_window.window_id)} 已登记，当前供给为 0。`)
    } catch (reason) { setError(messageOf(reason, '交付窗口登记失败')) } finally { setCreating(false) }
  }

  async function changeSupply(action: SupplyAction, quantityUnits: number, idempotencyKey: string) {
    if (!providerId || !poolId || !supplyDialog || creating) return
    setCreating(true); setError(''); setNotice('')
    const body: ChangeCapacitySupplyBody = { idempotency_key: idempotencyKey, lines: [{ bucket_id: supplyDialog.bucket.balance.binding.bucket_id, quantity_units: quantityUnits }] }
    try {
      const receipt = action === 'add' ? await computeSupplyApi.addSupply(providerId, poolId, body) : await computeSupplyApi.withdrawSupply(providerId, poolId, body)
      await loadBuckets()
      setBucketId(supplyDialog.bucket.balance.binding.bucket_id)
      setSupplyDialog(null)
      setNotice(`${action === 'add' ? '供给已追加' : '可用供给已撤出'}，账本序号 ${receipt.ledger_sequence}${receipt.replayed ? '（幂等重放）' : ''}。`)
    } catch (reason) { setError(messageOf(reason, action === 'add' ? '供给追加失败' : '供给撤出失败')) } finally { setCreating(false) }
  }

  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <div><span className={styles.eyebrow}>容量市场合同</span><h1>算力供给</h1><p>管理 Provider 的资源池与计量边界。</p></div>
        <div className={styles.controls}>
          <label><span>Provider</span><select value={providerId} onChange={(event) => setProviderId(event.target.value)} disabled={loading || providers.length === 0}>{providers.length === 0 && <option value="">暂无 Provider</option>}{providers.map((provider) => <option key={provider.provider_id} value={provider.provider_id}>{provider.display_name}</option>)}</select></label>
          <button type="button" className={styles.iconButton} onClick={() => void loadPools()} disabled={loading || !providerId} aria-label="刷新" title="刷新"><RefreshCw size={16} className={loading ? styles.spinning : ''} /></button>
          <button type="button" className={styles.primaryButton} onClick={() => { setError(''); setPoolDialogOpen(true) }} disabled={!selectedProvider}><Plus size={16} />登记 Pool</button>
        </div>
      </header>
      {error && !poolDialogOpen && !bucketDialogOpen && !supplyDialog && <div className={styles.alert}><TriangleAlert size={15} />{error}</div>}
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
            {selectedPool ? <CapacityPoolDetail pool={selectedPool} provider={selectedProvider} buckets={buckets} selectedBucketId={bucketId} loadingBuckets={loadingBuckets} onSelectBucket={setBucketId} onCreateBucket={() => { setError(''); setBucketDialogOpen(true) }} onChangeSupply={(bucket, action) => { setError(''); setBucketId(bucket.balance.binding.bucket_id); setSupplyDialog({ bucket, action }) }} /> : <div className={styles.detailEmpty}><DatabaseZap size={24} /><h2>选择或登记 CapacityPool</h2></div>}
          </div>
        </section>
      )}

      {poolDialogOpen && selectedProvider && <CreateCapacityPoolDialog providerName={selectedProvider.display_name} defaultRegion={selectedProvider.home_region ?? ''} busy={creating} error={error} onClose={() => setPoolDialogOpen(false)} onSubmit={createPool} />}
      {bucketDialogOpen && selectedPool && <CreateCapacityBucketDialog pool={selectedPool} busy={creating} error={error} onClose={() => setBucketDialogOpen(false)} onSubmit={createBucket} />}
      {supplyDialog && <BucketSupplyDialog bucket={supplyDialog.bucket} initialAction={supplyDialog.action} busy={creating} error={error} onClose={() => setSupplyDialog(null)} onSubmit={changeSupply} />}
    </main>
  )
}
function statusLabel(value: string) { return ({ registering: '登记中', active: '已激活', draining: '排空中', retired: '已退场', quarantined: '已隔离' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
