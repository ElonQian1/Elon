import { useState, type FormEvent } from 'react'
import { LoaderCircle, Network, X } from 'lucide-react'
import { type ComputeActivationEvidenceRequest } from '../compute-supply/computeActivationApi'
import { type PrepareActivationPlanBody } from './computeActivationAdminApi'
import styles from './ActivationActionDialog.module.css'

interface Props {
  request: ComputeActivationEvidenceRequest
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: PrepareActivationPlanBody) => Promise<void>
}

const SHA256 = /^[0-9a-f]{64}$/

export default function PrepareActivationPlanDialog({ request, busy, error, onClose, onSubmit }: Props) {
  const [endpointId, setEndpointId] = useState(`${request.provider_id}-endpoint`)
  const [transport, setTransport] = useState('pc_node_agent')
  const [addressHint, setAddressHint] = useState('')
  const [gatewayId, setGatewayId] = useState('')
  const [credentialRef, setCredentialRef] = useState('')
  const [hardwareDigest, setHardwareDigest] = useState('')
  const [trustTier, setTrustTier] = useState('platform_verified')
  const [verifiedAt, setVerifiedAt] = useState(localDateTimeValue())
  const [confirmed, setConfirmed] = useState(false)
  const valid = Boolean(endpointId.trim() && transport.trim() && trustTier.trim() !== 'self_declared'
    && SHA256.test(hardwareDigest.trim()) && verifiedAt && confirmed && !busy)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      idempotency_key: `activation-plan:${request.request_id}:${request.request_digest.slice(0, 12)}`,
      expected_request_digest: request.request_digest,
      endpoint: {
        endpoint_id: endpointId.trim(), transport: transport.trim(),
        address_hint: optional(addressHint), gateway_id: optional(gatewayId), credential_ref: optional(credentialRef),
      },
      verified_hardware_digest: hardwareDigest.trim(), trust_tier: trustTier.trim(),
      verified_at: new Date(verifiedAt).toISOString(), confirm_prepare: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
    <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="prepare-plan-title">
      <header className={styles.header}><div><Network size={18} /><h2 id="prepare-plan-title">准备不可变激活计划</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
      <form onSubmit={(event) => void submit(event)}>
        {error && <div className={styles.error} role="alert">{error}</div>}
        <div className={styles.grid}>
          <label><span>Endpoint ID</span><input value={endpointId} onChange={(event) => setEndpointId(event.target.value)} maxLength={160} required /></label>
          <label><span>传输协议</span><input value={transport} onChange={(event) => setTransport(event.target.value)} maxLength={80} required /></label>
          <label><span>地址提示（选填）</span><input value={addressHint} onChange={(event) => setAddressHint(event.target.value)} maxLength={512} /></label>
          <label><span>Gateway ID（选填）</span><input value={gatewayId} onChange={(event) => setGatewayId(event.target.value)} maxLength={160} /></label>
          <label><span>凭据引用（选填）</span><input value={credentialRef} onChange={(event) => setCredentialRef(event.target.value)} maxLength={512} placeholder="仅填写保险库或环境变量引用" /></label>
          <label><span>目标信任层</span><input value={trustTier} onChange={(event) => setTrustTier(event.target.value)} maxLength={80} required /></label>
          <label className={styles.wide}><span>已验证硬件 SHA-256</span><input value={hardwareDigest} onChange={(event) => setHardwareDigest(event.target.value.toLowerCase())} maxLength={64} spellCheck={false} placeholder="64 位小写摘要" required /></label>
          <label><span>验证时间</span><input type="datetime-local" value={verifiedAt} onChange={(event) => setVerifiedAt(event.target.value)} required /></label>
        </div>
        <div className={styles.boundary}>这里只保存路由和凭据引用，不得填写密钥正文。准备计划不会改变 Provider 或 Pool 状态。</div>
        <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对证据、路由引用、信任层和验证时间，确认固定下一 Provider revision。</span></label>
        <footer className={styles.footer}><button type="button" className={styles.secondary} onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在准备' : '准备计划'}</button></footer>
      </form>
    </section>
  </div>
}

function optional(value: string) { return value.trim() || null }
function localDateTimeValue() { const now = new Date(); return new Date(now.getTime() - now.getTimezoneOffset() * 60_000).toISOString().slice(0, 16) }
