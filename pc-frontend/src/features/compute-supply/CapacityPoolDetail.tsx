import { CalendarClock, Minus, PackageOpen, Plus } from 'lucide-react'
import { type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'
import {
  type MyComputeCapacityBucket,
  type MyComputeCapacityPool,
} from './computeSupplyApi'
import { type SupplyAction } from './BucketSupplyDialog'
import ActivationEvidencePanel from './ActivationEvidencePanel'
import CapacityLedgerPanel from './CapacityLedgerPanel'
import CapacityOfferPanel from './CapacityOfferPanel'
import styles from './CapacityPoolDetail.module.css'

interface Props {
  pool: MyComputeCapacityPool
  provider: MyComputeProvider | null
  buckets: MyComputeCapacityBucket[]
  selectedBucketId: string
  loadingBuckets: boolean
  onSelectBucket: (bucketId: string) => void
  onCreateBucket: () => void
  onChangeSupply: (bucket: MyComputeCapacityBucket, action: SupplyAction) => void
}

export default function CapacityPoolDetail({ pool, provider, buckets, selectedBucketId, loadingBuckets, onSelectBucket, onCreateBucket, onChangeSupply }: Props) {
  return <div className={styles.detail}>
    <header className={styles.poolHeader}><div><span>{provider?.display_name ?? 'Provider'}</span><h2>{pool.pool_id}</h2></div><span className={styles.status}>{statusLabel(pool.status)}</span></header>
    <div className={styles.facts}><div><span>区域</span><strong>{pool.region_or_data_zone}</strong></div><div><span>Epoch</span><strong>{pool.capacity_epoch}</strong></div><div><span>Revision</span><strong>{pool.pool_revision}</strong></div><div><span>创建时间</span><strong>{formatTime(pool.created_at)}</strong></div></div>
    <section className={styles.meters}><header><h3>计量策略</h3><span>{pool.meter_policies.length}</span></header>{pool.meter_policies.map((meter) => <div className={styles.meter} key={meter.meter}><strong>{meter.meter}</strong><span>{meter.meter_mode === 'consumable' ? '消耗型' : '复用型'}</span><span>量子 {meter.quantum_units}</span><code>{shortDigest(meter.policy_digest)}</code></div>)}</section>
    <section className={styles.buckets}>
      <header><div><h3>交付窗口与供给</h3><span>{buckets.length} 个 Bucket</span></div><button type="button" onClick={onCreateBucket}><CalendarClock size={15} />登记窗口</button></header>
      {buckets.length === 0 && !loadingBuckets ? <div className={styles.bucketEmpty}><PackageOpen size={22} /><span>尚未登记交付窗口</span></div> : <div className={styles.bucketList}>{buckets.map((bucket) => {
        const binding = bucket.balance.binding
        const active = binding.bucket_id === selectedBucketId
        return <article key={binding.bucket_id} data-active={active} className={styles.bucket}>
          <button type="button" className={styles.bucketSelect} onClick={() => onSelectBucket(binding.bucket_id)}>
            <div className={styles.bucketIdentity}><strong>{shortId(binding.bucket_id)}</strong><span>{binding.meter} · {bucketStatusLabel(bucket.balance.status)}</span></div>
            <div className={styles.window}><span>{formatTime(bucket.starts_at_utc)}</span><small>至</small><span>{formatTime(bucket.ends_at_utc)}</span></div>
            <div className={styles.balance}><span><small>可用</small><strong>{bucket.balance.available_units}</strong></span><span><small>占用</small><strong>{bucket.balance.held_units + bucket.balance.active_units}</strong></span><span><small>已用</small><strong>{bucket.balance.consumed_units}</strong></span></div>
          </button>
          <div className={styles.bucketActions}>
            <button type="button" onClick={() => onChangeSupply(bucket, 'add')} disabled={bucket.balance.status !== 'open'} title="追加供给"><Plus size={14} /><span>追加</span></button>
            <button type="button" onClick={() => onChangeSupply(bucket, 'withdraw')} disabled={bucket.balance.status !== 'open' || bucket.balance.available_units <= 0} title="撤出可用供给"><Minus size={14} /><span>撤出</span></button>
          </div>
        </article>
      })}</div>}
    </section>
    <CapacityLedgerPanel key={`${pool.provider_id}:${pool.pool_id}`} providerId={pool.provider_id} poolId={pool.pool_id} refreshKey={buckets.map((bucket) => `${bucket.balance.binding.bucket_id}:${bucket.balance.balance_revision}`).join('|')} />
    <ActivationEvidencePanel key={`activation:${pool.provider_id}:${pool.pool_id}`} providerId={pool.provider_id} poolId={pool.pool_id} poolStatus={pool.status} />
    <CapacityOfferPanel key={`offers:${pool.provider_id}:${pool.pool_id}`} provider={provider} pool={pool} buckets={buckets} />
    <section className={styles.digests}><h3>合同摘要</h3><div><span>Pool</span><code>{pool.pool_digest}</code></div><div><span>Scope</span><code>{pool.resource_scope_digest}</code></div><div><span>Profile</span><code>{pool.resource_profile_digest}</code></div></section>
  </div>
}

function statusLabel(value: string) { return ({ registering: '登记中', active: '已激活', draining: '排空中', retired: '已退场', quarantined: '已隔离' } as Record<string, string>)[value] ?? value }
function bucketStatusLabel(value: string) { return ({ open: '开放', closed: '已关闭', retired: '已退场' } as Record<string, string>)[value] ?? value }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function shortDigest(value: string) { return value.length <= 22 ? value : `${value.slice(0, 10)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
