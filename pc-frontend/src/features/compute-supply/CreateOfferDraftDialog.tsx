import { useMemo, useState, type FormEvent, type ReactNode } from 'react'
import { LoaderCircle, PackagePlus, X } from 'lucide-react'
import { type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'
import { type ComputeOfferDraftBody } from './computeOfferApi'
import { type MyComputeCapacityBucket, type MyComputeCapacityPool } from './computeSupplyApi'
import styles from './CreateOfferDraftDialog.module.css'

interface Props {
  provider: MyComputeProvider
  pool: MyComputeCapacityPool
  buckets: MyComputeCapacityBucket[]
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: ComputeOfferDraftBody) => Promise<void>
}

const SHA256 = /^[0-9a-f]{64}$/

export default function CreateOfferDraftDialog({ provider, pool, buckets, busy, error, onClose, onSubmit }: Props) {
  const openBuckets = useMemo(() => buckets.filter((item) => item.balance.status === 'open'), [buckets])
  const taskKinds = provider.capabilities?.task_kinds.length ? provider.capabilities.task_kinds : ['llm_chat']
  const acceleratorKinds = provider.capabilities?.accelerator_kinds.length ? provider.capabilities.accelerator_kinds : ['gpu']
  const dataClasses = provider.capabilities?.allowed_data_classes.length ? provider.capabilities.allowed_data_classes : ['public']
  const [requestKey] = useState(() => `offer-draft:${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`)
  const [skuId, setSkuId] = useState(`sku-${shortId(pool.pool_id)}-spot`)
  const [taskKind, setTaskKind] = useState(taskKinds[0])
  const [shapeBucket, setShapeBucket] = useState('standard')
  const [verificationTier, setVerificationTier] = useState('platform_verified')
  const [slaTier, setSlaTier] = useState('best_effort')
  const [windowClass, setWindowClass] = useState('scheduled')
  const [runtimeFamily, setRuntimeFamily] = useState('yilong-node')
  const [runtimeVersion, setRuntimeVersion] = useState('1')
  const [precision, setPrecision] = useState('fp16')
  const [runnerDigest, setRunnerDigest] = useState('')
  const [acceleratorKind, setAcceleratorKind] = useState(acceleratorKinds[0])
  const [acceleratorCount, setAcceleratorCount] = useState(1)
  const [vramGiB, setVramGiB] = useState(8)
  const [ramGiB, setRamGiB] = useState(16)
  const [maxConcurrency, setMaxConcurrency] = useState(1)
  const [maxRuntime, setMaxRuntime] = useState(3600)
  const [isPublic, setIsPublic] = useState(true)
  const [accounts, setAccounts] = useState('')
  const [projects, setProjects] = useState('')
  const [allowedDataClasses, setAllowedDataClasses] = useState(dataClasses.join('\n'))
  const [validFrom, setValidFrom] = useState(localValue(new Date(Date.now() + 15 * 60_000)))
  const [validUntil, setValidUntil] = useState(localValue(defaultValidUntil(openBuckets)))
  const [capacity, setCapacity] = useState(() => Object.fromEntries(openBuckets.map((item) => [item.balance.binding.bucket_id, item.balance.available_units])))
  const [providerPrices, setProviderPrices] = useState<Record<string, number>>({})
  const [consumerPrices, setConsumerPrices] = useState<Record<string, number>>({})
  const [confirmed, setConfirmed] = useState(false)
  const meters = useMemo(() => unique(openBuckets.map((item) => item.balance.binding.meter)), [openBuckets])
  const capacityRows = openBuckets.map((item) => ({ item, units: capacity[item.balance.binding.bucket_id] ?? 0 }))
  const validWindow = openBuckets.every((item) => new Date(item.starts_at_utc) >= new Date(validFrom))
    && openBuckets.every((item) => new Date(item.ends_at_utc) <= new Date(validUntil))
  const valid = Boolean(openBuckets.length && validWindow && new Date(validFrom).getTime() > Date.now()
    && new Date(validUntil) > new Date(validFrom) && skuId.trim() && taskKind && shapeBucket.trim()
    && verificationTier.trim() && slaTier.trim() && windowClass.trim() && runtimeFamily.trim() && runtimeVersion.trim()
    && precision.trim() && SHA256.test(runnerDigest.trim()) && acceleratorKind && acceleratorCount > 0
    && vramGiB > 0 && ramGiB > 0 && maxConcurrency > 0 && maxRuntime > 0
    && capacityRows.every(({ item, units }) => units > 0 && units <= item.balance.available_units && units % item.balance.binding.quantum_units === 0)
    && (isPublic || splitLines(accounts).length + splitLines(projects).length > 0)
    && splitLines(allowedDataClasses).length > 0 && splitLines(allowedDataClasses).every((value) => dataClasses.includes(value))
    && meters.every((meter) => (providerPrices[meter] ?? 0) >= 0 && (consumerPrices[meter] ?? 0) >= (providerPrices[meter] ?? 0))
    && confirmed && !busy)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      idempotency_key: requestKey,
      sku: { sku_id: skuId.trim(), task_kind: taskKind, context_or_shape_bucket: shapeBucket.trim(), verification_tier: verificationTier.trim(), sla_tier: slaTier.trim(), delivery_window_class: windowClass.trim() },
      model: null,
      runtime: { runtime_family: runtimeFamily.trim(), runtime_version: runtimeVersion.trim(), precision: precision.trim(), runner_digest: runnerDigest.trim(), plugin_id: null, plugin_version: null, plugin_digest: null },
      resource_profile: { accelerator_kind: acceleratorKind, accelerator_count: acceleratorCount, vram_bytes: gib(vramGiB), ram_bytes: gib(ramGiB) },
      capacity: capacityRows.map(({ item, units }) => ({ bucket_id: item.balance.binding.bucket_id, total_units: units, reservable_units: units })),
      execution_limits: { max_concurrent_attempts: maxConcurrency, max_attempt_runtime_seconds: maxRuntime },
      authorization: { public: isPublic, allowed_account_ids: isPublic ? [] : splitLines(accounts), allowed_project_ids: isPublic ? [] : splitLines(projects), allowed_data_classes: splitLines(allowedDataClasses) },
      price_terms: { pricing_mode: 'spot', currency: 'CNY', curve_id: null, curve_version: null, instrument_id: null, components: meters.map((meter) => ({ meter, unit_size: openBuckets.find((item) => item.balance.binding.meter === meter)!.balance.binding.quantum_units, consumer_unit_price_micros: consumerPrices[meter] ?? 0, provider_unit_price_micros: providerPrices[meter] ?? 0, max_units: capacityRows.filter(({ item }) => item.balance.binding.meter === meter).reduce((sum, row) => sum + row.units, 0) })), fee_rules: [] },
      valid_from: new Date(validFrom).toISOString(), valid_until: new Date(validUntil).toISOString(), confirm_create: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="create-offer-title">
    <header><div><PackagePlus size={18} /><h2 id="create-offer-title">创建算力 Offer 草稿</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
    <form onSubmit={(event) => void submit(event)}>
      {error && <div className={styles.error}>{error}</div>}
      <fieldset><legend>服务规格</legend><div className={styles.grid}><Field label="SKU ID"><input value={skuId} onChange={(event) => setSkuId(event.target.value)} required /></Field><Field label="任务类型"><select value={taskKind} onChange={(event) => setTaskKind(event.target.value)}>{taskKinds.map((value) => <option key={value}>{value}</option>)}</select></Field><Field label="形状档位"><input value={shapeBucket} onChange={(event) => setShapeBucket(event.target.value)} required /></Field><Field label="验证等级"><input value={verificationTier} onChange={(event) => setVerificationTier(event.target.value)} required /></Field><Field label="SLA 等级"><input value={slaTier} onChange={(event) => setSlaTier(event.target.value)} required /></Field><Field label="交付窗口类别"><input value={windowClass} onChange={(event) => setWindowClass(event.target.value)} required /></Field></div></fieldset>
      <fieldset><legend>运行环境</legend><div className={styles.grid}><Field label="运行时家族"><input value={runtimeFamily} onChange={(event) => setRuntimeFamily(event.target.value)} required /></Field><Field label="运行时版本"><input value={runtimeVersion} onChange={(event) => setRuntimeVersion(event.target.value)} required /></Field><Field label="精度"><input value={precision} onChange={(event) => setPrecision(event.target.value)} required /></Field><Field label="加速器"><select value={acceleratorKind} onChange={(event) => setAcceleratorKind(event.target.value)}>{acceleratorKinds.map((value) => <option key={value}>{value}</option>)}</select></Field><Field label="加速器数量"><NumberInput value={acceleratorCount} onChange={setAcceleratorCount} /></Field><Field label="显存 GiB"><NumberInput value={vramGiB} onChange={setVramGiB} /></Field><Field label="内存 GiB"><NumberInput value={ramGiB} onChange={setRamGiB} /></Field><Field label="最大并发"><NumberInput value={maxConcurrency} onChange={setMaxConcurrency} /></Field><Field label="最长运行秒数"><NumberInput value={maxRuntime} onChange={setMaxRuntime} /></Field><Field label="Runner SHA-256" wide><input value={runnerDigest} onChange={(event) => setRunnerDigest(event.target.value.toLowerCase())} maxLength={64} spellCheck={false} required /></Field></div></fieldset>
      <fieldset><legend>容量与价格（人民币微元/量子）</legend><div className={styles.capacity}>{capacityRows.map(({ item, units }) => <div key={item.balance.binding.bucket_id}><span><strong>{item.balance.binding.meter}</strong><small>{shortId(item.balance.binding.bucket_id)} · 可用 {item.balance.available_units} · 量子 {item.balance.binding.quantum_units}</small></span><input type="number" min={item.balance.binding.quantum_units} max={item.balance.available_units} step={item.balance.binding.quantum_units} value={units} onChange={(event) => setCapacity((current) => ({ ...current, [item.balance.binding.bucket_id]: Number(event.target.value) }))} /></div>)}</div><div className={styles.prices}>{meters.map((meter) => <div key={meter}><strong>{meter}</strong><Field label="Provider 单价"><NumberInput value={providerPrices[meter] ?? 0} min={0} onChange={(value) => setProviderPrices((current) => ({ ...current, [meter]: value }))} /></Field><Field label="消费者单价"><NumberInput value={consumerPrices[meter] ?? 0} min={providerPrices[meter] ?? 0} onChange={(value) => setConsumerPrices((current) => ({ ...current, [meter]: value }))} /></Field></div>)}</div></fieldset>
      <fieldset><legend>授权与有效期</legend><label className={styles.toggle}><input type="checkbox" checked={isPublic} onChange={(event) => setIsPublic(event.target.checked)} /><span>公开 Offer</span></label>{!isPublic && <div className={styles.grid}><Field label="允许账户（每行一个）"><textarea value={accounts} onChange={(event) => setAccounts(event.target.value)} rows={3} /></Field><Field label="允许项目（每行一个）"><textarea value={projects} onChange={(event) => setProjects(event.target.value)} rows={3} /></Field></div>}<div className={styles.grid}><Field label="允许数据等级（每行一个）"><textarea value={allowedDataClasses} onChange={(event) => setAllowedDataClasses(event.target.value)} rows={3} /></Field><Field label="生效时间"><input type="datetime-local" value={validFrom} onChange={(event) => setValidFrom(event.target.value)} /></Field><Field label="失效时间"><input type="datetime-local" value={validUntil} onChange={(event) => setValidUntil(event.target.value)} /></Field></div>{!validWindow && <div className={styles.validation}>Offer 有效期必须完整覆盖所有 Bucket 的交付窗口。</div>}</fieldset>
      <div className={styles.boundary}>草稿只保存可审计意图，不发布到市场、不生成报价、不预留容量，也不移动资金。</div>
      <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对服务规格、容量、价格和授权范围，确认创建 draft。</span></label>
      <footer><button type="button" className={styles.secondary} onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在创建' : '创建草稿'}</button></footer>
    </form>
  </section></div>
}

function Field({ label, wide, children }: { label: string; wide?: boolean; children: ReactNode }) { return <label data-wide={wide || undefined}><span>{label}</span>{children}</label> }
function NumberInput({ value, min = 1, onChange }: { value: number; min?: number; onChange: (value: number) => void }) { return <input type="number" min={min} step={1} value={value} onChange={(event) => onChange(Number(event.target.value))} /> }
function splitLines(value: string) { return unique(value.split(/\r?\n|,/).map((item) => item.trim()).filter(Boolean)) }
function unique(values: string[]) { return [...new Set(values)] }
function gib(value: number) { return Math.round(value * 1024 * 1024 * 1024) }
function shortId(value: string) { return value.length <= 24 ? value : value.slice(0, 24) }
function localValue(date: Date) { return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16) }
function defaultValidUntil(buckets: MyComputeCapacityBucket[]) { const latest = Math.max(Date.now() + 24 * 60 * 60_000, ...buckets.map((item) => new Date(item.ends_at_utc).getTime())); return new Date(latest) }
