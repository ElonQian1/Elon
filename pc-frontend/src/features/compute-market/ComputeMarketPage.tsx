import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleCheck, LoaderCircle, Plus, RefreshCw, Search, TriangleAlert } from 'lucide-react'
import { useProjectStore } from '../conversation/useProjectStore'
import ComputeReservationPanel from './ComputeReservationPanel'
import CreateComputeJobDialog from './CreateComputeJobDialog'
import QuoteComputeJobDialog from './QuoteComputeJobDialog'
import { computeMarketApi, type ComputeJobReceipt, type ComputeQuoteCandidate, type ComputeQuoteCandidatePage, type CreateComputeJobBody } from './computeMarketApi'
import styles from './ComputeMarketPage.module.css'

export default function ComputeMarketPage() {
  const projects = useProjectStore((state) => state.projects)
  const projectsLoaded = useProjectStore((state) => state.projectsLoaded)
  const activeProjectId = useProjectStore((state) => state.activeProjectId)
  const loadProjects = useProjectStore((state) => state.loadProjects)
  const [projectId, setProjectId] = useState('')
  const [jobs, setJobs] = useState<ComputeJobReceipt[]>([])
  const [selectedId, setSelectedId] = useState('')
  const [candidates, setCandidates] = useState<ComputeQuoteCandidatePage | null>(null)
  const [quoteCandidate, setQuoteCandidate] = useState<ComputeQuoteCandidate | null>(null)
  const [createOpen, setCreateOpen] = useState(false)
  const [loading, setLoading] = useState(false)
  const [candidateLoading, setCandidateLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const selected = useMemo(() => jobs.find((item) => item.job.job_id === selectedId) ?? jobs[0] ?? null, [jobs, selectedId])
  const reservationCandidate = useMemo(() => {
    if (!selected || !candidates || candidates.job_revision !== selected.revision || candidates.job_digest !== selected.job_digest) return null
    return candidates.candidates.find((candidate) => candidate.price_snapshot.snapshot_id === selected.job.price_snapshot_id) ?? null
  }, [candidates, selected])

  useEffect(() => { if (!projectsLoaded) void loadProjects().catch(() => undefined) }, [loadProjects, projectsLoaded])
  useEffect(() => { if (!projectId) setProjectId(activeProjectId || projects[0]?.id || '') }, [activeProjectId, projectId, projects])

  const loadJobs = useCallback(async (preferredId?: string) => {
    if (!projectId) { setJobs([]); return }
    setLoading(true); setError('')
    try {
      const all = await computeMarketApi.listJobs()
      const scoped = all.filter((item) => item.job.project_id === projectId)
      setJobs(scoped)
      setSelectedId((current) => preferredId && scoped.some((item) => item.job.job_id === preferredId) ? preferredId : scoped.some((item) => item.job.job_id === current) ? current : scoped[0]?.job.job_id ?? '')
    } catch (reason) { setError(messageOf(reason, '算力 Job 读取失败')) } finally { setLoading(false) }
  }, [projectId])

  useEffect(() => { setCandidates(null); setSelectedId(''); setNotice(''); void loadJobs() }, [loadJobs])
  useEffect(() => { setCandidates(null); setQuoteCandidate(null) }, [selected?.job.job_id])

  async function create(body: CreateComputeJobBody) {
    if (!projectId || busy) return
    setBusy(true); setError(''); setNotice('')
    try { const created = await computeMarketApi.createJob(projectId, body); setCreateOpen(false); setNotice('算力需求已登记为 submitted。'); await loadJobs(created.job.job_id) }
    catch (reason) { setError(messageOf(reason, '算力需求创建失败')) } finally { setBusy(false) }
  }

  async function discover() {
    if (!projectId || !selected || candidateLoading) return
    setCandidateLoading(true); setError(''); setNotice('')
    try { setCandidates(await computeMarketApi.candidates(projectId, selected.job.job_id)) }
    catch (reason) { setError(messageOf(reason, '报价候选读取失败')) } finally { setCandidateLoading(false) }
  }

  async function quote() {
    if (!projectId || !selected || !candidates || !quoteCandidate || busy) return
    setBusy(true); setError(''); setNotice('')
    try { const updated = await computeMarketApi.quote(projectId, selected.job.job_id, quoteCandidate, candidates); setQuoteCandidate(null); setNotice('Job 已绑定不可变报价；尚未冻结余额或容量。'); await loadJobs(updated.job.job_id); setCandidates(null) }
    catch (reason) { setError(messageOf(reason, '锁定报价失败')) } finally { setBusy(false) }
  }

  async function jobChanged(jobId: string, message: string) {
    setNotice(message); setCandidates(null); await loadJobs(jobId)
  }

  return <main className={styles.page}><header className={styles.header}><div><span>消费者控制面</span><h1>算力市场</h1><p>按项目登记需求、发现当前报价并锁定不可变快照。</p></div><div><select value={projectId} onChange={(event) => { setProjectId(event.target.value); setError('') }} disabled={!projects.length}><option value="">选择项目</option>{projects.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select><button type="button" onClick={() => void loadJobs()} disabled={!projectId || loading}><RefreshCw size={14} className={loading ? styles.spinning : ''} />刷新</button><button type="button" className={styles.primary} onClick={() => { setError(''); setCreateOpen(true) }} disabled={!projectId}><Plus size={14} />创建需求</button></div></header>{error && !createOpen && !quoteCandidate && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}{notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}<section className={styles.workbench}><aside><header><strong>当前项目 Job</strong><span>{jobs.length}</span></header>{jobs.map((item) => <button type="button" key={item.job.job_id} data-active={item.job.job_id === selected?.job.job_id} onClick={() => setSelectedId(item.job.job_id)}><span><strong>{item.job.workload.task_kind}</strong><small>{statusLabel(item.job.status)} · {formatAmount(item.job.max_consumer_charge_micros, item.job.currency)}</small></span></button>)}{loading && !jobs.length && <div className={styles.empty}><LoaderCircle size={16} className={styles.spinning} />读取需求</div>}{!loading && !jobs.length && <div className={styles.empty}>当前项目尚无算力 Job</div>}</aside><div className={styles.detail}>{selected ? <><header className={styles.jobHeader}><div><span>Job ID</span><h2>{selected.job.job_id}</h2></div><b>{statusLabel(selected.job.status)}</b></header><div className={styles.facts}><div><span>任务</span><strong>{selected.job.workload.task_kind}</strong></div><div><span>版本</span><strong>{selected.revision}</strong></div><div><span>预算</span><strong>{formatAmount(selected.job.max_consumer_charge_micros, selected.job.currency)}</strong></div><div><span>截止时间</span><strong>{formatTime(selected.job.workload.deadline_at)}</strong></div></div><section className={styles.contract}><div>{selected.job.workload.usage_limits.map((limit) => <span key={limit.meter}>{limit.meter}: {limit.max_quantity}</span>)}</div><code>{selected.job_digest}</code></section>{selected.job.selected_offer && <section className={styles.selection}><CircleCheck size={16} /><div><strong>已锁定 Offer {shortId(selected.job.selected_offer.offer_id)}</strong><span>Price Snapshot {shortId(selected.job.price_snapshot_id ?? '')}</span></div></section>}<section className={styles.candidates}><header><div><h3>报价候选</h3><span>{candidates ? `扫描 ${candidates.scanned_count}，命中 ${candidates.candidates.length}` : '仅返回通过完整合同校验的当前候选'}</span></div><button type="button" onClick={() => void discover()} disabled={candidateLoading || !['submitted', 'quoted'].includes(selected.job.status)}><Search size={14} />{candidateLoading ? '正在发现' : '发现报价'}</button></header>{candidates?.candidates.map((candidate) => <div className={styles.candidate} key={candidate.price_snapshot.snapshot_id}><div><strong>{candidate.provider.display_name}</strong><span>{candidate.provider.provider_kind} · {candidate.provider.trust_tier}</span></div><div><strong>{formatAmount(candidate.price_snapshot.consumer_max_amount_micros, candidate.price_snapshot.currency)}</strong><span>至 {formatTime(candidate.price_snapshot.expires_at)}</span></div><button type="button" onClick={() => { setError(''); setQuoteCandidate(candidate) }}>锁定</button></div>)}{candidates && !candidates.candidates.length && <div className={styles.empty}>没有满足当前合同的报价</div>}</section><ComputeReservationPanel job={selected} candidate={reservationCandidate} onJobChanged={jobChanged} /></> : <div className={styles.empty}>选择一个 Job 查看合同</div>}</div></section>{createOpen && <CreateComputeJobDialog busy={busy} error={error} onClose={() => setCreateOpen(false)} onSubmit={create} />}{quoteCandidate && <QuoteComputeJobDialog candidate={quoteCandidate} busy={busy} error={error} onClose={() => setQuoteCandidate(null)} onSubmit={quote} />}</main>
}

function statusLabel(value: string) { return ({ submitted: '待锁价', quoted: '已锁价', reserved: '已预留', running: '运行中', verification_pending: '待验证', settled: '已结算', failed: '失败', canceled: '已取消' } as Record<string, string>)[value] ?? value }
function formatAmount(value: number, currency: string) { return `${currency} ${(value / 1_000_000).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 22 ? value : `${value.slice(0, 11)}…${value.slice(-7)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
