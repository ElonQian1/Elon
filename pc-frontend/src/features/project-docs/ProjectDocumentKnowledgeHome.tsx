import { ArrowRight, BookOpenCheck, Bot, CircleAlert, Compass, FileText, Layers3, Sparkles } from 'lucide-react'

import type { DocumentCatalog } from './projectDocumentModel'
import type { DocumentSection, DocumentSectionManifest } from './projectDocumentSections'
import {
  KNOWLEDGE_PROFILE_OPTIONS,
  recommendedStartDocuments,
  type KnowledgeArchitectureHealth,
} from './projectDocumentArchitecture'
import styles from './ProjectDocumentKnowledge.module.css'

interface Props {
  projectName: string
  catalog: DocumentCatalog | null
  manifest: DocumentSectionManifest
  health: KnowledgeArchitectureHealth
  sections: DocumentSection[]
  counts: Record<string, number>
  onOpenDocument: (path: string) => void
  onOpenSection: (section: string) => void
  onOpenSuggestions: () => void
  onProfileChange: (profile: string) => void
}

export default function ProjectDocumentKnowledgeHome({
  projectName,
  catalog,
  manifest,
  health,
  sections,
  counts,
  onOpenDocument,
  onOpenSection,
  onOpenSuggestions,
  onProfileChange,
}: Props) {
  const startDocuments = recommendedStartDocuments(catalog, manifest)
  const topics = sections.filter((section) => !['knowledge-home', 'suggestions'].includes(section.key) && (counts[section.key] ?? 0) > 0)
  const statusLabel = health.status === 'healthy' ? '结构良好' : health.status === 'needs_attention' ? '建议完善' : '需要建立架构'
  return (
    <main className={styles.home}>
      <header className={styles.hero}>
        <div className={styles.heroIcon}><Compass size={27} aria-hidden="true" /></div>
        <div className={styles.heroCopy}>
          <span className={styles.profileLine}>
            <select
              aria-label="项目知识模板"
              title="选择项目知识模板"
              value={health.profile}
              onChange={(event) => onProfileChange(event.target.value)}
            >
              {KNOWLEDGE_PROFILE_OPTIONS.map((profile) => (
                <option key={profile.key} value={profile.key}>{profile.label}</option>
              ))}
            </select>
            <b>{health.profileSource === 'manifest' ? '已固定模板' : '程序推断'}</b>
          </span>
          <h1>{manifest.home.title || projectName}</h1>
          <p>{manifest.home.summary || '这套知识库尚未配置项目摘要。AI 可以根据目录和少量按需阅读生成项目地图、推荐阅读顺序与主题架构。'}</p>
        </div>
        <div className={styles.score} data-status={health.status}>
          <strong>{health.score}</strong><span>/ 100</span><small>{statusLabel}</small>
        </div>
      </header>

      <section className={styles.metrics}>
        <article><FileText size={17} /><span><strong>{catalog?.documents.length ?? 0}</strong>文档</span></article>
        <article><Layers3 size={17} /><span><strong>{topics.length}</strong>主题</span></article>
        <article><BookOpenCheck size={17} /><span><strong>{health.foundations.filter((item) => item.covered).length}/{health.foundations.length}</strong>基础文档</span></article>
        <article><CircleAlert size={17} /><span><strong>{health.topicAutomatic}</strong>程序自动归类</span></article>
      </section>

      <div className={styles.homeGrid}>
        <section className={styles.panel}>
          <header><BookOpenCheck size={18} /><div><strong>从这里开始</strong><small>按推荐顺序理解项目</small></div></header>
          <div className={styles.startList}>
            {startDocuments.map((document, index) => (
              <button key={document.path} type="button" onClick={() => onOpenDocument(document.path)}>
                <i>{index + 1}</i><span><strong>{document.title}</strong><small>{document.path}</small></span><ArrowRight size={14} />
              </button>
            ))}
            {!startDocuments.length && <p className={styles.empty}>还没有推荐阅读入口。运行 AI 架构建议后会自动建立。</p>}
          </div>
        </section>

        <section className={styles.panel}>
          <header><Layers3 size={18} /><div><strong>知识主题</strong><small>按业务领域浏览，而不是按文档状态找文件</small></div></header>
          <div className={styles.topicGrid}>
            {topics.slice(0, 12).map((section) => (
              <button key={section.key} type="button" onClick={() => onOpenSection(section.key)}>
                <i style={{ background: section.color }} /><span><strong>{section.label}</strong><small>{section.detail}</small></span><em>{counts[section.key] ?? 0}</em>
              </button>
            ))}
          </div>
        </section>

        <section className={[styles.panel, styles.diagnostics].join(' ')}>
          <header><Bot size={18} /><div><strong>架构诊断</strong><small>程序先用路径和标题检查，不消耗模型 token</small></div></header>
          <div className={styles.foundationList}>
            {health.foundations.map((foundation) => (
              <span key={foundation.id} data-covered={foundation.covered}><i />{foundation.label}<b>{foundation.covered ? '已有入口' : '缺失'}</b></span>
            ))}
          </div>
          <ul>
            {health.findings.slice(0, 5).map((finding) => <li key={finding}>{finding}</li>)}
            {!health.findings.length && <li>项目类型、知识首页、主题归类和基础文档均已达到当前模板要求。</li>}
          </ul>
          <button className={styles.aiButton} type="button" onClick={onOpenSuggestions}><Sparkles size={15} />查看 AI 架构建议</button>
        </section>
      </div>
    </main>
  )
}
