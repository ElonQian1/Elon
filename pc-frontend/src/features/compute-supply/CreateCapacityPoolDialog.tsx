import { useMemo, useState, type FormEvent } from 'react'
import { DatabaseZap, LoaderCircle, Plus, Trash2, X } from 'lucide-react'
import {
  type CapacityMeterMode,
  type CreateMyComputeCapacityPoolBody,
} from './computeSupplyApi'
import styles from './CreateCapacityPoolDialog.module.css'

interface MeterDraft {
  id: string
  meter: string
  meterMode: CapacityMeterMode
  quantumUnits: string
}

interface CreateCapacityPoolDialogProps {
  providerName: string
  defaultRegion: string
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: CreateMyComputeCapacityPoolBody) => Promise<void>
}

export default function CreateCapacityPoolDialog({
  providerName,
  defaultRegion,
  busy,
  error,
  onClose,
  onSubmit,
}: CreateCapacityPoolDialogProps) {
  const [poolId] = useState(() => `pool-${globalThis.crypto.randomUUID()}`)
  const [scopeKey, setScopeKey] = useState('')
  const [region, setRegion] = useState(defaultRegion)
  const [profile, setProfile] = useState('{\n  "resource_kind": "user_declared_compute"\n}')
  const [meters, setMeters] = useState<MeterDraft[]>([
    { id: globalThis.crypto.randomUUID(), meter: 'gpu_second', meterMode: 'consumable', quantumUnits: '1' },
  ])
  const profileResult = useMemo(() => parseProfile(profile), [profile])
  const validMeters = meters.every((meter) => (
    meter.meter.trim()
    && /^\d+$/.test(meter.quantumUnits.trim())
    && Number(meter.quantumUnits) > 0
    && Number.isSafeInteger(Number(meter.quantumUnits))
  ))
  const uniqueMeters = new Set(meters.map((meter) => meter.meter.trim())).size === meters.length
  const canSubmit = Boolean(
    scopeKey.trim()
    && region.trim()
    && profileResult.value
    && meters.length
    && validMeters
    && uniqueMeters
    && !busy,
  )

  function updateMeter(id: string, patch: Partial<MeterDraft>) {
    setMeters((current) => current.map((meter) => meter.id === id ? { ...meter, ...patch } : meter))
  }

  function addMeter() {
    setMeters((current) => [
      ...current,
      { id: globalThis.crypto.randomUUID(), meter: '', meterMode: 'consumable', quantumUnits: '1' },
    ])
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!canSubmit || !profileResult.value) return
    await onSubmit({
      pool_id: poolId,
      resource_scope_key: scopeKey.trim(),
      region_or_data_zone: region.trim(),
      resource_profile: profileResult.value,
      meter_policies: meters.map((meter) => ({
        meter: meter.meter.trim(),
        meter_mode: meter.meterMode,
        quantum_units: Number(meter.quantumUnits),
      })),
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="create-pool-title">
        <header className={styles.header}>
          <div><span>{providerName}</span><h2 id="create-pool-title">登记 CapacityPool</h2></div>
          <button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button>
        </header>
        <div className={styles.identity}><DatabaseZap size={18} /><div><span>Pool ID</span><strong>{poolId}</strong></div></div>

        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.error} role="alert">{error}</div>}
          <div className={styles.twoColumns}>
            <label className={styles.field}><span>资源范围密钥</span><input value={scopeKey} onChange={(event) => setScopeKey(event.target.value)} maxLength={256} placeholder="例如本机节点 ID 或集群资产编号" autoFocus required /></label>
            <label className={styles.field}><span>区域 / 数据区</span><input value={region} onChange={(event) => setRegion(event.target.value)} maxLength={80} placeholder="例如 cn-jiangxi-jian" required /></label>
          </div>
          <label className={styles.field}>
            <span>资源档案（JSON 对象）</span>
            <textarea value={profile} onChange={(event) => setProfile(event.target.value)} rows={5} spellCheck={false} />
            {profileResult.error && <small>{profileResult.error}</small>}
          </label>

          <section className={styles.meterSection}>
            <header><div><strong>计量策略</strong><span>定义容量的最小计量单位</span></div><button type="button" onClick={addMeter} disabled={meters.length >= 64}><Plus size={15} />增加 meter</button></header>
            <div className={styles.meterRows}>
              {meters.map((meter) => (
                <div className={styles.meterRow} key={meter.id}>
                  <input value={meter.meter} onChange={(event) => updateMeter(meter.id, { meter: event.target.value })} maxLength={80} placeholder="meter，例如 gpu_second" aria-label="meter 名称" required />
                  <select value={meter.meterMode} onChange={(event) => updateMeter(meter.id, { meterMode: event.target.value as CapacityMeterMode })} aria-label="meter 模式">
                    <option value="consumable">消耗型</option><option value="reusable">复用型</option>
                  </select>
                  <input value={meter.quantumUnits} onChange={(event) => updateMeter(meter.id, { quantumUnits: event.target.value })} inputMode="numeric" placeholder="最小量子" aria-label="最小量子" required />
                  <button type="button" onClick={() => setMeters((current) => current.filter((item) => item.id !== meter.id))} disabled={meters.length === 1} aria-label="删除 meter" title="删除 meter"><Trash2 size={15} /></button>
                </div>
              ))}
            </div>
            {!uniqueMeters && <small className={styles.meterError}>meter 名称不能重复</small>}
          </section>

          <div className={styles.boundary}>创建只登记资源边界和计量合同，不发行容量、不激活 Pool，也不发布报价。</div>
          <footer className={styles.footer}>
            <button type="button" className={styles.cancelButton} onClick={onClose} disabled={busy}>取消</button>
            <button type="submit" className={styles.submitButton} disabled={!canSubmit}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在登记' : '确认登记'}</button>
          </footer>
        </form>
      </section>
    </div>
  )
}

function parseProfile(value: string): { value: Record<string, unknown> | null; error: string } {
  try {
    const parsed = JSON.parse(value) as unknown
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') return { value: null, error: '资源档案必须是 JSON 对象' }
    return { value: parsed as Record<string, unknown>, error: '' }
  } catch {
    return { value: null, error: 'JSON 格式不正确' }
  }
}
