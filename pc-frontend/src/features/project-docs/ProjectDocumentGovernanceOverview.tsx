import { Filter, PencilLine, Search, ShieldCheck, Tags, X } from 'lucide-react'
import { useMemo, useState, type FormEvent } from 'react'

import {
  AUTHORITY_OPTIONS,
  DOCUMENT_TYPE_OPTIONS,
  LIFECYCLE_OPTIONS,
  RETRIEVAL_OPTIONS,
  effectiveGovernanceFacets,
  facetLabel,
  type DocumentGovernanceFacets,
} from './projectDocumentGovernance'
import type { DocumentCatalog, ProjectDocumentEntry } from './projectDocumentModel'
import { customSectionKey, type DocumentSectionManifest } from './projectDocumentSections'
import styles from './ProjectDocumentGovernanceOverview.module.css'

interface Props {
  catalog: DocumentCatalog | null
  manifest: DocumentSectionManifest
  canEdit: boolean
  onOpenDocument: (path: string) => void
  onSave: (path: string, facets: DocumentGovernanceFacets, secondaryTopics: string[]) => Promise<void>
}

interface Filters {
  query: string
  retrieval: string
  lifecycle: string
  authority: string
  documentType: string
}

const emptyFilters: Filters = { query: '', retrieval: '', lifecycle: '', authority: '', documentType: '' }

export default function ProjectDocumentGovernanceOverview({ catalog, manifest, canEdit, onOpenDocument, onSave }: Props) {
  const [filters, setFilters] = useState(emptyFilters)
  const [editing, setEditing] = useState<ProjectDocumentEntry | null>(null)
  const documents = catalog?.documents ?? []
  const rows = useMemo(() => documents.map((document) => ({
    document,
    facets: effectiveGovernanceFacets(document, manifest.governance_facets[normalizePath(document.path)]),
  })), [documents, manifest.governance_facets])
  const filtered = useMemo(() => rows.filter(({ document, facets }) => {
    const query = filters.query.trim().toLowerCase()
    return (!query || `${document.title} ${document.path}`.toLowerCase().includes(query))
      && (!filters.retrieval || facets.retrieval === filters.retrieval)
      && (!filters.lifecycle || facets.lifecycle === filters.lifecycle)
      && (!filters.authority || facets.authority === filters.authority)
      && (!filters.documentType || facets.document_type === filters.documentType)
  }), [filters, rows])
  const count = (field: keyof DocumentGovernanceFacets, value: string) => rows.filter((row) => row.facets[field] === value).length
  return (
    <main className={styles.overview}>
      <header className={styles.hero}>
        <span><ShieldCheck size={22} /></span>
        <div><small>主题与治理分离 · 路径权威上限受保护</small><h1>多维文档治理</h1>
          <p>快捷分区仍可使用，但底层分别保存检索策略、生命周期、权威性和文档类型。</p></div>
      </header>
      <section className={styles.metrics}>
        <article><span>必须读取</span><strong>{count('retrieval', 'required')}</strong></article>
        <article><span>当前有效</span><strong>{count('lifecycle', 'active') + count('lifecycle', 'accepted')}</strong></article>
        <article><span>权威事实</span><strong>{count('authority', 'binding') + count('authority', 'authoritative')}</strong></article>
        <article><span>需要确认</span><strong>{count('authority', 'unknown') + count('lifecycle', 'unclassified')}</strong></article>
      </section>
      <section className={styles.filters}>
        <label className={styles.search}><Search size={14} /><input value={filters.query} onChange={(event) => setFilters({ ...filters, query: event.target.value })} placeholder="搜索标题或路径" /></label>
        <FacetSelect label="检索" value={filters.retrieval} options={RETRIEVAL_OPTIONS} onChange={(retrieval) => setFilters({ ...filters, retrieval })} />
        <FacetSelect label="生命周期" value={filters.lifecycle} options={LIFECYCLE_OPTIONS} onChange={(lifecycle) => setFilters({ ...filters, lifecycle })} />
        <FacetSelect label="权威性" value={filters.authority} options={AUTHORITY_OPTIONS} onChange={(authority) => setFilters({ ...filters, authority })} />
        <FacetSelect label="类型" value={filters.documentType} options={DOCUMENT_TYPE_OPTIONS} onChange={(documentType) => setFilters({ ...filters, documentType })} />
        <button type="button" disabled={Object.values(filters).every((value) => !value)} onClick={() => setFilters(emptyFilters)}><X size={13} />清除</button>
      </section>
      <div className={styles.resultMeta}><Filter size={13} />显示 {filtered.length} / {rows.length} 份文档</div>
      <section className={styles.table}>
        <header><span>文档与主题</span><span>检索</span><span>生命周期</span><span>权威性</span><span>类型</span><span /></header>
        {filtered.map(({ document, facets }) => {
          const path = normalizePath(document.path)
          const primary = topicLabel(manifest, manifest.assignments[path])
          const secondary = (manifest.secondary_assignments[path] ?? []).map((topic) => topicLabel(manifest, topic)).filter(Boolean)
          return <article key={path}>
            <button className={styles.document} type="button" onClick={() => onOpenDocument(path)}>
              <strong>{document.title}</strong><small>{path}</small>
              <em><Tags size={11} />{primary || '自动主题'}{secondary.length ? ` · +${secondary.length} 副主题` : ''}</em>
            </button>
            <span data-tone={facets.retrieval}>{facetLabel(RETRIEVAL_OPTIONS, facets.retrieval)}</span>
            <span data-tone={facets.lifecycle}>{facetLabel(LIFECYCLE_OPTIONS, facets.lifecycle)}</span>
            <span data-tone={facets.authority}>{facetLabel(AUTHORITY_OPTIONS, facets.authority)}</span>
            <span>{facetLabel(DOCUMENT_TYPE_OPTIONS, facets.document_type)}</span>
            <button className={styles.edit} type="button" disabled={!canEdit} title="编辑多维治理属性" onClick={() => setEditing(document)}><PencilLine size={14} /></button>
          </article>
        })}
        {!filtered.length && <p className={styles.empty}>没有符合当前交叉条件的文档。</p>}
      </section>
      {editing && <GovernanceEditor document={editing} manifest={manifest}
        onClose={() => setEditing(null)} onSave={async (facets, topics) => {
          await onSave(editing.path, facets, topics)
          setEditing(null)
        }} />}
    </main>
  )
}

