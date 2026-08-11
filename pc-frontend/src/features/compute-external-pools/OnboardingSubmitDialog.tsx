import { useState, type FormEvent, type ReactNode } from 'react'
import { X } from 'lucide-react'
import {
  canonicalUtcNow,
  createIdempotencyKey,
  createRequestId,
  optionalDigest,
  optionalPair,
  parseIdentifiers,
  requireDigest,
  requirePositiveInteger,
} from './externalPoolDraft'
import { type SubmitOnboardingBody } from './externalPoolApi'
import styles from './ExternalPoolDialog.module.css'

interface Props {
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: SubmitOnboardingBody) => Promise<void>
}

interface FormState {
  providerId: string; displayName: string; homeRegion: string; taskKinds: string
  acceleratorKinds: string; regions: string; dataClasses: string
  declaredHardwareDigest: string; adapterId: string; releaseVersion: string
  configRevision: number; configDigest: string; credentialRef: string; credentialHint: string
  evidenceRef: string; evidenceDigest: string; ownerNote: string
}

const INITIAL: FormState = {
  providerId: '', displayName: '', homeRegion: 'cn-east', taskKinds: 'llm_inference',
  acceleratorKinds: 'consumer_gpu', regions: 'cn-east', dataClasses: 'public',
  declaredHardwareDigest: '', adapterId: '', releaseVersion: '', configRevision: 1,
  configDigest: '', credentialRef: '', credentialHint: '', evidenceRef: '',
  evidenceDigest: '', ownerNote: '',
}

