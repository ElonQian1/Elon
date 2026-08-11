import { useState, type FormEvent, type ReactNode } from 'react'
import { X } from 'lucide-react'
import {
  createIdempotencyKey,
  defaultCapabilities,
  requireDigest,
  requirePositiveInteger,
  updateCapabilityRevision,
} from './externalPoolDraft'
import { type SubmitReleaseBody } from './externalPoolApi'
import styles from './ExternalPoolDialog.module.css'

interface Props { busy: boolean; error: string; onClose: () => void; onSubmit: (body: SubmitReleaseBody) => Promise<void> }

export default function AdapterReleaseSubmitDialog({ busy, error, onClose, onSubmit }: Props) {
  const [adapterId, setAdapterId] = useState('')
  const [releaseVersion, setReleaseVersion] = useState('')
  const [artifactRef, setArtifactRef] = useState('')
  const [implementationDigest, setImplementationDigest] = useState('')
  const [capabilities, setCapabilities] = useState(defaultCapabilities)
  const [verificationKind, setVerificationKind] = useState('signed_challenge')
  const [verifierId, setVerifierId] = useState('')
  const [verifierRevision, setVerifierRevision] = useState(1)
  const [verifierDigest, setVerifierDigest] = useState('')
  const [note, setNote] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [localError, setLocalError] = useState('')
  const [idempotencyKey] = useState(() => createIdempotencyKey('external-pool-release'))

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (busy) return
    setLocalError('')
    try {
      const artifact = required(artifactRef, '候选工件引用')
      if (!/^artifact-ref:[A-Za-z0-9._-]{1,160}$/.test(artifact)) throw new Error('候选工件必须使用 artifact-ref: 定位符')
      capabilities.forEach((capability) => requirePositiveInteger(capability.capability_revision, `${capability.capability_id} 修订号`))
      if (!confirmed) throw new Error('请确认本次提交只登记候选 release 元数据')
      await onSubmit({
        idempotency_key: idempotencyKey,
        adapter_id: required(adapterId, 'Adapter ID'),
        release_version: required(releaseVersion, 'Release 版本'),
        candidate_artifact_ref: artifact,
        declared_implementation_sha256: requireDigest(implementationDigest, '实现摘要'),
        supported_capabilities: capabilities,
        expected_credential_verifier: {
          verification_kind: required(verificationKind, '验证类型'),
          verifier_id: required(verifierId, 'Verifier ID'),
          verifier_revision: requirePositiveInteger(verifierRevision, 'Verifier 修订号'),
          verifier_digest: requireDigest(verifierDigest, 'Verifier 摘要'),
        },
        submission_note: note.trim(),
        confirm_submission: true,
      })
    } catch (reason) { setLocalError(messageOf(reason, 'Adapter release 提交失败')) }
  }

  return <div className={styles.overlay} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose() }}><form className={styles.wideDialog} onSubmit={(event) => void submit(event)}>
    <header><div><span>平台 Adapter 治理</span><h2>新建候选 release</h2></div><button type="button" title="关闭" aria-label="关闭" onClick={onClose} disabled={busy}><X size={18} /></button></header>
    <div className={styles.formGrid}>
      <Field label="Adapter ID"><input value={adapterId} maxLength={160} onChange={(event) => setAdapterId(event.target.value)} /></Field>
      <Field label="Release 版本"><input value={releaseVersion} maxLength={80} onChange={(event) => setReleaseVersion(event.target.value)} /></Field>
      <Field label="候选工件定位符"><input value={artifactRef} placeholder="artifact-ref:artifact-id" onChange={(event) => setArtifactRef(event.target.value)} /></Field>
      <Field label="声明实现 SHA-256"><input value={implementationDigest} maxLength={64} onChange={(event) => setImplementationDigest(event.target.value)} /></Field>
    </div>
    <section className={styles.formSection}><header><strong>固定协议能力</strong><span>必须完整保留六项，页面只允许调整正整数修订号</span></header><div className={styles.capabilityGrid}>{capabilities.map((capability, index) => <label key={capability.capability_id}><span>{capability.capability_id}</span><input type="number" min="1" step="1" value={capability.capability_revision} onChange={(event) => setCapabilities((current) => updateCapabilityRevision(current, index, Number(event.target.value)))} /></label>)}</div></section>
    <section className={styles.formSection}><header><strong>预期凭据 Verifier</strong><span>仅绑定未来 verifier 身份，不证明 registry 存在</span></header><div className={styles.formGrid}>
      <Field label="验证类型"><input value={verificationKind} maxLength={80} onChange={(event) => setVerificationKind(event.target.value)} /></Field>
      <Field label="Verifier ID"><input value={verifierId} maxLength={160} onChange={(event) => setVerifierId(event.target.value)} /></Field>
      <Field label="Verifier 修订号"><input type="number" min="1" step="1" value={verifierRevision} onChange={(event) => setVerifierRevision(Number(event.target.value))} /></Field>
      <Field label="Verifier SHA-256"><input value={verifierDigest} maxLength={64} onChange={(event) => setVerifierDigest(event.target.value)} /></Field>
      <label className={styles.fullField}><span>提交说明</span><textarea rows={3} maxLength={2000} value={note} onChange={(event) => setNote(event.target.value)} /></label>
    </div></section>
    {(localError || error) && <div className={styles.dialogError}>{localError || error}</div>}
    <label className={styles.confirmRow}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>确认只提交候选元数据；系统尚未下载、重算摘要、验签、加载工件或生成 v213 route</span></label>
    <footer><span /><div><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={busy || !confirmed}>{busy ? '提交中' : '提交 release'}</button></div></footer>
  </form></div>
}

function Field({ label, children }: { label: string; children: ReactNode }) { return <label><span>{label}</span>{children}</label> }
function required(value: string, label: string) { const normalized = value.trim(); if (!normalized) throw new Error(`${label}不能为空`); return normalized }
function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
