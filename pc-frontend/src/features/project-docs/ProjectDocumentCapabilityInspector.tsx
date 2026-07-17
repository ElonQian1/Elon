import { ArrowRight, BookOpenText, CircleAlert, FileText, Sparkles } from 'lucide-react'

import { capabilityStatusLabel, type ProjectCapabilityNode } from './projectDocumentCapabilityGraph'
import { roleLabel } from './projectDocumentModel'
import styles from './ProjectDocumentCapabilityMap.module.css'

interface Props {
  node: ProjectCapabilityNode
  canStartAi: boolean
  organizing: boolean
  onOpenDocument: (path: string) => void
  onOpenSection: (sectionId: string) => void
  onAiOrganize: (node: ProjectCapabilityNode) => void
}

export default function ProjectDocumentCapabilityInspector({
  node,
  canStartAi,
  organizing,
  onOpenDocument,
  onOpenSection,
  onAiOrganize,
}: Props) {
  return (
    <aside className={styles.inspector} aria-label="功能节点详情">
      <header>
        <span className={styles.inspectorIcon} style={{ background: node.color }}><BookOpenText size={17} /></span>
        <div><small>{node.isRoot ? '项目能力总览' : '功能节点'}</small><strong>{node.label}</strong></div>
        <em data-status={node.status}>{capabilityStatusLabel(node.status)}</em>
      </header>
      <p className={styles.inspectorDescription}>{node.detail}</p>

      <section className={styles.coveragePanel}>
        <div className={styles.panelHeading}><strong>文档覆盖</strong><span>{node.documentCount} 份</span></div>
        <div className={styles.coverageGrid}>
          {node.coverage.map((item) => (
            <span key={item.key} data-covered={item.covered || undefined}>
              <i />{item.label}<b>{item.count}</b>
            </span>
          ))}
        </div>
        {node.missingCoverage.length > 0 && (
          <p className={styles.coverageGap}><CircleAlert size={13} />建议补齐：{node.missingCoverage.join('、')}</p>
        )}
      </section>

      <section className={styles.entryPanel}>
        <div className={styles.panelHeading}><strong>{node.entrypointSource === 'configured' ? '正式入口文档' : '推荐入口文档'}</strong></div>
        {node.entrypoint ? (
          <button type="button" onClick={() => onOpenDocument(node.entrypoint)}>
            <FileText size={14} /><span>{node.entrypoint}</span><ArrowRight size={13} />
          </button>
        ) : <p>还没有可作为入口的 Markdown 文档。</p>}
        {node.entrypointSource === 'inferred' && <small>由权威性、生命周期和文档角色推断，尚未写入分区配置。</small>}
      </section>

      <section className={styles.documentPanel}>
        <div className={styles.panelHeading}><strong>对应 Markdown</strong><span>最多显示 8 份</span></div>
        <div className={styles.documentList}>
          {node.documents.slice(0, 8).map((document) => (
            <button type="button" key={document.path} onClick={() => onOpenDocument(document.path)}>
              <span><strong>{document.title}</strong><small>{document.path}</small></span>
              <em>{roleLabel(document.metadata.role)}</em>
            </button>
          ))}
          {!node.documents.length && <p>这个能力还没有对应文档。</p>}
        </div>
      </section>

      <div className={styles.inspectorActions}>
        {!node.isRoot && <button type="button" onClick={() => onOpenSection(node.id)}>打开文档分区</button>}
        <button
          className={styles.aiCapabilityButton}
          type="button"
          disabled={!canStartAi || organizing}
          onClick={() => onAiOrganize(node)}
        >
          <Sparkles size={14} />{organizing ? '正在创建任务…' : node.status === 'healthy' ? '让 AI 检查此功能' : '让 AI 补齐此功能'}
        </button>
      </div>
    </aside>
  )
}