function FacetSelect({ label, value, options, onChange }: { label: string; value: string; options: Array<{ key: string; label: string }>; onChange: (value: string) => void }) {
  return <label><span>{label}</span><select value={value} onChange={(event) => onChange(event.target.value)}><option value="">全部</option>
    {options.map((option) => <option key={option.key} value={option.key}>{option.label}</option>)}</select></label>
}

function GovernanceEditor({ document, manifest, onClose, onSave }: {
  document: ProjectDocumentEntry
  manifest: DocumentSectionManifest
  onClose: () => void
  onSave: (facets: DocumentGovernanceFacets, secondaryTopics: string[]) => Promise<void>
}) {
  const path = normalizePath(document.path)
  const effective = effectiveGovernanceFacets(document, manifest.governance_facets[path])
  const [facets, setFacets] = useState<DocumentGovernanceFacets>(effective)
  const [secondary, setSecondary] = useState(manifest.secondary_assignments[path] ?? [])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  async function submit(event: FormEvent) {
    event.preventDefault(); setBusy(true); setError('')
    try { await onSave(facets, secondary) } catch (reason) { setError(reason instanceof Error ? reason.message : '保存治理属性失败'); setBusy(false) }
  }
  return <div className={styles.backdrop} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}>
    <form className={styles.dialog} role="dialog" aria-modal="true" onSubmit={submit}>
      <header><div><strong>编辑多维治理属性</strong><small>{path}</small></div><button type="button" onClick={onClose}><X size={15} /></button></header>
      <p>路径规则仍是权威上限。将归档或草稿设置为“当前/必须”时，程序会自动保持较低级别。</p>
      <div className={styles.formGrid}>
        <EditorSelect label="检索策略" value={facets.retrieval} options={RETRIEVAL_OPTIONS} onChange={(retrieval) => setFacets({ ...facets, retrieval: retrieval as DocumentGovernanceFacets['retrieval'] })} />
        <EditorSelect label="生命周期" value={facets.lifecycle} options={LIFECYCLE_OPTIONS} onChange={(lifecycle) => setFacets({ ...facets, lifecycle: lifecycle as DocumentGovernanceFacets['lifecycle'] })} />
        <EditorSelect label="权威性" value={facets.authority} options={AUTHORITY_OPTIONS} onChange={(authority) => setFacets({ ...facets, authority: authority as DocumentGovernanceFacets['authority'] })} />
        <EditorSelect label="文档类型" value={facets.document_type} options={DOCUMENT_TYPE_OPTIONS} onChange={(document_type) => setFacets({ ...facets, document_type })} />
      </div>
      <fieldset><legend>副主题（主要主题：{topicLabel(manifest, manifest.assignments[path]) || '自动判断'}）</legend>
        <div className={styles.topicOptions}>{manifest.sections.map((section) => {
          const key = customSectionKey(section.id)
          if (key === manifest.assignments[path]) return null
          return <label key={key}><input type="checkbox" checked={secondary.includes(key)} onChange={(event) => setSecondary(event.target.checked ? [...secondary, key].slice(0, 12) : secondary.filter((topic) => topic !== key))} />{section.label}</label>
        })}</div>
      </fieldset>
      {error && <div className={styles.error}>{error}</div>}
      <footer><button type="button" onClick={onClose}>取消</button><button type="submit" disabled={busy}>{busy ? '保存中…' : '保存治理属性'}</button></footer>
    </form>
  </div>
}

function EditorSelect({ label, value, options, onChange }: { label: string; value: string; options: Array<{ key: string; label: string; detail: string }>; onChange: (value: string) => void }) {
  return <label><span>{label}</span><select value={value} onChange={(event) => onChange(event.target.value)}>{options.map((option) => <option key={option.key} value={option.key}>{option.label} — {option.detail}</option>)}</select></label>
}

function topicLabel(manifest: DocumentSectionManifest, key?: string) {
  return manifest.sections.find((section) => customSectionKey(section.id) === key)?.label ?? ''
}

function normalizePath(value: string) { return value.trim().replace(/\\/g, '/') }
