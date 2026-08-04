import { useMemo, useState, type FormEvent } from 'react'
import { FileCheck2, LoaderCircle, X } from 'lucide-react'
import { type SubmitActivationEvidenceBody } from './computeActivationApi'
import styles from './SubmitActivationEvidenceDialog.module.css'

interface Props {
  poolId: string
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: SubmitActivationEvidenceBody) => Promise<void>
}

export default function SubmitActivationEvidenceDialog({ poolId, busy, error, onClose, onSubmit }: Props) {
  const [nodeBindingRef, setNodeBindingRef] = useState('')
  const [readyDigest, setReadyDigest] = useState('')
  const [routeDigest, setRouteDigest] = useState('')
  const [hardwareDigest, setHardwareDigest] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [requestId] = useState(() => globalThis.crypto.randomUUID())
  const digestsValid = useMemo(() => [readyDigest, routeDigest, hardwareDigest].every(isDigest), [hardwareDigest, readyDigest, routeDigest])
  const canSubmit = Boolean(nodeBindingRef.trim() && nodeBindingRef.trim().length <= 160 && digestsValid && confirmed && !busy)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!canSubmit) return
    await onSubmit({
      idempotency_key: `pc-activation-evidence-${requestId}`,
      node_binding_ref: nodeBindingRef.trim(),
      ready_capability_digest: readyDigest.trim(),
      route_proof_digest: routeDigest.trim(),
      hardware_observation_digest: hardwareDigest.trim(),
      confirm_evidence_submission: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
    <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="activation-evidence-title">
      <header className={styles.header}><div><span>{shortId(poolId)}</span><h2 id="activation-evidence-title">提交激活证据</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
      <div className={styles.identity}><FileCheck2 size={18} /><span>只提交引用和 SHA-256 摘要，不上传凭据、密钥或原始硬件报告。</span></div>
      <form onSubmit={(event) => void submit(event)}>
        {error && <div className={styles.error} role="alert">{error}</div>}
        <label className={styles.field}><span>节点绑定引用</span><input value={nodeBindingRef} onChange={(event) => { setNodeBindingRef(event.target.value); setConfirmed(false) }} maxLength={160} placeholder="例如 node-binding:本机节点编号" autoFocus required /></label>
        <DigestField label="ReadyCapability SHA-256" value={readyDigest} onChange={(value) => { setReadyDigest(value); setConfirmed(false) }} />
        <DigestField label="路由证明 SHA-256" value={routeDigest} onChange={(value) => { setRouteDigest(value); setConfirmed(false) }} />
        <DigestField label="硬件观测 SHA-256" value={hardwareDigest} onChange={(value) => { setHardwareDigest(value); setConfirmed(false) }} />
        {!digestsValid && (readyDigest || routeDigest || hardwareDigest) && <div className={styles.validation}>三个摘要都必须是 64 位小写十六进制 SHA-256。</div>}
        <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认这些内容只是待管理员审核的证据引用，不代表平台已经验证或激活资源。</span></label>
        <div className={styles.boundary}>提交不会连接节点、改变 Provider/Pool 状态、发布报价、开放预留、派发任务或移动资金。</div>
        <footer className={styles.footer}><button type="button" className={styles.cancelButton} onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.submitButton} disabled={!canSubmit}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在提交' : '提交审核'}</button></footer>
      </form>
    </section>
  </div>
}

function DigestField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return <label className={styles.field}><span>{label}</span><input value={value} onChange={(event) => onChange(event.target.value.trim().toLowerCase())} maxLength={64} spellCheck={false} placeholder="64 位小写十六进制摘要" required /></label>
}

function isDigest(value: string) { return /^[a-f0-9]{64}$/.test(value.trim()) }
function shortId(value: string) { return value.length <= 28 ? value : `${value.slice(0, 14)}…${value.slice(-8)}` }
