import { Activity, ArrowRight, CircleAlert, FilePenLine, Sparkles } from 'lucide-react'

import type { DocumentCatalog } from './projectDocumentModel'
import type { DocumentOrganizationSuggestions } from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  catalog: DocumentCatalog | null
  unclassified: number
  suggestions: DocumentOrganizationSuggestions | null
  onOpenSuggestions: () => void
}

export default function ProjectDocumentHealthSummary({ catalog, unclassified, suggestions, onOpenSuggestions }: Props) {
  if (!catalog) return null
  const namingIssues = catalog.documents.filter((document) => poorlyNamed(document.path)).length
  const proposedOperations = suggestions?.file_operations.filter((operation) => operation.status === 'proposed').length ?? 0
  const conflicts = suggestions?.conflicts.length ?? 0
  const next = catalog.access?.degraded
    ? '先恢复项目绑定的 PC 节点，再编辑或执行实体整理。'
    : suggestions?.status === 'ready'
      ? '审核 AI 建议：先应用虚拟分区，再逐项确认实体整理。'
      : unclassified > 0 || namingIssues > 0
        ? '生成 AI 整理建议，优先处理等待整理和命名含糊的文档。'
        : '文档结构稳定；新增笔记继续放入笔记区并定期复查。'
  return (
    <details className={styles.healthSummary}>
      <summary><Activity size={14} aria-hidden="true" /><strong>项目文档健康</strong><span>{unclassified || conflicts ? '需要处理' : '结构稳定'}</span></summary>
      <div className={styles.healthMetrics}>
        <span><b>{catalog.documents.length}</b> 文档总数</span>
        <span><CircleAlert size={12} /><b>{unclassified}</b> 等待整理</span>
        <span><FilePenLine size={12} /><b>{Math.max(namingIssues, proposedOperations)}</b> 命名/路径建议</span>
        <span><Sparkles size={12} /><b>{conflicts}</b> 权威冲突</span>
      </div>
      <p>{next}</p>
      <button type="button" onClick={onOpenSuggestions}>查看整理建议<ArrowRight size={13} /></button>
    </details>
  )
}

function poorlyNamed(path: string) {
  const name = path.replace(/\\/g, '/').split('/').pop()?.replace(/\.(md|markdown|mdown)$/i, '').trim().toLowerCase() ?? ''
  return !name || /^(\d+|new|note|notes|todo|untitled|readme[-_ ]?\d+)$/.test(name)
}