export default function OnboardingSubmitDialog({ busy, error, onClose, onSubmit }: Props) {
  const [form, setForm] = useState(INITIAL)
  const [streaming, setStreaming] = useState(true)
  const [checkpointing, setCheckpointing] = useState(false)
  const [confirmed, setConfirmed] = useState(false)
  const [localError, setLocalError] = useState('')
  const [requestId] = useState(() => createRequestId('external-pool-request'))
  const [idempotencyKey] = useState(() => createIdempotencyKey('external-pool-onboarding'))

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((current) => ({ ...current, [key]: value }))
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (busy) return
    setLocalError('')
    try {
      const providerId = required(form.providerId, 'Provider ID')
      const displayName = required(form.displayName, '显示名称')
      const homeRegion = required(form.homeRegion, '主区域')
      const taskKinds = parseIdentifiers(form.taskKinds, '任务类型')
      const acceleratorKinds = parseIdentifiers(form.acceleratorKinds, '加速器类型')
      const regions = parseIdentifiers(form.regions, '服务区域')
      const allowedDataClasses = parseIdentifiers(form.dataClasses, '数据等级')
      if (!regions.includes(homeRegion)) throw new Error('服务区域必须包含主区域')
      const [credentialRef, credentialHint] = optionalPair(form.credentialRef, form.credentialHint, '凭据保管引用与提示')
      if (credentialRef && !/^(vault-ref|gateway-ref):[A-Za-z0-9._-]{1,160}$/.test(credentialRef)) throw new Error('凭据引用必须使用 vault-ref: 或 gateway-ref: 的服务端定位符')
      const [evidenceRef, evidenceDigest] = optionalPair(form.evidenceRef, form.evidenceDigest, '外部证据引用与摘要')
      if (evidenceRef && !/^evidence-ref:[A-Za-z0-9._-]{1,160}$/.test(evidenceRef)) throw new Error('外部证据必须使用 evidence-ref: 定位符')
      if (evidenceDigest) requireDigest(evidenceDigest, '外部证据摘要')
      if (!confirmed) throw new Error('请确认申请只登记受控元数据')
      await onSubmit({
        request_id: requestId,
        idempotency_key: idempotencyKey,
        submitted_at: canonicalUtcNow(),
        provider_id: providerId,
        display_name: displayName,
        home_region: homeRegion,
        task_kinds: taskKinds,
        accelerator_kinds: acceleratorKinds,
        regions,
        allowed_data_classes: allowedDataClasses,
        supports_streaming: streaming,
        supports_checkpointing: checkpointing,
        declared_hardware_digest: optionalDigest(form.declaredHardwareDigest, '声明硬件摘要'),
        adapter_intent: {
          expected_adapter_id: required(form.adapterId, 'Adapter ID'),
          expected_release_version: required(form.releaseVersion, 'Adapter 版本'),
          expected_config_revision: requirePositiveInteger(form.configRevision, '配置修订号'),
          expected_config_digest: required(form.configDigest, '配置摘要'),
        },
        credential_intent: { non_bearer_credential_ref: credentialRef, credential_hint: credentialHint },
        external_evidence_ref: evidenceRef,
        external_evidence_sha256: evidenceDigest,
        owner_note: form.ownerNote.trim(),
        confirm_submission: true,
      })
    } catch (reason) { setLocalError(messageOf(reason, '接入申请提交失败')) }
  }

  return <div className={styles.overlay} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose() }}>
    <form className={styles.wideDialog} onSubmit={(event) => void submit(event)}>
      <header><div><span>Provider Owner 声明</span><h2>新建外部算力池接入申请</h2></div><button type="button" title="关闭" aria-label="关闭" onClick={onClose} disabled={busy}><X size={18} /></button></header>
      <div className={styles.formGrid}>
        <Field label="Provider ID"><input value={form.providerId} maxLength={160} onChange={(event) => update('providerId', event.target.value)} /></Field>
        <Field label="显示名称"><input value={form.displayName} maxLength={160} onChange={(event) => update('displayName', event.target.value)} /></Field>
        <Field label="主区域"><input value={form.homeRegion} maxLength={80} onChange={(event) => update('homeRegion', event.target.value)} /></Field>
        <Field label="任务类型（逗号分隔）"><input value={form.taskKinds} onChange={(event) => update('taskKinds', event.target.value)} /></Field>
        <Field label="加速器类型（逗号分隔）"><input value={form.acceleratorKinds} onChange={(event) => update('acceleratorKinds', event.target.value)} /></Field>
        <Field label="服务区域（逗号分隔）"><input value={form.regions} onChange={(event) => update('regions', event.target.value)} /></Field>
        <Field label="允许数据等级（逗号分隔）"><input value={form.dataClasses} onChange={(event) => update('dataClasses', event.target.value)} /></Field>
        <Field label="声明硬件 SHA-256（可选）"><input value={form.declaredHardwareDigest} maxLength={64} onChange={(event) => update('declaredHardwareDigest', event.target.value)} /></Field>
      </div>
      <div className={styles.toggleRow}><label><input type="checkbox" checked={streaming} onChange={(event) => setStreaming(event.target.checked)} />支持流式输出</label><label><input type="checkbox" checked={checkpointing} onChange={(event) => setCheckpointing(event.target.checked)} />支持检查点</label></div>
      <section className={styles.formSection}><header><strong>预期 Adapter</strong><span>只声明版本与配置身份</span></header><div className={styles.formGrid}>
        <Field label="Adapter ID"><input value={form.adapterId} maxLength={160} onChange={(event) => update('adapterId', event.target.value)} /></Field>
        <Field label="Release 版本"><input value={form.releaseVersion} maxLength={80} onChange={(event) => update('releaseVersion', event.target.value)} /></Field>
        <Field label="配置修订号"><input type="number" min="1" step="1" value={form.configRevision} onChange={(event) => update('configRevision', Number(event.target.value))} /></Field>
        <Field label="Opaque 配置摘要"><input value={form.configDigest} maxLength={512} onChange={(event) => update('configDigest', event.target.value)} /></Field>
      </div></section>
      <section className={styles.formSection}><header><strong>服务端保管引用</strong><span>禁止填写 Token、密码、API Key 或 Cookie</span></header><div className={styles.formGrid}>
        <Field label="non-bearer 凭据定位符（可选）"><input value={form.credentialRef} placeholder="vault-ref:credential-id" onChange={(event) => update('credentialRef', event.target.value)} /></Field>
        <Field label="脱敏提示（可选）"><input value={form.credentialHint} maxLength={160} onChange={(event) => update('credentialHint', event.target.value)} /></Field>
        <Field label="外部证据定位符（可选）"><input value={form.evidenceRef} placeholder="evidence-ref:evidence-id" onChange={(event) => update('evidenceRef', event.target.value)} /></Field>
        <Field label="外部证据 SHA-256（可选）"><input value={form.evidenceDigest} maxLength={64} onChange={(event) => update('evidenceDigest', event.target.value)} /></Field>
        <label className={styles.fullField}><span>Owner 说明</span><textarea rows={3} maxLength={2000} value={form.ownerNote} onChange={(event) => update('ownerNote', event.target.value)} /></label>
      </div></section>
      {(localError || error) && <div className={styles.dialogError}>{localError || error}</div>}
      <label className={styles.confirmRow}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>确认本申请只登记候选来源，不验证凭据或硬件，不创建 route、Pool、Offer、Job 或结算</span></label>
      <footer><span className={styles.requestId}>申请 ID：{requestId}</span><div><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={busy || !confirmed}>{busy ? '提交中' : '提交申请'}</button></div></footer>
    </form>
  </div>
}

function Field({ label, children }: { label: string; children: ReactNode }) { return <label><span>{label}</span>{children}</label> }
function required(value: string, label: string) { const normalized = value.trim(); if (!normalized) throw new Error(`${label}不能为空`); return normalized }
function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
