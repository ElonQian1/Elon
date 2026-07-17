import { Activity, AlertTriangle, GitCommitHorizontal, Network, RefreshCw, Sparkles } from 'lucide-react'

import type { DocumentHealthAnalysis } from './projectDocumentModel'
import styles from './ProjectDocumentHealthCenter.module.css'

interface Props {
  analysis?: DocumentHealthAnalysis
  onRefresh: () => void
  onOpenSuggestions: () => void
}

export default function ProjectDocumentHealthCenter({ analysis, onRefresh, onOpenSuggestions }: Props) {
  if (!analysis) {
    return (
      <main className={styles.center}>
        <section className={styles.empty}>
          <Activity size={28} />
          <h1>文档健康分析尚不可用</h1>
          <p>刷新目录后，服务端会用零模型 token 建立增量索引并运行确定性检查。</p>
          <button type="button" onClick={onRefresh}><RefreshCw size={14} />刷新分析</button>
        </section>
      </main>
    )
  }
  const quality = analysis.quality.summary
  return (
    <main className={styles.center}>
      <header className={styles.hero}>
        <span><Activity size={22} /></span>
        <div>
          <small>服务端统一真源 · 零模型 token 预检</small>
          <h1>项目文档健康中心</h1>
          <p>结构、链接、可发现性、维护责任、复查周期、实现引用和大型项目知识节点集中在这里。</p>
        </div>
        <strong data-status={analysis.overall.status}>{analysis.overall.score}<small>/ 100</small></strong>
      </header>

      <section className={styles.metrics}>
        <article><Activity /><span>结构健康<strong>{analysis.architecture.score}</strong></span></article>
        <article><AlertTriangle /><span>质量问题<strong>{quality.total_issues}</strong></span></article>
        <article><GitCommitHorizontal /><span>本次变更<strong>{analysis.maintenance.changed_documents}</strong></span></article>
        <article><Network /><span>知识节点<strong>{analysis.federation.node_count}</strong></span></article>
      </section>

      <div className={styles.grid}>
        <section className={styles.panel}>
          <header><div><strong>需要处理的问题</strong><small>{quality.errors} 错误 · {quality.warnings} 警告 · {quality.info} 提示</small></div><button type="button" onClick={onRefresh}><RefreshCw size={13} />刷新</button></header>
          <div className={styles.issueList}>
            {analysis.quality.issues.length ? analysis.quality.issues.map((issue) => (
              <article key={issue.fingerprint} data-severity={issue.severity}>
                <i />
                <div><strong>{issue.message}</strong><small>{issue.path}</small><p>{issue.evidence}</p></div>
                <em>{issue.confidence}%</em>
              </article>
            )) : <p className={styles.muted}>当前没有确定性质量问题。</p>}
          </div>
          <button className={styles.aiButton} type="button" onClick={onOpenSuggestions}><Sparkles size={14} />让 AI 根据证据提出整理建议</button>
        </section>

        <aside className={styles.side}>
          <section className={styles.panel}>
            <header><div><strong>持续维护</strong><small>索引 v{analysis.maintenance.index_version}</small></div></header>
            <dl>
              <div><dt>持久事件队列</dt><dd>{analysis.maintenance.durable_queue ? '已启用' : '未启用'}</dd></div>
              <div><dt>待处理事件</dt><dd>{analysis.maintenance.pending_events}</dd></div>
              <div><dt>本轮已处理</dt><dd>{analysis.maintenance.processed_events}</dd></div>
              <div><dt>后台复查</dt><dd>{analysis.maintenance.poll_interval_seconds} 秒</dd></div>
              <div><dt>外链待检查</dt><dd>{quality.external_links_pending}</dd></div>
            </dl>
          </section>
          <section className={styles.panel}>
            <header><div><strong>联邦知识架构</strong><small>{analysis.federation.source === 'manifest' ? '显式清单' : '程序推断'} · 最深 {analysis.federation.max_depth} 层</small></div></header>
            <div className={styles.nodes}>
              {analysis.federation.nodes.map((node) => (
                <article key={node.id} style={{ marginLeft: node.parent_id ? 12 : 0 }}>
                  <span><strong>{node.label}</strong><small>{node.scope_path || '项目根'}</small></span>
                  <em>{node.document_count} · {node.score}</em>
                </article>
              ))}
            </div>
          </section>
        </aside>
      </div>
    </main>
  )
}
