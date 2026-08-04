import { useState, type FormEvent } from 'react'
import { LoaderCircle, PencilLine, X } from 'lucide-react'
import { type MyComputeOfferView, type ReviseComputeOfferDraftBody } from './computeOfferApi'
import styles from './OfferDraftActionDialog.module.css'

interface Props {
  view: MyComputeOfferView
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: ReviseComputeOfferDraftBody) => Promise<void>
}

export default function ReviseOfferDraftDialog({ view, busy, error, onClose, onSubmit }: Props) {
  const offer = view.offer
  const [capacity, setCapacity] = useState(() => Object.fromEntries(offer.capacity.map((line) => [line.bucket.bucket_id, { total: line.total_units, reservable: line.reservable_units }])))
  const [prices, setPrices] = useState(() => Object.fromEntries(offer.price_terms.components.map((line) => [line.meter, { consumer: line.consumer_unit_price_micros, provider: line.provider_unit_price_micros, max: line.max_units }])))
  const [maxConcurrency, setMaxConcurrency] = useState(offer.execution_limits.max_concurrent_attempts)
  const [maxRuntime, setMaxRuntime] = useState(offer.execution_limits.max_attempt_runtime_seconds)
  const [isPublic, setIsPublic] = useState(offer.authorization.public)
  const [accounts, setAccounts] = useState(offer.authorization.allowed_account_ids.join('\n'))
  const [projects, setProjects] = useState(offer.authorization.allowed_project_ids.join('\n'))
  const [dataClasses, setDataClasses] = useState(offer.authorization.allowed_data_classes.join('\n'))
  const [validFrom, setValidFrom] = useState(localValue(offer.valid_from))
  const [validUntil, setValidUntil] = useState(localValue(offer.valid_until))
  const [confirmed, setConfirmed] = useState(false)
  const capacityValid = offer.capacity.every((line) => {
    const value = capacity[line.bucket.bucket_id]
    return value && value.total > 0 && value.reservable >= 0 && value.reservable <= value.total
      && value.total % line.bucket.quantum_units === 0 && value.reservable % line.bucket.quantum_units === 0
  })
  const pricesValid = offer.price_terms.components.every((line) => {
    const value = prices[line.meter]
    return value && value.provider >= 0 && value.consumer >= value.provider && value.max > 0
  })
  const authorizationValid = isPublic || splitLines(accounts).length + splitLines(projects).length > 0
  const valid = Boolean(capacityValid && pricesValid && authorizationValid && splitLines(dataClasses).length
    && maxConcurrency > 0 && maxRuntime > 0 && new Date(validUntil) > new Date(validFrom) && confirmed && !busy)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      expected_offer_version: offer.offer_version,
      expected_offer_digest: offer.offer_digest,
      sku: {
        sku_id: offer.sku.sku_id, task_kind: offer.sku.task_kind,
        context_or_shape_bucket: offer.sku.context_or_shape_bucket,
        verification_tier: offer.sku.verification_tier, sla_tier: offer.sku.sla_tier,
        delivery_window_class: offer.sku.delivery_window_class,
      },
      model: offer.model,
      runtime: offer.runtime,
      resource_profile: {
        accelerator_kind: offer.resource_profile.accelerator_kind,
        accelerator_count: offer.resource_profile.accelerator_count,
        vram_bytes: offer.resource_profile.vram_bytes,
        ram_bytes: offer.resource_profile.ram_bytes,
      },
      capacity: offer.capacity.map((line) => ({ bucket_id: line.bucket.bucket_id, total_units: capacity[line.bucket.bucket_id].total, reservable_units: capacity[line.bucket.bucket_id].reservable })),
      execution_limits: { max_concurrent_attempts: maxConcurrency, max_attempt_runtime_seconds: maxRuntime },
      authorization: { public: isPublic, allowed_account_ids: isPublic ? [] : splitLines(accounts), allowed_project_ids: isPublic ? [] : splitLines(projects), allowed_data_classes: splitLines(dataClasses) },
      price_terms: { pricing_mode: offer.price_terms.pricing_mode, currency: offer.price_terms.currency, curve_id: offer.price_terms.curve_id, curve_version: offer.price_terms.curve_version, instrument_id: offer.price_terms.instrument_id, components: offer.price_terms.components.map((line) => ({ ...line, consumer_unit_price_micros: prices[line.meter].consumer, provider_unit_price_micros: prices[line.meter].provider, max_units: prices[line.meter].max })), fee_rules: offer.price_terms.fee_rules },
      valid_from: new Date(validFrom).toISOString(), valid_until: new Date(validUntil).toISOString(), confirm_revise: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="revise-offer-title">
    <header><div><PencilLine size={18} /><h2 id="revise-offer-title">修订 Offer 草稿</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
    <form onSubmit={(event) => void submit(event)}>
      {error && <div className={styles.error}>{error}</div>}
      <div className={styles.identity}><span>稳定身份</span><strong>{offer.sku.sku_id}</strong><code>{offer.offer_id}</code></div>
      <fieldset><legend>容量</legend>{offer.capacity.map((line) => <div className={styles.row} key={line.bucket.bucket_id}><span><strong>{line.bucket.meter}</strong><small>{shortId(line.bucket.bucket_id)} · 量子 {line.bucket.quantum_units}</small></span><label><small>总量</small><NumberInput value={capacity[line.bucket.bucket_id].total} step={line.bucket.quantum_units} onChange={(value) => setCapacity((current) => ({ ...current, [line.bucket.bucket_id]: { ...current[line.bucket.bucket_id], total: value } }))} /></label><label><small>可预留</small><NumberInput value={capacity[line.bucket.bucket_id].reservable} min={0} step={line.bucket.quantum_units} onChange={(value) => setCapacity((current) => ({ ...current, [line.bucket.bucket_id]: { ...current[line.bucket.bucket_id], reservable: value } }))} /></label></div>)}</fieldset>
      <fieldset><legend>价格（人民币微元/量子）</legend>{offer.price_terms.components.map((line) => <div className={styles.row} key={line.meter}><span><strong>{line.meter}</strong><small>单位 {line.unit_size}</small></span><label><small>Provider</small><NumberInput value={prices[line.meter].provider} min={0} onChange={(value) => setPrices((current) => ({ ...current, [line.meter]: { ...current[line.meter], provider: value } }))} /></label><label><small>消费者</small><NumberInput value={prices[line.meter].consumer} min={prices[line.meter].provider} onChange={(value) => setPrices((current) => ({ ...current, [line.meter]: { ...current[line.meter], consumer: value } }))} /></label><label><small>最大单位</small><NumberInput value={prices[line.meter].max} onChange={(value) => setPrices((current) => ({ ...current, [line.meter]: { ...current[line.meter], max: value } }))} /></label></div>)}</fieldset>
      <fieldset><legend>执行与授权</legend><div className={styles.grid}><label><span>最大并发</span><NumberInput value={maxConcurrency} onChange={setMaxConcurrency} /></label><label><span>最长运行秒数</span><NumberInput value={maxRuntime} onChange={setMaxRuntime} /></label><label><span>生效时间</span><input type="datetime-local" value={validFrom} onChange={(event) => setValidFrom(event.target.value)} /></label><label><span>失效时间</span><input type="datetime-local" value={validUntil} onChange={(event) => setValidUntil(event.target.value)} /></label></div><label className={styles.toggle}><input type="checkbox" checked={isPublic} onChange={(event) => setIsPublic(event.target.checked)} /><span>公开 Offer</span></label>{!isPublic && <div className={styles.grid}><label><span>允许账户</span><textarea value={accounts} onChange={(event) => setAccounts(event.target.value)} rows={3} /></label><label><span>允许项目</span><textarea value={projects} onChange={(event) => setProjects(event.target.value)} rows={3} /></label></div>}<label className={styles.dataClasses}><span>允许数据等级</span><textarea value={dataClasses} onChange={(event) => setDataClasses(event.target.value)} rows={3} /></label></fieldset>
      <div className={styles.boundary}>修订会追加 v{offer.offer_version + 1}，不会覆盖当前版本，也不会发布草稿或移动容量与资金。</div>
      <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认以当前版本和摘要提交完整替换合同。</span></label>
      <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在修订' : '提交修订'}</button></footer>
    </form>
  </section></div>
}

function NumberInput({ value, min = 1, step = 1, onChange }: { value: number; min?: number; step?: number; onChange: (value: number) => void }) { return <input type="number" min={min} step={step} value={value} onChange={(event) => onChange(Number(event.target.value))} /> }
function splitLines(value: string) { return [...new Set(value.split(/\r?\n|,/).map((item) => item.trim()).filter(Boolean))] }
function localValue(value: string) { const date = new Date(value); return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
