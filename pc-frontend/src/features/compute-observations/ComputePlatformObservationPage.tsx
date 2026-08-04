import { useCallback, useEffect, useState } from 'react'
import { CircleCheck, LoaderCircle, Radar, RefreshCw, TriangleAlert } from 'lucide-react'
import ObserveTerminalCandidateDialog from './ObserveTerminalCandidateDialog'
import {
  computePlatformObservationApi,
  type ComputeAttemptPlatformObservationReceipt,
  type ComputePendingPlatformObservationCandidate,
  type ObserveComputeAttemptTerminalCandidateBody,
} from './computePlatformObservationApi'
import styles from './ComputePlatformObservationPage.module.css'

export default function ComputePlatformObservationPage() {
  const [candidates, setCandidates] = useState<ComputePendingPlatformObservationCandidate[]>([])
  const [selected, setSelected] = useState<ComputePendingPlatformObservationCandidate | null>(null)
  const [latest, setLatest] = useState<ComputeAttemptPlatformObservationReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [dialogError, setDialogError] = useState('')

  const loadCandidates = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      setCandidates(await computePlatformObservationApi.listPending())
    } catch (reason) {
      setError(messageOf(reason, '待观测候选读取失败'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { void loadCandidates() }, [loadCandidates])

  async function observe(body: ObserveComputeAttemptTerminalCandidateBody) {
    if (!selected || busy) return
    setBusy(true)
    setDialogError('')
    try {
      const receipt = await computePlatformObservationApi.observe(selected, body)
      setLatest(receipt)
      setSelected(null)
      await loadCandidates()
    } catch (reason) {
      setDialogError(messageOf(reason, '平台观测证据保存失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className={styles.page}>
      <header className={styles.pageHeader}>
        <div><span>管理员控制面</span><h1>平台观测</h1><p>Attempt 终态证据</p></div>
        <button type="button" onClick={() => void loadCandidates()} disabled={loading} title="刷新待观测候选"><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新</button>
      </header>

      {error && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
      {latest && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />已登记平台观测 · 差异 meter {latest.variance_meters.length} · {shortId(latest.platform_observation_id)}</div>}

      <section className={styles.queue}>
        <header><div><Radar size={17} /><h2>待观测候选</h2></div><span>{candidates.length}</span></header>
        {loading && !candidates.length && <div className={styles.empty}><LoaderCircle size={17} className={styles.spinning} />读取候选</div>}
        {!loading && !candidates.length && <div className={styles.empty}>暂无待观测候选</div>}
        <div className={styles.candidateList}>
          {candidates.map((item) => {
            const candidate = item.terminal_candidate
            return (
              <article className={styles.candidate} key={candidate.terminal_candidate_id}>
                <header>
                  <div><b data-outcome={candidate.outcome}>{outcomeLabel(candidate.outcome)}</b><span>{formatTime(candidate.declared_at)}</span></div>
                  <button type="button" onClick={() => { setDialogError(''); setSelected(item) }}>登记观测</button>
                </header>
                <div className={styles.facts}>
                  <div><span>Job</span><strong>{shortId(candidate.job_id)}</strong></div>
                  <div><span>Provider</span><strong>{shortId(candidate.provider_id)}</strong></div>
                  <div><span>用量序号</span><strong>{item.provider_usage.sequence_no}</strong></div>
                  <div><span>Meter 数量</span><strong>{item.provider_usage.cumulative_declared_usage.length}</strong></div>
                </div>
                <div className={styles.meters}>{item.provider_usage.cumulative_declared_usage.map((reading) => <span key={reading.meter}>{reading.meter}: {reading.quantity}</span>)}</div>
                <code className={styles.digest}>{item.provider_usage.cumulative_usage_digest}</code>
              </article>
            )
          })}
        </div>
      </section>

      {selected && <ObserveTerminalCandidateDialog candidate={selected} busy={busy} error={dialogError} onClose={() => { if (!busy) setSelected(null) }} onSubmit={observe} />}
    </main>
  )
}

function outcomeLabel(value: string) { return ({ succeeded: '已完成', failed: '执行失败', canceled: '已取消' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
