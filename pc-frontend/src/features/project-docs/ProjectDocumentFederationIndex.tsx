import { ChevronDown, ChevronRight } from 'lucide-react'
import { useMemo, useState } from 'react'

import type { DocumentHealthAnalysis } from './projectDocumentModel'
import styles from './ProjectDocumentHealthCenter.module.css'

type FederationNode = DocumentHealthAnalysis['federation']['nodes'][number]
const PAGE_SIZE = 8

export default function ProjectDocumentFederationIndex({ nodes }: { nodes: FederationNode[] }) {
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set())
  const [visibleCounts, setVisibleCounts] = useState<Record<string, number>>({ '': PAGE_SIZE })
  const children = useMemo(() => nodes.reduce<Record<string, FederationNode[]>>((output, node) => {
    const parent = node.parent_id || ''
    ;(output[parent] ??= []).push(node)
    return output
  }, {}), [nodes])

  function toggle(id: string) {
    setExpanded((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id); else next.add(id)
      return next
    })
    setVisibleCounts((current) => ({ ...current, [id]: current[id] ?? PAGE_SIZE }))
  }

  function branch(parentId: string, depth: number) {
    const all = children[parentId] ?? []
    const visible = all.slice(0, visibleCounts[parentId] ?? PAGE_SIZE)
    return <>
      {visible.map((node) => {
        const hasChildren = (children[node.id]?.length ?? 0) > 0 || node.direct_children > 0
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
      {visible.length < all.length && <button className={styles.moreNodes} type="button" onClick={() => setVisibleCounts((current) => ({
        ...current, [parentId]: (current[parentId] ?? PAGE_SIZE) + PAGE_SIZE,
      }))}>加载下一页（{all.length - visible.length}）</button>}
    </>
  }

  return <div className={styles.nodes} data-pagination="lazy">
    <small>分页惰性展开 · 每页 {PAGE_SIZE} 个节点</small>
    {branch('', 0)}
  </div>
}
