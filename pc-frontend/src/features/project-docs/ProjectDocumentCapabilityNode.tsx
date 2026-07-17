import { ChevronDown, ChevronRight, FileText, FolderTree } from 'lucide-react'
import { Handle, Position, type Node, type NodeProps } from '@xyflow/react'

import {
  capabilityStatusLabel,
  type ProjectCapabilityNode,
} from './projectDocumentCapabilityGraph'
import styles from './ProjectDocumentCapabilityMap.module.css'

export interface CapabilityNodeData extends Record<string, unknown> {
  capability: ProjectCapabilityNode
  collapsed: boolean
  onToggle: (id: string) => void
}

export type CapabilityFlowNode = Node<CapabilityNodeData, 'capability'>

export default function ProjectDocumentCapabilityNode({ data, selected }: NodeProps<CapabilityFlowNode>) {
  const node = data.capability
  return (
    <article
      className={styles.capabilityNode}
      data-root={node.isRoot || undefined}
      data-status={node.status}
      data-selected={selected || undefined}
      style={{ '--node-color': node.color } as React.CSSProperties}
    >
      {!node.isRoot && <Handle className={styles.nodeHandle} type="target" position={Position.Left} />}
      <header>
        <span className={styles.nodeIcon}>{node.isRoot ? <FolderTree size={16} /> : <FileText size={14} />}</span>
        <span className={styles.nodeTitle}>
          <strong>{node.label}</strong>
          <small>{capabilityStatusLabel(node.status)}</small>
        </span>
        {node.childCount > 0 && (
          <button
            type="button"
            title={data.collapsed ? '展开子能力' : '收起子能力'}
            aria-label={data.collapsed ? `展开${node.label}` : `收起${node.label}`}
            onClick={(event) => { event.stopPropagation(); data.onToggle(node.id) }}
          >
            {data.collapsed ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
          </button>
        )}
      </header>
      <p>{node.detail}</p>
      <footer>
        <span>{node.documentCount} 份文档</span>
        <span className={styles.coverageDots} aria-label={`已覆盖 ${node.coverage.filter((item) => item.covered).length} 类文档`}>
          {node.coverage.map((item) => <i key={item.key} data-covered={item.covered || undefined} title={`${item.label}：${item.count}`} />)}
        </span>
        {node.childCount > 0 && <em>{node.childCount} 个子能力</em>}
      </footer>
      {node.childCount > 0 && <Handle className={styles.nodeHandle} type="source" position={Position.Right} />}
    </article>
  )
}
