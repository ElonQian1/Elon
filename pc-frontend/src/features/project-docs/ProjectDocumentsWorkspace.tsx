import { useCallback, useEffect, useMemo, useState } from 'react'
import { Bot, FilePlus2, FileText, FolderTree, RefreshCw, Save, Search, Sparkles } from 'lucide-react'

import { api } from '../../api/client'
import MarkdownContent from '../markdown/MarkdownContent'
import ProjectDocumentNotebookRail from './ProjectDocumentNotebookRail'
import ProjectDocumentSuggestions from './ProjectDocumentSuggestions'
import {
  formatNumber,
  lifecycleLabel,
  roleLabel,
  type DocumentCatalog,
  type DocumentFile,
} from './projectDocumentModel'
import {
  buildDocumentSections,
  buildOrganizationPrompt,
  sectionForDocument,
  type DocumentSection,
} from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'
import { useProjectDocumentOrganization } from './useProjectDocumentOrganization'

interface Props {
  projectId: string
  projectName: string
  onBack: () => void
  onStartAiOrganize: (prompt: string) => Promise<void>
  canStartAi: boolean
}

type ViewMode = 'edit' | 'preview' | 'split'
const AUTOMATIC_SECTION = '__automatic__'

export default function ProjectDocumentsWorkspace({
  projectId,
  projectName,
  onBack,
  onStartAiOrganize,
  canStartAi,
}: Props) {
  const [catalog, setCatalog] = useState<DocumentCatalog | null>(null)
  const [catalogLoading, setCatalogLoading] = useState(true)
  const [catalogError, setCatalogError] = useState('')
  const [activeSection, setActiveSection] = useState('required')
  const [query, setQuery] = useState('')
  const [selectedPath, setSelectedPath] = useState('')
  const [document, setDocument] = useState<DocumentFile | null>(null)
  const [draft, setDraft] = useState('')
  const [documentLoading, setDocumentLoading] = useState(false)
  const [saveState, setSaveState] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')
  const [message, setMessage] = useState('')
  const [viewMode, setViewMode] = useState<ViewMode>('split')
  const [organizing, setOrganizing] = useState(false)
  const [applyingSuggestions, setApplyingSuggestions] = useState(false)
  const organization = useProjectDocumentOrganization(projectId)
  const sections = useMemo(() => buildDocumentSections(organization.manifest), [organization.manifest])

  const loadCatalog = useCallback(async () => {
    setCatalogLoading(true)
    setCatalogError('')
    try {
      const response = await api.get<DocumentCatalog>(
        `/api/projects/${encodeURIComponent(projectId)}/docs/catalog`,
      )
      setCatalog(response)
      setSelectedPath((current) => current || response.documents[0]?.path || '')
    } catch (error) {
      setCatalogError(errorMessage(error, '读取项目文档目录失败'))
    } finally {
      setCatalogLoading(false)
    }
  }, [projectId])

  useEffect(() => { loadCatalog() }, [loadCatalog])

  const selectedEntry = useMemo(
    () => catalog?.documents.find((entry) => entry.path === selectedPath),
    [catalog, selectedPath],
  )
  const automaticSectionKey = useMemo(() => {
    if (!selectedEntry) return 'unclassified'
    const assignments = { ...organization.manifest.assignments }
    delete assignments[normalizeDocumentPath(selectedEntry.path)]
    return sectionForDocument(selectedEntry, { ...organization.manifest, assignments })
  }, [organization.manifest, selectedEntry])
  const automaticSectionLabel = sections.find((section) => section.key === automaticSectionKey)?.label ?? '等待整理'
  const dirty = !!document && draft !== document.content

  const openDocument = useCallback(async (path: string) => {
    if (!path) return
    setDocumentLoading(true)
    setMessage('')
    try {
      const response = await api.get<DocumentFile>(
        `/api/projects/${encodeURIComponent(projectId)}/docs/file?path=${encodeURIComponent(path)}`,
      )
      setDocument(response)
      setDraft(response.content)
      setSaveState('idle')
    } catch (error) {
      setDocument(null)
      setDraft('')
      setMessage(errorMessage(error, '读取文档失败'))
    } finally {
      setDocumentLoading(false)
    }
  }, [projectId])

  useEffect(() => {
    if (selectedPath) openDocument(selectedPath)
  }, [openDocument, selectedPath])

  const sectionCounts = useMemo(() => {
    const counts = Object.fromEntries(sections.map((section) => [section.key, 0])) as Record<string, number>
    for (const entry of catalog?.documents ?? []) {
      const section = sectionForDocument(entry, organization.manifest)
      counts[section] = (counts[section] ?? 0) + 1
    }
    counts.suggestions = organization.suggestions
      ? organization.suggestions.proposed_sections.length + organization.suggestions.assignments.length || 1
      : 0
    return counts
  }, [catalog, organization.manifest, organization.suggestions, sections])

  const activeSectionDefinition = sections.find((section) => section.key === activeSection) ?? sections[0]
  const visibleDocuments = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase()
    return (catalog?.documents ?? [])
      .filter((entry) => sectionForDocument(entry, organization.manifest) === activeSection)
      .filter((entry) => !normalizedQuery
        || entry.title.toLowerCase().includes(normalizedQuery)
        || entry.path.toLowerCase().includes(normalizedQuery))
      .sort((left, right) => left.path.localeCompare(right.path, 'zh-CN'))
  }, [activeSection, catalog, organization.manifest, query])

  function chooseDocument(path: string) {
    if (dirty && !window.confirm('当前文档有未保存修改，确定切换吗？')) return
    setSelectedPath(path)
  }

  async function saveDocument() {
    if (!document || !catalog?.can_edit || !dirty) return
    setSaveState('saving')
    setMessage('')
    try {
      const response = await api.put<{ revision: string; byte_len: number }>(
        `/api/projects/${encodeURIComponent(projectId)}/docs/file`,
        { path: document.path, content: draft, expected_revision: document.revision || undefined },
      )
      setDocument({ ...document, content: draft, revision: response.revision, byte_len: response.byte_len })
      setSaveState('saved')
      await loadCatalog()
    } catch (error) {
      setSaveState('error')
      setMessage(errorMessage(error, '保存失败'))
    }
  }

  async function createNote() {
    if (!catalog?.can_edit) return
    const title = window.prompt('新笔记标题')?.trim()
    if (!title) return
    const timestamp = new Date().toISOString().replace(/\D/g, '').slice(0, 17)
    const path = `docs/inbox/${timestamp}-note.md`
    try {
      await api.put(`/api/projects/${encodeURIComponent(projectId)}/docs/file`, {
        path,
        content: `# ${title}\n\n`,
      })
      await loadCatalog()
      setActiveSection('unclassified')
      setSelectedPath(path)
      setViewMode('edit')
    } catch (error) {
      setMessage(errorMessage(error, '新建笔记失败'))
    }
  }

  async function createSection() {
    const label = window.prompt('新分区名称')?.trim()
    if (!label) return
    try {
      const key = await organization.addSection(label)
      setActiveSection(key)
    } catch (error) {
      setMessage(errorMessage(error, '新建分区失败'))
    }
  }

  async function removeSection(section: DocumentSection) {
    if (!section.custom || !window.confirm(`删除分区“${section.label}”？文档不会被删除。`)) return
    try {
      await organization.removeSection(section.key)
      if (activeSection === section.key) setActiveSection('unclassified')
    } catch (error) {
      setMessage(errorMessage(error, '删除分区失败'))
    }
  }

  async function assignSelectedDocument(sectionKey: string) {
    if (!selectedEntry) return
    try {
      const nextManifest = await organization.assignDocument(
        selectedEntry.path,
        sectionKey === AUTOMATIC_SECTION ? '' : sectionKey,
      )
      const nextSection = sectionForDocument(selectedEntry, nextManifest)
      setActiveSection(nextSection)
      setMessage(sectionKey === AUTOMATIC_SECTION
        ? '已恢复按路径和元数据自动分类。'
        : '分区已保存；只更新虚拟分类，文件路径未改变。')
    } catch (error) {
      setMessage(errorMessage(error, '保存文档分区失败'))
    }
  }

  async function startAiOrganize() {
    if (!catalog || !canStartAi) return
    setOrganizing(true)
    setMessage('')
    try {
      await organization.markSuggestionsRequested()
      await onStartAiOrganize(buildOrganizationPrompt(projectName, catalog, organization.manifest))
    } catch (error) {
      setMessage(errorMessage(error, '无法发起 AI 整理任务'))
      setOrganizing(false)
    }
  }

  async function applySuggestions() {
    if (!catalog) return
    setApplyingSuggestions(true)
    try {
      await organization.applySuggestions(catalog.documents)
      setMessage('AI 分区建议已应用；Markdown 文件未被移动或改写。')
    } catch (error) {
      setMessage(errorMessage(error, '应用 AI 建议失败'))
    } finally {
      setApplyingSuggestions(false)
    }
  }

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
        event.preventDefault()
        saveDocument()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [document, draft, catalog])

  return (
    <div className={styles.workspace}>
      <ProjectDocumentNotebookRail
        projectName={projectName}
        sections={sections}
        activeSection={activeSection}
        counts={sectionCounts}
        budget={catalog?.budget}
        canEdit={!!catalog?.can_edit}
        onBack={onBack}
        onSelect={setActiveSection}
        onCreate={createSection}
        onRemove={removeSection}
      />

      {activeSection === 'suggestions' ? (
        <ProjectDocumentSuggestions
          suggestions={organization.suggestions}
          loading={organization.loading}
          error={organization.error}
          canEdit={!!catalog?.can_edit}
          applying={applyingSuggestions}
          onRefresh={organization.reload}
          onApply={applySuggestions}
        />
      ) : (
        <>
          <aside className={styles.pageRail}>
            <header className={styles.pageHeader}>
              <div><strong>{activeSectionDefinition?.label}</strong><small>{visibleDocuments.length} 页</small></div>
              <button type="button" title="刷新目录" onClick={loadCatalog} disabled={catalogLoading}>
                <RefreshCw size={15} className={catalogLoading ? styles.spinning : ''} aria-hidden="true" />
              </button>
              <button type="button" title="新建 Inbox 笔记" onClick={createNote} disabled={!catalog?.can_edit}>
                <FilePlus2 size={16} aria-hidden="true" />
              </button>
            </header>
            <label className={styles.searchBox}>
              <Search size={14} aria-hidden="true" />
              <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索标题或路径" />
            </label>
            <div className={styles.pageList}>
              {catalogError && <div className={styles.errorBox}>{catalogError}</div>}
              {!catalogLoading && !catalogError && visibleDocuments.length === 0 && <div className={styles.emptyList}>这个分区还没有文档</div>}
              {visibleDocuments.map((entry) => (
                <button
                  className={[styles.pageButton, selectedPath === entry.path ? styles.pageActive : ''].join(' ')}
                  key={entry.path}
                  type="button"
                  onClick={() => chooseDocument(entry.path)}
                >
                  <span className={styles.pageTitle}><FileText size={14} aria-hidden="true" />{entry.title}</span>
                  <span className={styles.pagePath}>{entry.path}</span>
                  <span className={styles.pageMeta}>
                    <em>{roleLabel(entry.metadata.role)}</em>
                    <small>{formatNumber(entry.metadata.token_estimate)} token</small>
                    {entry.source === 'platform_default' && <b>平台模板</b>}
                    {entry.metadata.default_retrieval && <b>AI 必读</b>}
                  </span>
                </button>
              ))}
            </div>
            <button className={styles.organizeButton} type="button" disabled={!catalog || !canStartAi || organizing} onClick={startAiOrganize}>
              <Sparkles size={16} aria-hidden="true" />
              <span>{organizing ? '正在创建整理任务…' : '让当前 AI 生成整理建议'}</span>
            </button>
          </aside>

          <main className={styles.documentPane}>
            <header className={styles.documentHeader}>
              <div className={styles.documentIdentity}>
                <FolderTree size={18} aria-hidden="true" />
                <span><strong>{selectedEntry?.title ?? '选择一篇文档'}</strong><small>{selectedEntry?.path ?? catalog?.workspace ?? ''}</small></span>
              </div>
              <div className={styles.viewModes}>
                {(['edit', 'preview', 'split'] as ViewMode[]).map((mode) => (
                  <button className={viewMode === mode ? styles.modeActive : ''} key={mode} type="button" onClick={() => setViewMode(mode)}>
                    {mode === 'edit' ? '编辑' : mode === 'preview' ? '阅读' : '分栏'}
                  </button>
                ))}
              </div>
              <button className={styles.saveButton} type="button" onClick={saveDocument} disabled={!dirty || !catalog?.can_edit || saveState === 'saving'}>
                <Save size={15} aria-hidden="true" />
                {saveState === 'saving' ? '保存中' : saveState === 'saved' ? '已保存' : '保存'}
              </button>
            </header>

            {selectedEntry && (
              <div className={styles.authorityStrip}>
                <span>{roleLabel(selectedEntry.metadata.role)}</span>
                <span>{lifecycleLabel(selectedEntry.metadata.lifecycle)}</span>
                <span>{selectedEntry.metadata.authority || 'unknown'}</span>
                <select
                  aria-label="文档分区"
                  value={organization.manifest.assignments[normalizeDocumentPath(selectedEntry.path)] ?? AUTOMATIC_SECTION}
                  disabled={!catalog?.can_edit}
                  onChange={(event) => assignSelectedDocument(event.target.value)}
                >
                  <option value={AUTOMATIC_SECTION}>
                    自动：{automaticSectionLabel}
                  </option>
                  {sections.filter((section) => !section.virtual).map((section) => (
                    <option key={section.key} value={section.key}>{section.label}</option>
                  ))}
                </select>
                <small>{selectedEntry.metadata.reason}</small>
              </div>
            )}
            {message && <div className={styles.messageBar}>{message}</div>}

            <div className={[styles.editorBody, styles[`view_${viewMode}`]].join(' ')}>
              {documentLoading ? <div className={styles.documentEmpty}>正在按需读取这一篇文档…</div> : document ? (
                <>
                  {viewMode !== 'preview' && <textarea className={styles.editor} value={draft} onChange={(event) => { setDraft(event.target.value); setSaveState('idle') }} readOnly={!catalog?.can_edit} spellCheck={false} aria-label="Markdown 编辑器" />}
                  {viewMode !== 'edit' && <article className={styles.preview}><MarkdownContent content={draft || '（文档为空）'} /></article>}
                </>
              ) : (
                <div className={styles.documentEmpty}>
                  <Bot size={30} aria-hidden="true" /><strong>从左侧选择一篇文档</strong>
                  <span>程序只会在你打开时读取正文，目录扫描不会把全部 Markdown 送给 AI。</span>
                </div>
              )}
            </div>
          </main>
        </>
      )}
    </div>
  )
}

function errorMessage(error: unknown, fallback: string) {
  return (error as { message?: string })?.message ?? fallback
}

function normalizeDocumentPath(path: string) {
  return path.trim().replace(/\\/g, '/')
}
