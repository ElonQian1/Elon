import { ChevronDown, ChevronRight, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'

import { api } from '../../api/client'
import { projectDocumentErrorMessage } from './projectDocumentWorkspaceHelpers'
import {
  acceptFederationPage, beginFederationPage, rejectFederationPage,
  type FederationPage, type FederationPagingState,
} from './projectDocumentFederationPaging'
import styles from './ProjectDocumentHealthCenter.module.css'

const PAGE_SIZE = 8

export default function ProjectDocumentFederationIndex({ projectId }: { projectId: string }) {
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set())
  const [branches, setBranches] = useState<FederationPagingState>({})
  const branchesRef = useRef<FederationPagingState>({})
  const requestSequence = useRef(0)

  const load = useCallback(async (parentId: string, append = false) => {
    const requestId = ++requestSequence.current
    const cursor = append ? branchesRef.current[parentId]?.nextCursor ?? null : null
    setBranches((current) => {
      const next = beginFederationPage(current, parentId, requestId, append)
      branchesRef.current = next
      return next
    })
    const search = new URLSearchParams({ parent_id: parentId, limit: String(PAGE_SIZE) })
    if (cursor) search.set('cursor', cursor)
    try {
      const page = await api.get<FederationPage>(
        `/api/projects/${encodeURIComponent(projectId)}/docs/federation?${search}`,
      )
      setBranches((current) => {
        const next = acceptFederationPage(current, parentId, requestId, page, append)
        branchesRef.current = next
        return next
      })
    } catch (error) {
      setBranches((current) => {
        const next = rejectFederationPage(
          current, parentId, requestId, projectDocumentErrorMessage(error, '读取联邦节点失败'),
        )
        branchesRef.current = next
        return next
      })
    }
  }, [projectId])

  useEffect(() => { branchesRef.current = {}; setBranches({}); setExpanded(new Set()); void load('') }, [load])

  function toggle(id: string) {
    setExpanded((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id); else next.add(id)
      return next
    })
    if (!branches[id]) void load(id)
  }

  function branch(parentId: string, depth: number): ReactNode {
    const state = branches[parentId]
    return <>
      {state?.nodes.map((node) => {
        const hasChildren = node.direct_children > 0
        const isExpanded = expanded.has(node.id)
        return <div key={node.id}>
          <article style={{ marginLeft: Math.min(48, depth * 12) }}>
            <button type="button" aria-label={`${isExpanded ? '收起' : '展开'} ${node.label}`} disabled={!hasChildren} onClick={() => toggle(node.id)}>
              {hasChildren ? isExpanded ? <ChevronDown size={13} /> : <ChevronRight size={13} /> : <span>·</span>}
            </button>
            <span><strong>{node.label}</strong><small>{node.scope_path || '项目根'}</small></span>
            <em>{node.document_count} · {node.score}</em>
          </article>
          {isExpanded && branch(node.id, depth + 1)}
        </div>
      })}
      {state?.loading && <small>正在读取这一页…</small>}
      {state?.error && <button className={styles.moreNodes} type="button" onClick={() => void load(parentId)}><RefreshCw size={12} />{state.error}，重试</button>}
      {state?.hasMore && !state.loading && <button className={styles.moreNodes} type="button" onClick={() => void load(parentId, true)}>加载下一页（已载入 {state.nodes.length}/{state.total}）</button>}
    </>
  }

  return <div className={styles.nodes} data-pagination="server">
    <small>服务端分页惰性展开 · 每个父节点独立翻页</small>
    {branch('', 0)}
  </div>
}
