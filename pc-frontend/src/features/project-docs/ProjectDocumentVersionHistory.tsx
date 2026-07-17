import { Eye, GitCommitHorizontal, History, RotateCcw, X } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'

import { nodeApi } from '../node/localNodeApi'
import type { DocumentOrganizationTrackingRuntime } from './projectDocumentOrganizationStatus'
import styles from './ProjectDocumentHealthCenter.module.css'

interface Version {
  commit: string
  created_at: string
  summary: string
  changed_paths: string[]
  document_only: boolean
  reversible: boolean
  mode: 'managed_snapshot' | 'document_commit'
}

interface Props {
  runtime: DocumentOrganizationTrackingRuntime
  onRestored: () => void
}

export default function ProjectDocumentVersionHistory({ runtime, onRestored }: Props) {
  const [versions, setVersions] = useState<Version[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [diff, setDiff] = useState<{ commit: string; content: string; truncated: boolean } | null>(null)
  const [busy, setBusy] = useState('')
  const request = useCallback(async <T,>(path: string, body: Record<string, unknown>) => {
    const response = await nodeApi<{ ok: boolean; result: T; error?: string }>(runtime.adminUrl, path, {
      method: 'POST', body: JSON.stringify({ project_root: runtime.projectRoot, ...body }),
    })
    if (!response.ok) throw new Error(response.error || '文档版本操作失败')
    return response.result
  }, [runtime.adminUrl, runtime.projectRoot])
  const load = useCallback(async () => {
    if (!runtime.enabled || !runtime.projectRoot.trim()) return
    setLoading(true); setError('')
    try {
      const result = await request<{ versions: Version[] }>('/api/project-docs/governance/history', { limit: 20 })
      setVersions(result.versions)
    } catch (reason) { setError(message(reason)) } finally { setLoading(false) }
  }, [request, runtime.enabled, runtime.projectRoot])
  useEffect(() => { void load() }, [load])

  async function showDiff(version: Version) {
    setBusy(version.commit); setError('')
    try {
      const result = await request<{ diff: string; truncated: boolean }>('/api/project-docs/governance/diff', { commit: version.commit })
      setDiff({ commit: version.commit, content: result.diff, truncated: result.truncated })
    } catch (reason) { setError(message(reason)) } finally { setBusy('') }
  }

  async function restore(version: Version) {
    if (!version.reversible || !window.confirm(`回滚“${version.summary}”？系统会创建新的恢复提交，不会重写 Git 历史。`)) return
    setBusy(version.commit); setError('')
    try {
      await request('/api/project-docs/governance/restore', { commit: version.commit })
      await load(); onRestored()
    } catch (reason) { setError(message(reason)) } finally { setBusy('') }
  }

  return <section className={styles.panel}>
    <header><div><strong>文档版本与回滚</strong><small>Git 历史 · 新提交恢复</small></div>
      <button type="button" onClick={() => void load()} disabled={loading}><History size={13} />刷新</button></header>
    {!runtime.enabled ? <p className={styles.muted}>连接项目本机节点后可查看差异与安全回滚。</p> : (
      <div className={styles.versionList}>
        {versions.map((version) => <article key={version.commit}>
          <GitCommitHorizontal size={14} />
          <span><strong>{version.summary}</strong><small>{new Date(version.created_at).toLocaleString()} · {version.commit.slice(0, 8)} · {version.changed_paths.length} 个路径</small></span>
          <button type="button" title="查看文档差异" disabled={busy === version.commit} onClick={() => void showDiff(version)}><Eye size={13} /></button>
          <button type="button" title={version.reversible ? '创建恢复提交' : '混合代码提交不能一键回滚'} disabled={!version.reversible || busy === version.commit} onClick={() => void restore(version)}><RotateCcw size={13} /></button>
        </article>)}
        {!versions.length && !loading && <p className={styles.muted}>还没有文档版本记录。</p>}
      </div>
    )}
    {error && <p className={styles.errorText}>{error}</p>}
    {diff && <div className={styles.diffBackdrop} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setDiff(null) }}>
      <section className={styles.diffDialog} role="dialog" aria-modal="true"><header><div><strong>文档提交差异</strong><small>{diff.commit}</small></div><button type="button" onClick={() => setDiff(null)}><X size={14} /></button></header>
        <pre>{diff.content || '该提交没有 Markdown 或知识清单差异。'}</pre>{diff.truncated && <p>差异过长，已安全截断。</p>}</section>
    </div>}
  </section>
}

function message(reason: unknown) { return reason instanceof Error ? reason.message : '文档版本操作失败' }
