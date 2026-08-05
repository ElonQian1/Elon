import { useState, type FormEvent } from 'react'
import { LoaderCircle, RotateCcw, X } from 'lucide-react'
import {
  type ComputeActivationQuarantine,
  type PrepareActivationRecoveryPlanBody,
} from './computeActivationAdminApi'
import styles from './ActivationActionDialog.module.css'

interface Props {
  quarantine: ComputeActivationQuarantine
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: PrepareActivationRecoveryPlanBody) => Promise<void>
}

type RouteMode = 'keep' | 'endpoint' | 'adapter'
const SHA256 = /^[0-9a-f]{64}$/

export default function PrepareActivationRecoveryDialog({ quarantine, busy, error, onClose, onSubmit }: Props) {
  const [routeMode, setRouteMode] = useState<RouteMode>('keep')
  const [endpointId, setEndpointId] = useState(`${quarantine.provider_id}-endpoint`)
  const [transport, setTransport] = useState('pc_node_agent')
  const [addressHint, setAddressHint] = useState('')
  const [gatewayId, setGatewayId] = useState('')
  const [credentialRef, setCredentialRef] = useState('')
  const [adapterId, setAdapterId] = useState('')
  const [adapterVersion, setAdapterVersion] = useState('')
  const [configRevision, setConfigRevision] = useState('1')
  const [configDigest, setConfigDigest] = useState('')
  const [hardwareDigest, setHardwareDigest] = useState('')
  const [trustTier, setTrustTier] = useState('platform_verified')
  const [verifiedAt, setVerifiedAt] = useState(localDateTimeValue())
  const [remediation, setRemediation] = useState('')
  const [evidenceText, setEvidenceText] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const evidenceRefs = uniqueLines(evidenceText)
  const endpointReady = routeMode !== 'endpoint' || Boolean(endpointId.trim() && transport.trim())
  const adapterReady = routeMode !== 'adapter' || Boolean(
    adapterId.trim() && adapterVersion.trim() && Number.isInteger(Number(configRevision))
      && Number(configRevision) > 0 && SHA256.test(configDigest.trim()),
  )
  const valid = Boolean(
    endpointReady && adapterReady && SHA256.test(hardwareDigest.trim())
      && trustTier.trim() && trustTier.trim() !== 'self_declared' && verifiedAt
      && remediation.trim() && evidenceRefs.length > 0 && evidenceRefs.length <= 20
      && confirmed && !busy,
  )

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      idempotency_key: `activation-recovery-plan:${quarantine.quarantine_digest}`,
      expected_quarantine_digest: quarantine.quarantine_digest,
      endpoint: routeMode === 'endpoint' ? {
        endpoint_id: endpointId.trim(),
        transport: transport.trim(),
        address_hint: optional(addressHint),
        gateway_id: optional(gatewayId),
        credential_ref: optional(credentialRef),
      } : null,
      adapter: routeMode === 'adapter' ? {
        adapter_id: adapterId.trim(),
        adapter_version: adapterVersion.trim(),
        config_revision: Number(configRevision),
        config_digest: configDigest.trim(),
      } : null,
      verified_hardware_digest: hardwareDigest.trim(),
      trust_tier: trustTier.trim(),
      verified_at: new Date(verifiedAt).toISOString(),
      remediation_summary: remediation.trim(),
      evidence_refs: evidenceRefs,
      confirm_prepare: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
    <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="prepare-recovery-title">
      <header className={styles.header}><div><RotateCcw size={18} /><h2 id="prepare-recovery-title">准备隔离恢复计划</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
      <form onSubmit={(event) => void submit(event)}>
        {error && <div className={styles.error} role="alert">{error}</div>}
        <div className={styles.grid}>
          <label><span>路由修复方式</span><select value={routeMode} onChange={(event) => { setRouteMode(event.target.value as RouteMode); setConfirmed(false) }}><option value="keep">保留现有路由</option><option value="endpoint">更新 Endpoint</option><option value="adapter">更新 Adapter</option></select></label>
          <label><span>目标信任层</span><input value={trustTier} onChange={(event) => { setTrustTier(event.target.value); setConfirmed(false) }} maxLength={80} required /></label>
          {routeMode === 'endpoint' && <><label><span>Endpoint ID</span><input value={endpointId} onChange={(event) => { setEndpointId(event.target.value); setConfirmed(false) }} maxLength={160} required /></label><label><span>传输协议</span><input value={transport} onChange={(event) => { setTransport(event.target.value); setConfirmed(false) }} maxLength={80} required /></label><label><span>地址提示（选填）</span><input value={addressHint} onChange={(event) => { setAddressHint(event.target.value); setConfirmed(false) }} maxLength={512} /></label><label><span>Gateway ID（选填）</span><input value={gatewayId} onChange={(event) => { setGatewayId(event.target.value); setConfirmed(false) }} maxLength={160} /></label><label className={styles.wide}><span>凭据引用（选填）</span><input value={credentialRef} onChange={(event) => { setCredentialRef(event.target.value); setConfirmed(false) }} maxLength={512} placeholder="仅填写保险库或环境变量引用" /></label></>}
          {routeMode === 'adapter' && <><label><span>Adapter ID</span><input value={adapterId} onChange={(event) => { setAdapterId(event.target.value); setConfirmed(false) }} maxLength={160} required /></label><label><span>Adapter 版本</span><input value={adapterVersion} onChange={(event) => { setAdapterVersion(event.target.value); setConfirmed(false) }} maxLength={80} required /></label><label><span>配置版本</span><input type="number" min="1" step="1" value={configRevision} onChange={(event) => { setConfigRevision(event.target.value); setConfirmed(false) }} required /></label><label className={styles.wide}><span>配置 SHA-256</span><input value={configDigest} onChange={(event) => { setConfigDigest(event.target.value.toLowerCase()); setConfirmed(false) }} maxLength={64} spellCheck={false} required /></label></>}
          <label className={styles.wide}><span>已验证硬件 SHA-256</span><input value={hardwareDigest} onChange={(event) => { setHardwareDigest(event.target.value.toLowerCase()); setConfirmed(false) }} maxLength={64} spellCheck={false} placeholder="64 位小写摘要" required /></label>
          <label><span>验证时间</span><input type="datetime-local" value={verifiedAt} onChange={(event) => { setVerifiedAt(event.target.value); setConfirmed(false) }} required /></label>
        </div>
        <label className={styles.reason}><span>修复说明</span><textarea value={remediation} onChange={(event) => { setRemediation(event.target.value); setConfirmed(false) }} maxLength={1000} rows={4} required /></label>
        <label className={styles.reason}><span>证据引用（每行一项，最多 20 项）</span><textarea value={evidenceText} onChange={(event) => { setEvidenceText(event.target.value); setConfirmed(false) }} rows={4} placeholder="工单、Artifact 或验证报告引用" required /></label>
        <div className={styles.boundary}>准备计划只固定下一 Provider 版本和证据摘要。不得填写密钥正文；不会解除隔离、恢复旧 Offer、启动节点或结算资金。</div>
        <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对隔离摘要、修复证据、路由引用和验证时间，确认固定恢复目标。</span></label>
        <footer className={styles.footer}><button type="button" className={styles.secondary} onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在准备' : '准备恢复计划'}</button></footer>
      </form>
    </section>
  </div>
}

function optional(value: string) { return value.trim() || null }
function uniqueLines(value: string) { return Array.from(new Set(value.split(/\r?\n/).map((line) => line.trim()).filter(Boolean))).sort() }
function localDateTimeValue() { const now = new Date(); return new Date(now.getTime() - now.getTimezoneOffset() * 60_000).toISOString().slice(0, 16) }
