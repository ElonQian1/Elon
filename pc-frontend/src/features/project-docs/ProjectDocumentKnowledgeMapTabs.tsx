import { Boxes, FolderTree, Workflow } from 'lucide-react'

import type { ProjectKnowledgeMapView } from './projectDocumentKnowledgeGraphModel'
import styles from './ProjectDocumentCapabilityMap.module.css'

interface Props {
  value: ProjectKnowledgeMapView
  onChange: (view: ProjectKnowledgeMapView) => void
}

const views: Array<{ id: ProjectKnowledgeMapView; label: string; detail: string; icon: typeof Boxes }> = [
  { id: 'capabilities', label: '产品功能', detail: '用户能做什么', icon: Boxes },
  { id: 'architecture', label: '技术架构', detail: '系统怎样实现', icon: Workflow },
  { id: 'topics', label: '文档主题', detail: '文档讲什么', icon: FolderTree },
]

export default function ProjectDocumentKnowledgeMapTabs({ value, onChange }: Props) {
  return (
    <nav className={styles.mapTabs} aria-label="项目知识图谱视图">
      {views.map((view) => {
        const Icon = view.icon
        return (
          <button key={view.id} type="button" data-active={value === view.id || undefined} onClick={() => onChange(view.id)}>
            <Icon size={14} />
            <span><strong>{view.label}</strong><small>{view.detail}</small></span>
          </button>
        )
      })}
    </nav>
  )
}
