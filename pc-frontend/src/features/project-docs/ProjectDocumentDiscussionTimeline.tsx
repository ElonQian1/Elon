import { AlertTriangle, GitCompareArrows, GitCommitHorizontal, History, RotateCcw, Sparkles } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'

import { nodeApi } from '../node/localNodeApi'
import type { DocumentOrganizationTrackingRuntime } from './projectDocumentOrganizationStatus'
import { sanitizeDiscussionGraph, type DiscussionGraph } from './projectDocumentDiscussionModel'
import styles from './ProjectDocumentDiscussionMap.module.css'

export interface DiscussionVersion {
  commit: string
  created_at: string
  summary: string
  change_kind: string
  actor: string
  graph_revision: string
  counts: Record<string, number>
  changes: DiscussionChangeCounts
}

interface DiscussionChangeCounts {
  nodes_added: number
  nodes_removed: number
  nodes_changed: number
  edges_added: number
  edges_removed: number
  edges_changed: number
  total_changes: number
}

interface ReviewIssue {
  id: string
  severity: 'error' | 'warning' | 'advice'
  title: string
  detail: string
  suggested_action: string
  node_ids: string[]
  auto_fixable: boolean
}

interface Review {
  graph_revision?: string
  health_score: number
  severity_counts: Record<string, number>
  issues: ReviewIssue[]
  safe_repair_count: number
}

interface TraceEvent {
  commit: string
  created_at: string
  summary: string
  event: string
  changed_fields: string[]
  from_status?: string
  to_status?: string
}

interface Props {
  runtime: DocumentOrganizationTrackingRuntime
  activeVersion: DiscussionVersion | null
  selectedNodeId: string
  canStartAi: boolean
  organizing: boolean
  onSelectVersion: (graph: DiscussionGraph, version: DiscussionVersion) => void
  onSelectCurrent: () => void
  onRunAi: (instruction: string) => void
}

