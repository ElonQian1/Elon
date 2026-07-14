import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  ArrowLeft,
  BookOpen,
  Bot,
  FilePlus2,
  FileText,
  FolderTree,
  RefreshCw,
  Save,
  Search,
  ShieldCheck,
  Sparkles,
} from 'lucide-react'

import { api } from '../../api/client'
import MarkdownContent from '../markdown/MarkdownContent'
import {
  buildOrganizationPrompt,
  DOCUMENT_SECTIONS,
  formatNumber,
  lifecycleLabel,
  roleLabel,
  type DocumentCatalog,
  type DocumentFile,
} from './projectDocumentModel'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  projectId: string
  projectName: string
  onBack: () => void
  onStartAiOrganize: (prompt: string) => Promise<void>
  canStartAi: boolean
}

type ViewMode = 'edit' | 'preview' | 'split'


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
      setCatalogError((error as { message?: string }).message ?? '读取项目文档目录失败')
    } finally {
      setCatalogLoading(false)
    }
  }, [projectId])

  useEffect(() => {
    loadCatalog()
  }, [loadCatalog])

  const selectedEntry = useMemo(
    () => catalog?.documents.find((entry) => entry.path === selectedPath),
    [catalog, selectedPath],
  )
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
      setMessage((error as { message?: string }).message ?? '读取文档失败')
    } finally {
      setDocumentLoading(false)
    }
  }, [projectId])

  useEffect(() => {
    if (selectedPath) openDocument(selectedPath)
  }, [openDocument, selectedPath])

  const sectionCounts = useMemo(() => {
    const documents = catalog?.documents ?? []
    return Object.fromEntries(DOCUMENT_SECTIONS.map((section) => [
      section.key,
      documents.filter(section.test).length,
    ])) as Record<string, number>
  }, [catalog])

  const visibleDocuments = useMemo(() => {
    const section = DOCUMENT_SECTIONS.find((candidate) => candidate.key === activeSection) ?? DOCUMENT_SECTIONS[0]
    const normalizedQuery = query.trim().toLowerCase()
    return (catalog?.documents ?? [])
      .filter(section.test)
      .filter((entry) => !normalizedQuery
        || entry.title.toLowerCase().includes(normalizedQuery)
        || entry.path.toLowerCase().includes(normalizedQuery))
      .sort((left, right) => left.path.localeCompare(right.path, 'zh-CN'))
  }, [activeSection, catalog, query])

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
        {
          path: document.path,
          content: draft,
          expected_revision: document.revision || undefined,
        },
      )
      setDocument({ ...document, content: draft, revision: response.revision, byte_len: response.byte_len })
      setSaveState('saved')
      await loadCatalog()
    } catch (error) {
      setSaveState('error')
      setMessage((error as { message?: string }).message ?? '保存失败')
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
      setMessage((error as { message?: string }).message ?? '新建笔记失败')
    }
  }

  async function startAiOrganize() {
    if (!catalog || !canStartAi) return
    setOrganizing(true)
    setMessage('')
    try {
      await onStartAiOrganize(buildOrganizationPrompt(projectName, catalog))
    } catch (error) {
      setMessage((error as { message?: string }).message ?? '无法发起 AI 整理任务')
      setOrganizing(false)
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
        </div>

        <div className={styles.sectionList}>
          {DOCUMENT_SECTIONS.map((section) => (
            <button
              className={[styles.sectionButton, activeSection === section.key ? styles.sectionActive : ''].join(' ')}
              key={section.key}
              type="button"
              onClick={() => setActiveSection(section.key)}
            >
              <span className={styles.sectionColor} data-section={section.key} />
              <span className={styles.sectionCopy}>
                <strong>{section.label}</strong>
                <small>{section.detail}</small>
              </span>
              <em>{sectionCounts[section.key] ?? 0}</em>
            </button>
          ))}
        </div>

        {catalog?.budget && (
          <div className={styles.budgetCard}>
            <span><ShieldCheck size={15} aria-hidden="true" /> 程序预分类</span>
            <strong>{catalog.budget.classification_model_tokens} AI token</strong>
            <small>
              默认排除 {catalog.budget.excluded_by_default} 份，预计少读 {formatNumber(catalog.budget.estimated_tokens_avoided)} token
            </small>
          </div>
        )}
      </aside>

      <aside className={styles.pageRail}>
        <header className={styles.pageHeader}>
          <div>
            <strong>{DOCUMENT_SECTIONS.find((section) => section.key === activeSection)?.label}</strong>
            <small>{visibleDocuments.length} 页</small>
          </div>
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
          {!catalogLoading && !catalogError && visibleDocuments.length === 0 && (
            <div className={styles.emptyList}>这个分区还没有文档</div>
          )}
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
                {entry.metadata.default_retrieval && <b>AI 必读</b>}
              </span>
            </button>
          ))}
        </div>
        <button
          className={styles.organizeButton}
          type="button"
          disabled={!catalog || !canStartAi || organizing}
          onClick={startAiOrganize}
        >
          <Sparkles size={16} aria-hidden="true" />
          <span>{organizing ? '正在创建整理任务…' : '让当前 AI 整理文档'}</span>
        </button>
      </aside>

      <main className={styles.documentPane}>
        <header className={styles.documentHeader}>
          <div className={styles.documentIdentity}>
            <FolderTree size={18} aria-hidden="true" />
            <span>
              <strong>{selectedEntry?.title ?? '选择一篇文档'}</strong>
              <small>{selectedEntry?.path ?? catalog?.workspace ?? ''}</small>
            </span>
          </div>
          <div className={styles.viewModes}>
            {(['edit', 'preview', 'split'] as ViewMode[]).map((mode) => (
              <button
                className={viewMode === mode ? styles.modeActive : ''}
                key={mode}
                type="button"
                onClick={() => setViewMode(mode)}
              >
                {mode === 'edit' ? '编辑' : mode === 'preview' ? '阅读' : '分栏'}
              </button>
            ))}
          </div>
          <button
            className={styles.saveButton}
            type="button"
            onClick={saveDocument}
            disabled={!dirty || !catalog?.can_edit || saveState === 'saving'}
          >
            <Save size={15} aria-hidden="true" />
            {saveState === 'saving' ? '保存中' : saveState === 'saved' ? '已保存' : '保存'}
          </button>
        </header>

        {selectedEntry && (
          <div className={styles.authorityStrip}>
            <span>{roleLabel(selectedEntry.metadata.role)}</span>
            <span>{lifecycleLabel(selectedEntry.metadata.lifecycle)}</span>
            <span>{selectedEntry.metadata.authority || 'unknown'}</span>
            <small>{selectedEntry.metadata.reason}</small>
          </div>
        )}
        {message && <div className={styles.messageBar}>{message}</div>}

        <div className={[styles.editorBody, styles[`view_${viewMode}`]].join(' ')}>
          {documentLoading ? (
            <div className={styles.documentEmpty}>正在按需读取这一篇文档…</div>
          ) : document ? (
            <>
              {viewMode !== 'preview' && (
                <textarea
                  className={styles.editor}
                  value={draft}
                  onChange={(event) => {
                    setDraft(event.target.value)
                    setSaveState('idle')
                  }}
                  readOnly={!catalog?.can_edit}
                  spellCheck={false}
                  aria-label="Markdown 编辑器"
                />
              )}
              {viewMode !== 'edit' && (
                <article className={styles.preview}>
                  <MarkdownContent content={draft || '（文档为空）'} />
                </article>
              )}
            </>
          ) : (
            <div className={styles.documentEmpty}>
              <Bot size={30} aria-hidden="true" />
              <strong>从左侧选择一篇文档</strong>
              <span>程序只会在你打开时读取正文，目录扫描不会把全部 Markdown 送给 AI。</span>
            </div>
          )}
        </div>
      </main>
    </div>
  )
}
