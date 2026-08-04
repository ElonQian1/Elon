import { useState, type FormEvent } from 'react'
import { FileCheck2, LoaderCircle, Plus, Trash2, X } from 'lucide-react'
import {
  type ComputeAttemptUsageTemplateReceipt,
  type ComputeDeclaredResultArtifactInput,
  type DeclareComputeAttemptTerminalCandidateBody,
} from './computeExecutionApi'
import styles from './ComputeExecutionDialog.module.css'

type Outcome = DeclareComputeAttemptTerminalCandidateBody['outcome']
type ArtifactDraft = ComputeDeclaredResultArtifactInput & { key: string }

interface Props {
  template: ComputeAttemptUsageTemplateReceipt
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: DeclareComputeAttemptTerminalCandidateBody) => Promise<void>
}

export default function TerminalCandidateDialog({ template, busy, error, onClose, onSubmit }: Props) {
  const [identity] = useState(createIdentity)
  const [outcome, setOutcome] = useState<Outcome>('succeeded')
  const [terminalRef, setTerminalRef] = useState('')
  const [reasonCode, setReasonCode] = useState('completed')
  const [diagnosticRef, setDiagnosticRef] = useState('')
  const [outputDigest, setOutputDigest] = useState('')
  const [artifacts, setArtifacts] = useState<ArtifactDraft[]>(() => template.output_contract.result_artifact_required ? [newArtifact(template.output_contract.media_type)] : [])
  const [confirmed, setConfirmed] = useState(false)
  const succeeded = outcome === 'succeeded'
  const digestValid = !outputDigest || isDigest(outputDigest)
  const artifactsValid = artifacts.every((artifact) => artifact.artifact_id.trim() && isDigest(artifact.digest) && artifact.media_type === template.output_contract.media_type && Number.isSafeInteger(artifact.size_bytes) && artifact.size_bytes >= 0 && artifact.location_ref.trim())
  const totalBytes = artifacts.reduce((sum, artifact) => sum + (Number.isSafeInteger(artifact.size_bytes) ? artifact.size_bytes : 0), 0)
  const outputValid = !succeeded || (digestValid && (!template.output_contract.deterministic_digest_expected || isDigest(outputDigest)) && (!template.output_contract.result_artifact_required || artifacts.length > 0) && artifactsValid && totalBytes <= template.output_contract.max_output_bytes)
  const valid = Boolean(template.latest_snapshot && terminalRef.trim() && /^[a-z0-9._-]+$/.test(reasonCode) && outputValid && confirmed && !busy)

  function changeOutcome(next: Outcome) {
    setOutcome(next); setReasonCode(next === 'succeeded' ? 'completed' : next === 'failed' ? 'executor_failed' : 'executor_canceled'); setConfirmed(false)
  }

  function updateArtifact(key: string, update: Partial<ArtifactDraft>) {
    setArtifacts((current) => current.map((artifact) => artifact.key === key ? { ...artifact, ...update } : artifact)); setConfirmed(false)
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid || !template.latest_snapshot) return
    await onSubmit({
      expected_lease_revision: template.lease_revision,
      expected_lease_digest: template.lease_digest,
      expected_fencing_generation: template.fencing_generation,
      final_usage_snapshot_id: template.latest_snapshot.snapshot_id,
      final_usage_sequence_no: template.latest_snapshot.sequence_no,
      final_cumulative_usage_digest: template.latest_snapshot.cumulative_usage_digest,
      executor_terminal_ref: terminalRef.trim(), outcome, reason_code: reasonCode,
      diagnostic_ref: diagnosticRef.trim() || null,
      output_digest: succeeded && outputDigest ? outputDigest : null,
      result_artifacts: succeeded ? artifacts.map((artifact) => ({ artifact_id: artifact.artifact_id.trim(), digest_algorithm: 'sha256', digest: artifact.digest.trim(), media_type: artifact.media_type, size_bytes: artifact.size_bytes, location_ref: artifact.location_ref.trim(), encryption_profile: artifact.encryption_profile?.trim() || null })) : [],
      idempotency_key: identity,
      confirm_provider_declaration_only: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="terminal-candidate-title"><header><div><FileCheck2 size={18} /><h2 id="terminal-candidate-title">提交 Provider 终态候选</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.segmented} aria-label="终态结果">{(['succeeded', 'failed', 'canceled'] as Outcome[]).map((value) => <button type="button" data-active={outcome === value} key={value} onClick={() => changeOutcome(value)}>{outcomeLabel(value)}</button>)}</div><div className={styles.grid}><label data-wide="true"><span>外部终态引用</span><input value={terminalRef} onChange={(event) => { setTerminalRef(event.target.value); setConfirmed(false) }} placeholder="terminal://executor/event/..." /></label><label><span>原因码</span><input value={reasonCode} onChange={(event) => { setReasonCode(event.target.value); setConfirmed(false) }} /></label><label><span>诊断引用（可选）</span><input value={diagnosticRef} onChange={(event) => { setDiagnosticRef(event.target.value); setConfirmed(false) }} /></label>{succeeded && <label data-wide="true"><span>确定性输出 SHA-256{template.output_contract.deterministic_digest_expected ? '' : '（可选）'}</span><input value={outputDigest} onChange={(event) => { setOutputDigest(event.target.value.toLowerCase()); setConfirmed(false) }} /></label>}</div>{succeeded && <section className={styles.artifactList}><header><div><strong>结果工件</strong><span>{template.output_contract.media_type} · 上限 {template.output_contract.max_output_bytes} bytes</span></div><button type="button" onClick={() => { setArtifacts((current) => [...current, newArtifact(template.output_contract.media_type)]); setConfirmed(false) }} disabled={artifacts.length >= 32}><Plus size={14} />添加</button></header>{artifacts.map((artifact) => <div className={styles.artifact} key={artifact.key}><div className={styles.grid}><label><span>工件 ID</span><input value={artifact.artifact_id} onChange={(event) => updateArtifact(artifact.key, { artifact_id: event.target.value })} /></label><label><span>大小（bytes）</span><input type="number" min="0" step="1" value={artifact.size_bytes} onChange={(event) => updateArtifact(artifact.key, { size_bytes: Number(event.target.value) })} /></label><label data-wide="true"><span>SHA-256</span><input value={artifact.digest} onChange={(event) => updateArtifact(artifact.key, { digest: event.target.value.toLowerCase() })} /></label><label data-wide="true"><span>位置引用</span><input value={artifact.location_ref} onChange={(event) => updateArtifact(artifact.key, { location_ref: event.target.value })} placeholder="artifact://..." /></label><label data-wide="true"><span>加密档案（可选）</span><input value={artifact.encryption_profile ?? ''} onChange={(event) => updateArtifact(artifact.key, { encryption_profile: event.target.value || null })} /></label></div><button type="button" className={styles.removeButton} onClick={() => { setArtifacts((current) => current.filter((item) => item.key !== artifact.key)); setConfirmed(false) }} aria-label="移除工件" title="移除工件"><Trash2 size={14} /></button></div>)}</section>}{!outputValid && <div className={styles.error}>输出摘要、工件字段或总大小不符合当前 Job 输出合同。</div>}<div className={styles.boundary}>终态候选只保存 Provider 的第一份外部声明。它不改变 Lease、Job、容量或预授权，也不是 Execution Receipt 或平台验证结论。</div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对最终用量快照和输出合同，并理解该候选提交后不能被另一份迟到声明覆盖。</span></label><code>{template.latest_snapshot?.cumulative_usage_digest ?? '尚无最终用量快照'}</code><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在提交' : '提交候选'}</button></footer></form></section></div>
}

function newArtifact(mediaType: string): ArtifactDraft { return { key: createIdentity(), artifact_id: '', digest_algorithm: 'sha256', digest: '', media_type: mediaType, size_bytes: 0, location_ref: '', encryption_profile: null } }
function createIdentity() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-terminal:${nonce}` }
function isDigest(value: string) { return /^[a-f0-9]{64}$/.test(value) }
function outcomeLabel(value: Outcome) { return ({ succeeded: '成功', failed: '失败', canceled: '取消' } as Record<Outcome, string>)[value] }