export default function ProjectDocumentDiscussionTimeline({
  runtime,
  activeVersion,
  selectedNodeId,
  canStartAi,
  organizing,
  onSelectVersion,
  onSelectCurrent,
  onRunAi,
}: Props) {
  const [versions, setVersions] = useState<DiscussionVersion[]>([])
  const [review, setReview] = useState<Review | null>(null)
  const [comparison, setComparison] = useState<DiscussionChangeCounts | null>(null)
  const [trace, setTrace] = useState<TraceEvent[]>([])
  const [expanded, setExpanded] = useState<'review' | 'trace' | ''>('')
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')

  const request = useCallback(async <T,>(path: string, body: Record<string, unknown>) => {
    const response = await nodeApi<{ ok: boolean; result: T; error?: string }>(
      runtime.adminUrl,
      path,
      { method: 'POST', body: JSON.stringify({ project_root: runtime.projectRoot, ...body }) },
    )
    if (!response.ok) throw new Error(response.error || '讨论图版本操作失败')
    return response.result
  }, [runtime.adminUrl, runtime.projectRoot])

  const loadOverview = useCallback(async () => {
    if (!runtime.enabled || !runtime.projectRoot.trim()) return
    setBusy('overview')
    setError('')
    try {
      const [history, quality] = await Promise.all([
        request<{ versions: DiscussionVersion[] }>('/api/project-docs/discussions/history', { limit: 30 }),
        request<Review>('/api/project-docs/discussions/review', {}),
      ])
      setVersions(history.versions ?? [])
      setReview(quality)
    } catch (reason) {
      setError(message(reason))
    } finally {
      setBusy('')
    }
  }, [request, runtime.enabled, runtime.projectRoot])

  useEffect(() => { void loadOverview() }, [loadOverview])

  async function selectVersion(version: DiscussionVersion) {
    setBusy(version.commit)
    setError('')
    try {
      const snapshot = await request<{ graph: unknown }>('/api/project-docs/discussions/version', { commit: version.commit })
      onSelectVersion(sanitizeDiscussionGraph(snapshot.graph), version)
      const diff = await request<{ counts: DiscussionChangeCounts }>('/api/project-docs/discussions/compare', {
        base_commit: version.commit,
        target_commit: 'HEAD',
      })
      setComparison(diff.counts)
    } catch (reason) {
      setError(message(reason))
    } finally {
      setBusy('')
    }
  }

  async function showTrace() {
    if (!selectedNodeId) return
    setExpanded('trace')
    setBusy('trace')
    setError('')
    try {
      const result = await request<{ events: TraceEvent[] }>('/api/project-docs/discussions/trace', {
        node_id: selectedNodeId,
        limit: 60,
      })
      setTrace(result.events ?? [])
    } catch (reason) {
      setError(message(reason))
    } finally {
      setBusy('')
    }
  }

  function repairWithAi() {
    const issueIds = review?.issues.slice(0, 80).map((issue) => issue.id).join(', ') || ''
    onRunAi(
      `复查并修正当前讨论推理图。先调用 project_discussions_review_graph；当前页面命中问题：${issueIds || '请重新获取'}。`
      + '对 auto_fixable 问题先调用 project_discussions_prepare_safe_repair，将返回的 proposal 原样 save_proposal 后 apply。'
      + '其余语义问题只读取 issue 命中的来源和节点，不扫描全部聊天；不得删除有价值的旧分支，使用 superseded、merged_into、resolves 等关系保留演化过程。'
      + '修正 proposal 的 change_kind=repair，应用后调用 get_history、compare_versions 和 review_graph 确认新版本及剩余问题。',
    )
  }

  if (!runtime.enabled) {
    return <section className={styles.timelineUnavailable}>
      <History size={13} /><span>连接当前项目的 Windows 节点后，可回看脑图版本、语义差异和节点演化。</span>
    </section>
  }

  return <section className={styles.timeline}>
    <header>
      <button type="button" data-active={!activeVersion || undefined} onClick={() => { onSelectCurrent(); setComparison(null) }}>
        <RotateCcw size={12} /><span>当前</span>
      </button>
      <div className={styles.versionTrack}>
        {versions.map((version) => <button key={version.commit} type="button"
          data-active={activeVersion?.commit === version.commit || undefined}
          disabled={busy === version.commit} title={`${version.summary}\n${version.commit}`}
          onClick={() => { void selectVersion(version) }}>
          <GitCommitHorizontal size={11} /><span>{version.summary || '脑图版本'}</span>
          <small>{new Date(version.created_at).toLocaleDateString()} · {version.commit.slice(0, 7)}</small>
        </button>)}
        {!versions.length && busy !== 'overview' && <small>还没有脑图版本；首次应用建议后自动生成。</small>}
      </div>
      <button type="button" data-alert={!!review?.issues.length || undefined}
        onClick={() => setExpanded((value) => value === 'review' ? '' : 'review')}>
        <AlertTriangle size={12} /><span>{review ? `${review.health_score} 分` : '质量'}</span>
      </button>
      <button type="button" disabled={!selectedNodeId || busy === 'trace'} onClick={() => { void showTrace() }}>
        <History size={12} /><span>节点演化</span>
      </button>
    </header>
    {activeVersion && <div className={styles.versionBanner}>
      <GitCompareArrows size={12} />
      <span>正在只读回看：{activeVersion.summary}（{activeVersion.commit.slice(0, 8)}）</span>
      {comparison && <small>
        与当前相比：+{comparison.nodes_added} 节点 / -{comparison.nodes_removed} / {comparison.nodes_changed} 项变化
      </small>}
      <button type="button" onClick={onSelectCurrent}>返回当前版</button>
    </div>}
    {expanded === 'review' && <div className={styles.timelinePanel}>
      <header><strong>脑图治理建议</strong><span>{review?.severity_counts.error ?? 0} 错误 · {review?.severity_counts.warning ?? 0} 警告 · {review?.safe_repair_count ?? 0} 项可确定性修正</span>
        <button type="button" disabled={!canStartAi || organizing} onClick={repairWithAi}><Sparkles size={12} />让 Win 端 AI 修正并发布新版本</button></header>
      <div>{review?.issues.slice(0, 8).map((issue) => <article key={issue.id} data-severity={issue.severity}>
        <strong>{issue.title}</strong><span>{issue.detail}</span><small>{issue.suggested_action}</small>
      </article>)}
        {!review?.issues.length && <p>确定性检查没有发现问题；仍可让 AI 做有来源约束的语义复查。</p>}
      </div>
    </div>}
    {expanded === 'trace' && <div className={styles.timelinePanel}>
      <header><strong>节点生命周期</strong><span>{selectedNodeId} · {trace.length} 个变化事件</span></header>
      <div>{trace.map((event, index) => <article key={`${event.commit}-${index}`}>
        <strong>{eventLabel(event.event)}</strong><span>{event.summary}</span>
        <small>{event.created_at ? new Date(event.created_at).toLocaleString() : '当前工作副本'}
          {event.to_status ? ` · ${event.from_status || '无'} → ${event.to_status}` : ''}
          {event.changed_fields.length ? ` · ${event.changed_fields.join(', ')}` : ''}</small>
      </article>)}
        {!trace.length && busy !== 'trace' && <p>该节点还没有可见的版本变化。</p>}
      </div>
    </div>}
    {error && <p className={styles.timelineError}>{error}</p>}
  </section>
}

function eventLabel(event: string) {
  return {
    observed: '历史中已存在',
    created: '创建节点',
    updated: '更新节点',
    removed: '移出当前图',
    relations_changed: '关系变化',
  }[event] ?? event
}

function message(reason: unknown) {
  return reason instanceof Error ? reason.message : '讨论图版本操作失败'
}
