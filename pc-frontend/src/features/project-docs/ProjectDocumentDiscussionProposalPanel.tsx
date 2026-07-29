import { FileCheck2, GitBranchPlus, Sparkles, X } from 'lucide-react'
import { useMemo } from 'react'

import {
  discussionKindLabel,
  discussionStatusLabel,
  type DiscussionGraph,
} from './projectDocumentDiscussionModel'
import {
  discussionProposalDiff,
  type DiscussionGraphProposalView,
} from './projectDocumentDiscussionProposal'
import styles from './ProjectDocumentDiscussionMap.module.css'

interface Props {
  currentGraph: DiscussionGraph
  proposal: DiscussionGraphProposalView
  busy: boolean
  canApply: boolean
  onApply: () => void
  onClose: () => void
}

export default function ProjectDocumentDiscussionProposalPanel({
  currentGraph,
  proposal,
  busy,
  canApply,
  onApply,
  onClose,
}: Props) {
  const diff = useMemo(() => discussionProposalDiff(currentGraph, proposal), [currentGraph, proposal])
  const visibleNodes = [...diff.newNodes, ...diff.changedNodes].slice(0, 80)
  const sourceChanges = [...diff.newSources, ...diff.changedSources]

  return (
    <div className={styles.dialogBackdrop} role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget && !busy) onClose()
    }}>
      <section className={`${styles.dialog} ${styles.proposalDialog}`} role="dialog" aria-modal="true" aria-labelledby="discussion-proposal-title">
        <header>
          <span><Sparkles size={17} /></span>
          <div><strong id="discussion-proposal-title">AI 讨论图整理建议</strong><small>{proposal.summary || '查看将要应用的结构变化'}</small></div>
          <button type="button" title="关闭" disabled={busy} onClick={onClose}><X size={15} /></button>
        </header>
        <div className={styles.proposalBody}>
          <div className={styles.proposalStats}>
            <article><strong>{diff.newNodes.length}</strong><span>新增节点</span></article>
            <article><strong>{diff.changedNodes.length}</strong><span>更新节点</span></article>
            <article><strong>{diff.newEdges}</strong><span>新增关系</span></article>
            <article><strong>{proposal.promotions.length}</strong><span>晋升文档</span></article>
          </div>
          <p className={styles.proposalMeta}>
            变更类型：{proposal.changeKind || '未说明'} · 执行者：{proposal.actor || '当前 AI'} ·
            普通文档读取 {proposal.documentsRead} 份 · 估算 {proposal.estimatedTokensUsed.toLocaleString()} token
          </p>
          {!!sourceChanges.length && <section>
            <h3>来源编译进度</h3>
            <div className={styles.proposalList}>{sourceChanges.map((source) => (
              <article key={source.id}>
                <GitBranchPlus size={14} />
                <div><strong>{source.title}</strong><small>
                  {source.compilation_status || 'pending'} · {source.processed_chunk_ids.length}/{source.chunk_count || '?'} chunks
                </small></div>
              </article>
            ))}</div>
          </section>}
          <section>
            <h3>节点变化 <em>{visibleNodes.length}{visibleNodes.length < diff.newNodes.length + diff.changedNodes.length ? '+' : ''}</em></h3>
            <div className={styles.proposalList}>
              {visibleNodes.map((node) => <article key={node.id}>
                <GitBranchPlus size={14} />
                <div><strong>{node.title}</strong><small>
                  {discussionKindLabel(node.kind)} · {discussionStatusLabel(node.status)} · {node.source_refs.length} 个来源引用
                </small></div>
              </article>)}
              {!visibleNodes.length && <p>建议没有新增或修改节点。</p>}
            </div>
          </section>
          {!!proposal.promotions.length && <section>
            <h3>拟晋升正式文档</h3>
            <div className={styles.proposalList}>{proposal.promotions.map((promotion) => (
              <article key={promotion.id}>
                <FileCheck2 size={14} />
                <div><strong>{promotion.title}</strong><small>{promotion.documentType || 'document'} · {promotion.path}</small></div>
              </article>
            ))}</div>
          </section>}
        </div>
        <footer className={styles.dialogActions}>
          <button type="button" disabled={busy} onClick={onClose}>暂不应用</button>
          <button type="button" disabled={busy || !canApply} onClick={onApply}>{busy ? '正在创建应用任务…' : '批准并生成新版本'}</button>
        </footer>
      </section>
    </div>
  )
}
