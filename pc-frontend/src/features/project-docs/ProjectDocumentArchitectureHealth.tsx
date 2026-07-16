import { Activity, ArrowRight, CircleAlert, Layers3 } from 'lucide-react'

import type { KnowledgeArchitectureHealth } from './projectDocumentArchitecture'
import styles from './ProjectDocumentKnowledge.module.css'

interface Props {
  health: KnowledgeArchitectureHealth
  onOpenHome: () => void
  onOpenSuggestions: () => void
}

export default function ProjectDocumentArchitectureHealth({ health, onOpenHome, onOpenSuggestions }: Props) {
  const label = health.status === 'healthy' ? '结构良好' : health.status === 'needs_attention' ? '建议完善' : '需要架构'
  return (
    <details className={styles.healthCard}>
      <summary><Activity size={14} /><strong>知识架构健康</strong><span>{health.score} · {label}</span></summary>
      <div className={styles.healthMetrics}>
        <span><Layers3 size={12} /><b>{health.topicAssigned}</b> 显式归类</span>
        <span><CircleAlert size={12} /><b>{health.topicAutomatic}</b> 自动归类</span>
        <span><b>{health.missingDocumentTypes.length}</b> 基础文档缺口</span>
        <span><b>{health.ambiguous}</b> 权威性待确认</span>
      </div>
      <p>{health.findings[0] || '当前结构满足项目模板的基础要求。'}</p>
      <div className={styles.healthActions}>
        <button type="button" onClick={onOpenHome}>知识首页<ArrowRight size={12} /></button>
        <button type="button" onClick={onOpenSuggestions}>AI 建议<ArrowRight size={12} /></button>
      </div>
    </details>
  )
}
