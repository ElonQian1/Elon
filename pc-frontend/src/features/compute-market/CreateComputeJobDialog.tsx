import { useMemo, useState, type FormEvent, type ReactNode } from 'react'
import { LoaderCircle, Plus, Trash2, X } from 'lucide-react'
import { type CreateComputeJobBody } from './computeMarketApi'
import styles from './CreateComputeJobDialog.module.css'

interface Props { busy: boolean; error: string; onClose: () => void; onSubmit: (body: CreateComputeJobBody) => Promise<void> }
interface MeterDraft { id: string; meter: string; maxQuantity: number }

const TASK_KINDS = ['llm_chat', 'embedding', 'rerank', 'image_generation', 'video_generation', 'evaluation_shard', 'gpu_batch']
const PROVIDER_KINDS = ['user_node', 'managed_cluster', 'external_pool']

export default function CreateComputeJobDialog({ busy, error, onClose, onSubmit }: Props) {
  const [identity] = useState(createIdentity)
  const [taskKind, setTaskKind] = useState('llm_chat')
  const [dataClass, setDataClass] = useState('public')
  const [budget, setBudget] = useState('10.000000')
  const [deadline, setDeadline] = useState(localValue(new Date(Date.now() + 4 * 60 * 60_000)))
  const [acceleratorKind, setAcceleratorKind] = useState('gpu')
  const [acceleratorCount, setAcceleratorCount] = useState(1)
  const [vramGiB, setVramGiB] = useState(1)
  const [ramGiB, setRamGiB] = useState(2)
  const [diskGiB, setDiskGiB] = useState(0)
  const [maxRuntime, setMaxRuntime] = useState(600)
  const [allowNetwork, setAllowNetwork] = useState(false)
  const [outputMediaType, setOutputMediaType] = useState('application/json')
  const [maxOutputMiB, setMaxOutputMiB] = useState(10)
  const [streaming, setStreaming] = useState(false)
  const [trustTier, setTrustTier] = useState('platform_verified')
  const [verificationTier, setVerificationTier] = useState('platform_verified')
  const [regions, setRegions] = useState('')
  const [providerKinds, setProviderKinds] = useState(PROVIDER_KINDS)
  const [meters, setMeters] = useState<MeterDraft[]>([{ id: newId(), meter: 'tokens', maxQuantity: 1000 }])
  const [confirmed, setConfirmed] = useState(false)
  const budgetMicros = parseMicros(budget)
  const validMeters = useMemo(() => meters.length > 0 && meters.every((item) => item.meter.trim() && item.maxQuantity > 0) && new Set(meters.map((item) => item.meter.trim())).size === meters.length, [meters])
  const valid = Boolean(budgetMicros !== null && new Date(deadline).getTime() > Date.now() && acceleratorKind.trim() && acceleratorCount > 0 && vramGiB >= 0 && ramGiB > 0 && diskGiB >= 0 && maxRuntime > 0 && outputMediaType.trim() && maxOutputMiB > 0 && trustTier.trim() && verificationTier.trim() && providerKinds.length && validMeters && confirmed && !busy)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid || budgetMicros === null) return
    await onSubmit({
      job_id: identity.jobId,
      idempotency_key: identity.idempotencyKey,
      merchant_id: null,
      workload: {
        schema: 'compute_federation.workload.v1', task_kind: taskKind, input_artifacts: [], model: null, runtime: null,
        resources: { accelerator_kinds: [acceleratorKind.trim()], min_accelerator_count: acceleratorCount, min_vram_bytes: gib(vramGiB), min_ram_bytes: gib(ramGiB), min_disk_bytes: gib(diskGiB), max_runtime_seconds: maxRuntime, allow_network_egress: allowNetwork },
        output: { media_type: outputMediaType.trim(), max_output_bytes: mib(maxOutputMiB), streaming, result_artifact_required: false, deterministic_digest_expected: false },
        usage_limits: meters.map((item) => ({ meter: item.meter.trim(), max_quantity: item.maxQuantity })), data_class: dataClass, shard: null,
        retry_policy: { max_attempts: 1, initial_backoff_ms: 0, max_backoff_ms: 0, retryable_error_codes: [] },
        checkpoint_policy: { mode: 'disabled', interval_seconds: null, max_checkpoints: 0, checkpoint_media_type: null },
        verification_policy: { verification_tier: verificationTier.trim(), minimum_independent_receipts: 1, duplicate_sample_rate_basis_points: 0, challenge_profile_id: null, require_server_metering: true },
        deadline_at: new Date(deadline).toISOString(),
      },
      provider_scope: { allowed_provider_ids: [], allowed_provider_kinds: providerKinds, excluded_provider_ids: [], required_trust_tier: trustTier.trim(), required_regions: splitLines(regions) },
      max_consumer_charge_micros: budgetMicros,
      currency: 'CNY',
    })
  }

  function toggleProviderKind(kind: string) { setProviderKinds((current) => current.includes(kind) ? current.filter((item) => item !== kind) : [...current, kind]); setConfirmed(false) }
  function updateMeter(id: string, patch: Partial<MeterDraft>) { setMeters((current) => current.map((item) => item.id === id ? { ...item, ...patch } : item)); setConfirmed(false) }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="create-job-title"><header><h2 id="create-job-title">创建算力需求</h2><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<fieldset><legend>任务与预算</legend><div className={styles.grid}><Field label="任务类型"><select value={taskKind} onChange={(event) => { setTaskKind(event.target.value); setConfirmed(false) }}>{TASK_KINDS.map((kind) => <option key={kind}>{kind}</option>)}</select></Field><Field label="数据等级"><select value={dataClass} onChange={(event) => { setDataClass(event.target.value); setConfirmed(false) }}><option value="public">公开</option><option value="low_sensitivity">低敏感</option><option value="restricted">受限</option></select></Field><Field label="最高预算（CNY）"><input value={budget} inputMode="decimal" onChange={(event) => { setBudget(event.target.value); setConfirmed(false) }} /></Field><Field label="任务截止时间"><input type="datetime-local" value={deadline} onChange={(event) => { setDeadline(event.target.value); setConfirmed(false) }} /></Field></div></fieldset><fieldset><legend>资源下限</legend><div className={styles.grid}><Field label="加速器类型"><input value={acceleratorKind} onChange={(event) => { setAcceleratorKind(event.target.value); setConfirmed(false) }} /></Field><NumberField label="数量" value={acceleratorCount} onChange={(value) => { setAcceleratorCount(value); setConfirmed(false) }} /><NumberField label="显存 GiB" value={vramGiB} min={0} onChange={(value) => { setVramGiB(value); setConfirmed(false) }} /><NumberField label="内存 GiB" value={ramGiB} onChange={(value) => { setRamGiB(value); setConfirmed(false) }} /><NumberField label="磁盘 GiB" value={diskGiB} min={0} onChange={(value) => { setDiskGiB(value); setConfirmed(false) }} /><NumberField label="最长运行秒数" value={maxRuntime} onChange={(value) => { setMaxRuntime(value); setConfirmed(false) }} /></div><label className={styles.toggle}><input type="checkbox" checked={allowNetwork} onChange={(event) => { setAllowNetwork(event.target.checked); setConfirmed(false) }} /><span>允许任务访问外部网络</span></label></fieldset><fieldset><legend>使用量上限</legend><div className={styles.meters}>{meters.map((item) => <div key={item.id}><input value={item.meter} onChange={(event) => updateMeter(item.id, { meter: event.target.value })} placeholder="meter" /><input type="number" min={1} step={1} value={item.maxQuantity} onChange={(event) => updateMeter(item.id, { maxQuantity: Number(event.target.value) })} /><button type="button" onClick={() => { setMeters((current) => current.filter((row) => row.id !== item.id)); setConfirmed(false) }} disabled={meters.length === 1} aria-label="删除使用量" title="删除使用量"><Trash2 size={14} /></button></div>)}</div><button type="button" className={styles.add} onClick={() => { setMeters((current) => [...current, { id: newId(), meter: '', maxQuantity: 1 }]); setConfirmed(false) }}><Plus size={14} />添加 meter</button></fieldset><fieldset><legend>输出与供给范围</legend><div className={styles.grid}><Field label="输出媒体类型"><input value={outputMediaType} onChange={(event) => { setOutputMediaType(event.target.value); setConfirmed(false) }} /></Field><NumberField label="最大输出 MiB" value={maxOutputMiB} onChange={(value) => { setMaxOutputMiB(value); setConfirmed(false) }} /><Field label="要求信任层"><input value={trustTier} onChange={(event) => { setTrustTier(event.target.value); setConfirmed(false) }} /></Field><Field label="验证等级"><input value={verificationTier} onChange={(event) => { setVerificationTier(event.target.value); setConfirmed(false) }} /></Field><Field label="要求区域（每行一个）" wide><textarea rows={2} value={regions} onChange={(event) => { setRegions(event.target.value); setConfirmed(false) }} /></Field></div><label className={styles.toggle}><input type="checkbox" checked={streaming} onChange={(event) => { setStreaming(event.target.checked); setConfirmed(false) }} /><span>要求流式输出</span></label><div className={styles.kinds}>{PROVIDER_KINDS.map((kind) => <label key={kind}><input type="checkbox" checked={providerKinds.includes(kind)} onChange={() => toggleProviderKind(kind)} /><span>{kind}</span></label>)}</div></fieldset><div className={styles.boundary}>创建后仅登记 submitted Job；不会选择报价、冻结余额、持有容量或派发节点。</div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对任务、预算、资源和使用量上限，确认创建需求。</span></label><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在创建' : '创建需求'}</button></footer></form></section></div>
}

function Field({ label, wide, children }: { label: string; wide?: boolean; children: ReactNode }) { return <label data-wide={wide || undefined}><span>{label}</span>{children}</label> }
function NumberField({ label, value, min = 1, onChange }: { label: string; value: number; min?: number; onChange: (value: number) => void }) { return <Field label={label}><input type="number" min={min} step={1} value={value} onChange={(event) => onChange(Number(event.target.value))} /></Field> }
function createIdentity() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return { jobId: `job_${nonce}`, idempotencyKey: `pc-compute-job:${nonce}` } }
function newId() { return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}` }
function parseMicros(value: string) { const match = /^(\d+)(?:\.(\d{0,6}))?$/.exec(value.trim()); if (!match) return null; const amount = BigInt(match[1]) * 1_000_000n + BigInt((match[2] ?? '').padEnd(6, '0')); return amount <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(amount) : null }
function splitLines(value: string) { return [...new Set(value.split(/\r?\n|,/).map((item) => item.trim()).filter(Boolean))] }
function gib(value: number) { return Math.round(value * 1024 * 1024 * 1024) }
function mib(value: number) { return Math.round(value * 1024 * 1024) }
function localValue(date: Date) { return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16) }
