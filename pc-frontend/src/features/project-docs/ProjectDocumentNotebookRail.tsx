import { ArrowLeft, BookOpen, FolderPlus, ShieldCheck, Trash2 } from 'lucide-react'

import { formatNumber, type DocumentBudget } from './projectDocumentModel'
import type { DocumentSection } from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  projectName: string
  sections: DocumentSection[]
  activeSection: string
  counts: Record<string, number>
  budget?: DocumentBudget
  canEdit: boolean
  onBack: () => void
  onSelect: (section: string) => void
  onCreate: () => void
  onRemove: (section: DocumentSection) => void
}

export default function ProjectDocumentNotebookRail({
  projectName,
  sections,
  activeSection,
  counts,
  budget,
  canEdit,
  onBack,
  onSelect,
  onCreate,
  onRemove,
}: Props) {
  return (
    <aside className={styles.notebookRail}>
      <button className={styles.backButton} type="button" onClick={onBack}>
        <ArrowLeft size={17} aria-hidden="true" />
        <span>返回项目频道</span>
      </button>

      <div className={styles.notebookIdentity}>
        <span className={styles.notebookIcon}><BookOpen size={20} aria-hidden="true" /></span>
        <span>
          <strong>{projectName}</strong>
          <small>项目知识库</small>
        </span>
        <button
          className={styles.addSectionButton}
          type="button"
          title="新建自定义分区"
          disabled={!canEdit}
          onClick={onCreate}
        >
          <FolderPlus size={16} aria-hidden="true" />
        </button>
      </div>

      <div className={styles.sectionList}>
        {sections.map((section) => (
          <div className={styles.sectionRow} key={section.key}>
            <button
              className={[styles.sectionButton, activeSection === section.key ? styles.sectionActive : ''].join(' ')}
              type="button"
              onClick={() => onSelect(section.key)}
            >
              <span className={styles.sectionColor} style={{ background: section.color }} />
              <span className={styles.sectionCopy}>
                <strong>{section.label}</strong>
                <small>{section.detail}</small>
              </span>
              <em>{counts[section.key] ?? 0}</em>
            </button>
            {section.custom && canEdit && (
              <button
                className={styles.removeSectionButton}
                type="button"
                title={`删除分区 ${section.label}`}
                onClick={() => onRemove(section)}
              >
                <Trash2 size={12} aria-hidden="true" />
              </button>
            )}
          </div>
        ))}
      </div>

      {budget && (
        <div className={styles.budgetCard}>
          <span><ShieldCheck size={15} aria-hidden="true" /> 程序预分类</span>
          <strong>{budget.classification_model_tokens} 模型 token</strong>
          <small>
            仅表示目录预分类；调用 AI 建议会另外消耗 token。<br />
            默认排除 {budget.excluded_by_default} 份，预计少读 {formatNumber(budget.estimated_tokens_avoided)} token
          </small>
        </div>
      )}
    </aside>
  )
}
