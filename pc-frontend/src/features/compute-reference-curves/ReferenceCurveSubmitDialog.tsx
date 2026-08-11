import { useState, type FormEvent } from 'react'
import { X } from 'lucide-react'
import ReferenceCurveOfferEntryBuilder from './ReferenceCurveOfferEntryBuilder'
import {
  type ReferenceCurveEntryIntent,
  type SubmitReferenceCurveBody,
} from './computeReferenceCurveApi'
import {
  defaultLocalTime,
  localTimeToIso,
  reindexEntries,
} from './referenceCurveDraft'
import styles from './ReferenceCurveDialog.module.css'

interface Props {
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: SubmitReferenceCurveBody) => Promise<void>
}

export default function ReferenceCurveSubmitDialog({ busy, error, onClose, onSubmit }: Props) {
  const [curveId, setCurveId] = useState('platform-fallback-cny')
  const [curveVersion, setCurveVersion] = useState(1)
  const [validFrom, setValidFrom] = useState(() => defaultLocalTime(10))
  const [validUntil, setValidUntil] = useState(() => defaultLocalTime(130))
  const [quoteTtlSeconds, setQuoteTtlSeconds] = useState(300)
  const [submissionNote, setSubmissionNote] = useState('')
  const [entries, setEntries] = useState<ReferenceCurveEntryIntent[]>([])
  const [localError, setLocalError] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [idempotencyKey] = useState(newIdempotencyKey)

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (busy) return
    setLocalError('')
    try {
      if (!curveId.trim()) throw new Error('曲线 ID 不能为空')
      if (!Number.isSafeInteger(curveVersion) || curveVersion <= 0) throw new Error('曲线版本必须为正整数')
      if (quoteTtlSeconds < 30 || quoteTtlSeconds > 3600) throw new Error('报价有效期必须在 30 至 3600 秒之间')
      const validFromIso = localTimeToIso(validFrom)
      const validUntilIso = localTimeToIso(validUntil)
      if (Date.parse(validUntilIso) - Date.parse(validFromIso) < quoteTtlSeconds * 1000) {
        throw new Error('价格曲线有效区间必须至少容纳一个完整的报价 TTL')
      }
      if (!entries.length) throw new Error('至少加入一个 active Offer 交付窗口')
      if (!confirmed) throw new Error('请确认提交只登记回退价格材料')
      await onSubmit({
        idempotency_key: idempotencyKey,
        curve_id: curveId.trim(),
        curve_version: curveVersion,
        valid_from: validFromIso,
        valid_until: validUntilIso,
        quote_ttl_seconds: quoteTtlSeconds,
        entries: reindexEntries(entries),
        submission_note: submissionNote.trim(),
        confirm_submission: true,
      })
    } catch (reason) { setLocalError(messageOf(reason, '参考价格批次提交失败')) }
  }

  return <div className={styles.overlay} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose() }}>
    <form className={styles.submitDialog} onSubmit={(event) => void submit(event)}>
      <header><div><span>平台回退价格</span><h2>新建参考曲线批次</h2></div><button type="button" title="关闭" aria-label="关闭" onClick={onClose} disabled={busy}><X size={18} /></button></header>
      <div className={styles.batchFields}>
        <label><span>曲线 ID</span><input value={curveId} maxLength={160} onChange={(event) => setCurveId(event.target.value)} /></label>
        <label><span>版本</span><input type="number" min="1" step="1" value={curveVersion} onChange={(event) => setCurveVersion(Number(event.target.value))} /></label>
        <label><span>生效时间</span><input type="datetime-local" value={validFrom} onChange={(event) => setValidFrom(event.target.value)} /></label>
        <label><span>失效时间</span><input type="datetime-local" value={validUntil} onChange={(event) => setValidUntil(event.target.value)} /></label>
        <label><span>报价 TTL（秒）</span><input type="number" min="30" max="3600" step="1" value={quoteTtlSeconds} onChange={(event) => setQuoteTtlSeconds(Number(event.target.value))} /></label>
        <label className={styles.noteField}><span>提交说明</span><textarea value={submissionNote} maxLength={2000} rows={2} onChange={(event) => setSubmissionNote(event.target.value)} /></label>
      </div>
      <ReferenceCurveOfferEntryBuilder
        entries={entries}
        onAdd={(entry) => setEntries((current) => [...current, entry])}
        onRemove={(index) => setEntries((current) => current.filter((_, entryIndex) => entryIndex !== index))}
      />
      {(localError || error) && <div className={styles.dialogError}>{localError || error}</div>}
      <label className={styles.confirmRow}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>确认提交不会自动创建 Job、预留容量或移动资金</span></label>
      <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={busy || !entries.length || !confirmed}>{busy ? '提交中' : `提交 ${entries.length} 个条目`}</button></footer>
    </form>
  </div>
}

function newIdempotencyKey() {
  const suffix = typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`
  return `reference-curve:${suffix}`
}

function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
