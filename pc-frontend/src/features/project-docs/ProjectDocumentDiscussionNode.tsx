import { GitBranch, MessageSquareText } from 'lucide-react'
import { Handle, Position, type Node, type NodeProps } from '@xyflow/react'

import {
  discussionKindLabel,
  discussionStatusLabel,
  type DiscussionNode,
} from './projectDocumentDiscussionModel'
import styles from './ProjectDocumentDiscussionMap.module.css'

interface DiscussionNodeData extends Record<string, unknown> {
  discussion: DiscussionNode
  childCount: number
}

export type DiscussionFlowNode = Node<DiscussionNodeData, 'discussion'>

export default function ProjectDocumentDiscussionNode({ data, selected }: NodeProps<DiscussionFlowNode>) {
  const node = data.discussion
  return (
    <article
      className={styles.discussionNode}
      data-selected={selected || undefined}
      data-status={node.status}
      style={{ '--node-color': node.color } as React.CSSProperties}
    >
      {!!node.parent_id && <Handle className={styles.nodeHandle} type="target" position={Position.Left} />}
      <header>
        <span>{node.kind === 'topic' ? <MessageSquareText size={15} /> : <GitBranch size={14} />}</span>
        <div><strong>{node.title}</strong><small>{discussionKindLabel(node.kind)} · {discussionStatusLabel(node.status)}</small></div>
      </header>
      <p>{node.summary || '等待补充摘要与依据。'}</p>
      <footer>
        <span>{node.source_refs.length} 个来源</span>
        <span>{node.document_paths.length} 份文档</span>
        {!!data.childCount && <em>{data.childCount} 个分支</em>}
      </footer>
      {!!data.childCount && <Handle className={styles.nodeHandle} type="source" position={Position.Right} />}
    </article>
  )
}
