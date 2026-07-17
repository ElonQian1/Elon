import { lazy, Suspense, useCallback, useEffect, useMemo, useState } from 'react'
import { ArrowUpDown, FilePlus2, RefreshCw, Search, Sparkles } from 'lucide-react'

import { api } from '../../api/client'
import ProjectDocumentArchitectureHealth from './ProjectDocumentArchitectureHealth'
import ProjectDocumentCommandDialog, {
  type ProjectDocumentDialogResult,
  type ProjectDocumentDialogState,
} from './ProjectDocumentCommandDialog'
import ProjectDocumentCommandMenu, {
  type ProjectDocumentCommandId,
  type ProjectDocumentMenuTarget,
} from './ProjectDocumentCommandMenu'
import type { ProjectCapabilityNode } from './projectDocumentCapabilityGraph'
import type { ProjectKnowledgeMapView } from './projectDocumentKnowledgeGraphModel'
import {
  knowledgeMapReviewInstruction,
  knowledgeNodeReviewInstruction,
} from './projectDocumentKnowledgeMapPrompts'
import { useProjectDocumentAutomationPolicy } from './projectDocumentAutomationPolicy'
import ProjectDocumentHealthSummary from './ProjectDocumentHealthSummary'
import ProjectDocumentHealthCenter from './ProjectDocumentHealthCenter'
import ProjectDocumentGovernanceOverview from './ProjectDocumentGovernanceOverview'
import ProjectDocumentEditorPane, {
  AUTOMATIC_DOCUMENT_SECTION,
  type ProjectDocumentViewMode,
} from './ProjectDocumentEditorPane'
import ProjectDocumentKnowledgeHome from './ProjectDocumentKnowledgeHome'
import ProjectDocumentNotebookRail from './ProjectDocumentNotebookRail'
import ProjectDocumentPageList from './ProjectDocumentPageList'
import ProjectDocumentSuggestions from './ProjectDocumentSuggestions'
import type { DocumentOrganizationTrackingRuntime } from './projectDocumentOrganizationStatus'
import {
  analyzeKnowledgeArchitecture,
  CAPABILITY_MAP_SECTION,
  DOCUMENT_HEALTH_SECTION,
  KNOWLEDGE_HOME_SECTION,
  knowledgeSectionCounts,
  serverArchitectureHealth,
  topicSectionForDocument,
  type DocumentNavigationMode,
} from './projectDocumentArchitecture'
import {
  loadDocumentViewPreferences,
  mergeSections,
  pinDocuments,
  reorderDocument,
  reorderDocumentBefore,
  reorderSection,
  reorderSectionBefore,
  setGovernanceFacets,
  setKnowledgeEntrypoint,
  setRecommendedDocuments,
  setSecondaryTopics,
  saveDocumentViewPreferences,
  sortDocuments,
  updateSectionDefinition,
  type ProjectDocumentViewPreferences,
} from './projectDocumentCommands'
import { type DocumentCatalog, type DocumentFile } from './projectDocumentModel'
import {
  buildOrganizationPrompt,
  customSectionKey,
  GOVERNANCE_OVERVIEW_SECTION,
  governanceSectionForDocument,
  sectionForDocument,
  type DocumentSection,
} from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'
import { useProjectDocumentOrganization } from './useProjectDocumentOrganization'
import { useProjectDocumentNavigation } from './useProjectDocumentNavigation'
import { menuPointForButton, type ProjectDocumentMenuPoint } from './useProjectDocumentMenuTrigger'

const ProjectDocumentCapabilityMap = lazy(() => import('./ProjectDocumentCapabilityMap'))

interface Props {
  projectId: string
  projectName: string
  onBack: () => void
  organizationTracking: DocumentOrganizationTrackingRuntime
  onStartAiOrganize: (prompt: string) => Promise<{ task_id?: string } | null>
  canStartAi: boolean
}

export default function ProjectDocumentsWorkspace({
  projectId,
  projectName,
  onBack,
  onStartAiOrganize,
  canStartAi,
  organizationTracking,
}: Props) {
  const [catalog, setCatalog] = useState<DocumentCatalog | null>(null)
  const [catalogLoading, setCatalogLoading] = useState(true)
  const [catalogError, setCatalogError] = useState('')
  const [navigationMode, setNavigationMode] = useState<DocumentNavigationMode>('knowledge')
  const [activeSection, setActiveSection] = useState(KNOWLEDGE_HOME_SECTION)
  const [query, setQuery] = useState('')
  const [selectedPath, setSelectedPath] = useState('')
  const [document, setDocument] = useState<DocumentFile | null>(null)
  const [draft, setDraft] = useState('')
  const [documentLoading, setDocumentLoading] = useState(false)
  const [documentError, setDocumentError] = useState('')
  const [saveState, setSaveState] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')
  const [message, setMessage] = useState('')
  const [viewMode, setViewMode] = useState<ProjectDocumentViewMode>('split')
  const [organizing, setOrganizing] = useState(false)
  const [applyingSuggestions, setApplyingSuggestions] = useState(false)
  const [applyingFileOperations, setApplyingFileOperations] = useState(false)
  const [commandBusy, setCommandBusy] = useState(false)
  const [menuTarget, setMenuTarget] = useState<ProjectDocumentMenuTarget | null>(null)
  const [menuPoint, setMenuPoint] = useState<ProjectDocumentMenuPoint>({ x: 0, y: 0 })
  const [dialogState, setDialogState] = useState<ProjectDocumentDialogState | null>(null)
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(() => new Set())
  const [viewPreferences, setViewPreferences] = useState<ProjectDocumentViewPreferences>(() => loadDocumentViewPreferences(projectId))
  const organization = useProjectDocumentOrganization(projectId, organizationTracking)
  const automationPolicy = useProjectDocumentAutomationPolicy(projectId)
  const architectureHealth = useMemo(() => serverArchitectureHealth(
    catalog, analyzeKnowledgeArchitecture(catalog, organization.manifest),
  ), [catalog, organization.manifest])

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
  useEffect(() => setViewPreferences(loadDocumentViewPreferences(projectId)), [projectId])
  useEffect(() => saveDocumentViewPreferences(projectId, viewPreferences), [projectId, viewPreferences])

  const {
    governanceSections, knowledgeSections, baseSections, governanceCounts, sectionCounts,
    sections, activeSectionDefinition, sectionDocuments, visibleDocuments,
  } = useProjectDocumentNavigation({
    catalog, manifest: organization.manifest, suggestions: organization.suggestions,
    navigationMode, activeSection, query, viewPreferences,
  })

  const selectedEntry = useMemo(
    () => catalog?.documents.find((entry) => entry.path === selectedPath),
    [catalog, selectedPath],
  )
  const automaticSectionKey = useMemo(() => {
    if (!selectedEntry) return 'unclassified'
    const field = navigationMode === 'knowledge' ? 'assignments' : 'governance_overrides'
    const assignments = { ...organization.manifest[field] }
    delete assignments[normalizeDocumentPath(selectedEntry.path)]
    return navigationMode === 'knowledge'
      ? topicSectionForDocument(selectedEntry, catalog, { ...organization.manifest, assignments })
      : governanceSectionForDocument(selectedEntry, { ...organization.manifest, governance_overrides: assignments })
  }, [catalog, navigationMode, organization.manifest, selectedEntry])
  const automaticSectionLabel = baseSections.find((section) => section.key === automaticSectionKey)?.label ?? '等待整理'
  const assignmentSections = navigationMode === 'knowledge'
    ? knowledgeSections.filter((section) => section.custom)
    : governanceSections.filter((section) => !section.virtual)
  const persistedAssignment = selectedEntry
    ? organization.manifest[navigationMode === 'knowledge' ? 'assignments' : 'governance_overrides'][normalizeDocumentPath(selectedEntry.path)]
    : undefined
  const selectedAssignment = assignmentSections.some((section) => section.key === persistedAssignment)
    ? persistedAssignment!
    : AUTOMATIC_DOCUMENT_SECTION
  const dirty = !!document && draft !== document.content

  const openDocument = useCallback(async (path: string) => {
    if (!path) return
    setDocumentLoading(true)
    setMessage('')
    setDocumentError('')
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
      setDocumentError(errorMessage(error, '读取文档失败'))
    } finally {
      setDocumentLoading(false)
    }
  }, [projectId])

  useEffect(() => {
    if (selectedPath) openDocument(selectedPath)
  }, [openDocument, selectedPath])

  useEffect(() => setSelectedPaths(new Set()), [activeSection, navigationMode])

  const openCommandMenu = useCallback((target: ProjectDocumentMenuTarget, point: ProjectDocumentMenuPoint) => {
    setMenuTarget(target)
    setMenuPoint(point)
  }, [])

  function changeNavigationMode(mode: DocumentNavigationMode) {
    setNavigationMode(mode)
    setActiveSection(mode === 'knowledge' ? KNOWLEDGE_HOME_SECTION : GOVERNANCE_OVERVIEW_SECTION.key)
  }

  function openDocumentFromHome(path: string) {
    const entry = catalog?.documents.find((document) => document.path === path)
    if (!entry) return
    setNavigationMode('knowledge')
    setActiveSection(topicSectionForDocument(entry, catalog, organization.manifest))
    chooseDocument(path)
  }

  function openDocumentFromGovernance(path: string) {
    const entry = catalog?.documents.find((document) => document.path === path)
    if (!entry) return
    setNavigationMode('governance')
    setActiveSection(governanceSectionForDocument(entry, organization.manifest))
    chooseDocument(path)
  }

  function organizeCapability(node: ProjectCapabilityNode) {
    void startAiOrganize(knowledgeNodeReviewInstruction(node))
  }

  function reviewKnowledgeMap(view: ProjectKnowledgeMapView) {
    void startAiOrganize(knowledgeMapReviewInstruction(view))
  }

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
      setNavigationMode('governance')
      setActiveSection('unclassified')
      setSelectedPath(path)
      setViewMode('edit')
    } catch (error) {
      setMessage(errorMessage(error, '新建笔记失败'))
    }
  }

  async function saveGovernance(path: string, facets: Parameters<typeof setGovernanceFacets>[2], secondaryTopics: string[]) {
    let next = setGovernanceFacets(organization.manifest, path, facets)
    next = setSecondaryTopics(next, path, secondaryTopics)
    await organization.applyManifest(next)
    setMessage('多维治理属性和副主题已保存；路径权威上限仍由程序强制保护。')
    await loadCatalog()
  }

  function createSection(parentId = '') {
    setDialogState({ mode: 'create-section', title: parentId ? '新建子分区' : '新建一级分区', parentId })
  }

  async function removeSection(section: DocumentSection) {
    if (!section.custom || !window.confirm(`删除分区“${section.label}”？文档不会被删除。`)) return
    try {
      await organization.removeSection(section.key)
      if (activeSection === section.key) setActiveSection(KNOWLEDGE_HOME_SECTION)
    } catch (error) {
      setMessage(errorMessage(error, '删除分区失败'))
    }
  }

  async function assignSelectedDocument(sectionKey: string) {
    if (!selectedEntry) return
    try {
      const nextManifest = await organization.assignDocument(
        selectedEntry.path,
        sectionKey === AUTOMATIC_DOCUMENT_SECTION ? '' : sectionKey,
        navigationMode,
      )
      const nextSection = navigationMode === 'knowledge'
        ? topicSectionForDocument(selectedEntry, catalog, nextManifest)
        : sectionForDocument(selectedEntry, nextManifest)
      setActiveSection(nextSection)
      setMessage(sectionKey === AUTOMATIC_DOCUMENT_SECTION
        ? '已恢复按路径和元数据自动分类。'
        : navigationMode === 'knowledge'
          ? '主题归类已保存；治理属性和真实文件路径均未改变。'
          : '治理归类已保存；主题知识树和真实文件路径均未改变。')
    } catch (error) {
      setMessage(errorMessage(error, '保存文档分区失败'))
    }
  }

  async function startAiOrganize(scopeInstruction = '') {
    if (!catalog || !canStartAi) return
    setOrganizing(true)
    setMessage('')
    let operationId: string | undefined
    try {
      operationId = await organization.startRun()
      const basePrompt = buildOrganizationPrompt(
        projectName,
        catalog,
        organization.manifest,
        operationId,
        automationPolicy.mode,
      )
      const response = await onStartAiOrganize(scopeInstruction
        ? `${basePrompt}\n\n本次菜单范围：${scopeInstruction}`
        : basePrompt)
      await organization.markDispatched(operationId, response?.task_id)
      setMessage(operationId
        ? 'AI 整理任务已发起；可在“AI 整理建议”分区观察 MCP 每一步。'
        : 'AI 整理任务已发起；当前运行路线不提供本机 MCP 分阶段观测。')
    } catch (error) {
      await organization.markFailed(operationId, errorMessage(error, '无法发起 AI 整理任务'))
      setMessage(errorMessage(error, '无法发起 AI 整理任务'))
    } finally {
      setOrganizing(false)
    }
  }

  async function applySuggestions() {
    if (!catalog) return
    setApplyingSuggestions(true)
    try {
      const result = await organization.applySuggestions(catalog.revision, automationPolicy.mode)
      setMessage(result?.git_result_commit
        ? `AI 分区建议已应用；Git 已保存整理前 ${result.git_baseline_commit?.slice(0, 8)} 和整理后 ${result.git_result_commit.slice(0, 8)} 两个提交。`
        : result?.git_baseline_commit
          ? `整理前 Git 备份 ${result.git_baseline_commit.slice(0, 8)} 已保存；继续执行实体文档操作后会提交整理结果。`
          : 'AI 分区建议已应用；Markdown 文件未被移动或改写。')
    } catch (error) {
      setMessage(errorMessage(error, '应用 AI 建议失败'))
    } finally {
      setApplyingSuggestions(false)
    }
  }

  async function applyFileOperations(input: { operationIds: string[]; allowRename: boolean; allowMove: boolean }) {
    if (!catalog) return
    const localCatalogRevision = organization.trace?.catalog_revision
    if (!localCatalogRevision) throw new Error('缺少本机 MCP 目录 revision，请刷新整理建议后再执行实体操作')
    setApplyingFileOperations(true)
    try {
      const result = await organization.applyFileOperations({
        catalogRevision: localCatalogRevision,
        authorizationMode: automationPolicy.mode,
        ...input,
      })
      if (result.git_baseline_commit && result.git_result_commit) {
        setMessage(`文档整理已完成：Git 已保存整理前 ${result.git_baseline_commit.slice(0, 8)} 和整理后 ${result.git_result_commit.slice(0, 8)} 两个提交。`)
      }
      const selectedMove = result.operations.find((operation) => operation.source_path === selectedPath)
      if (selectedMove) {
        setDocument(null)
        setDraft('')
        setSelectedPath(selectedMove.target_path)
      }
      await loadCatalog()
    } finally {
      setApplyingFileOperations(false)
    }
  }

  async function executeCommand(command: ProjectDocumentCommandId, target: ProjectDocumentMenuTarget) {
    setMessage('')
    if (command.startsWith('section-sort:')) {
      setViewPreferences((current) => ({ ...current, sectionSort: command.split(':')[1] as ProjectDocumentViewPreferences['sectionSort'] }))
      return
    }
    if (command.startsWith('document-sort:')) {
      setViewPreferences((current) => ({ ...current, documentSort: command.split(':')[1] as ProjectDocumentViewPreferences['documentSort'] }))
      return
    }
    if (command === 'undo') {
      try {
        const restored = await organization.undoLastChange()
        if (restored) setMessage('已撤销上一次知识架构操作；撤销记录已写入项目审计。')
      } catch (error) {
        setMessage(errorMessage(error, '撤销失败'))
      }
      return
    }
    if (command === 'new-root') { createSection(); return }

    if (target.kind === 'section') {
      const section = target.section
      const stored = organization.manifest.sections.find((item) => customSectionKey(item.id) === section.key)
      if (command === 'open') { setActiveSection(section.key); return }
      if (command === 'new-child') { createSection(stored?.id ?? ''); return }
      if (command === 'new-sibling') { createSection(stored?.parent_id ?? ''); return }
      if (!stored) {
        if (command === 'ai-section') await startAiOrganize(`只整理知识主题“${section.label}”（${section.key}）；优先检查其中歧义、重复和缺少入口的文档。`)
        else setMessage('模板分区由项目类型维护；如需自定义，请新建项目分区。')
        return
      }
      if (command === 'edit-section') setDialogState({ mode: 'edit-section', title: '重命名与外观', section: stored })
      else if (command === 'move-parent') setDialogState({ mode: 'move-parent', title: `更改“${stored.label}”的父分区`, section: stored })
      else if (command === 'merge-section') setDialogState({ mode: 'merge-section', title: `合并“${stored.label}”`, section: stored })
      else if (['move-top', 'move-up', 'move-down', 'move-bottom'].includes(command)) {
        try {
          await organization.applyManifest(reorderSection(
            organization.manifest,
            section.key,
            command.replace('move-', '') as 'top' | 'up' | 'down' | 'bottom',
          ))
          setViewPreferences((current) => ({ ...current, sectionSort: 'manual' }))
        } catch (error) { setMessage(errorMessage(error, '调整分区顺序失败')) }
      } else if (command === 'section-entrypoint') {
        if (!selectedPath) setMessage('请先打开一篇文档，再把它设置为分区入口。')
        else try {
          await organization.applyManifest(setKnowledgeEntrypoint(organization.manifest, selectedPath, section.key))
          setMessage(`已将 ${selectedPath} 设为“${section.label}”入口。`)
        } catch (error) { setMessage(errorMessage(error, '设置分区入口失败')) }
      } else if (command === 'ai-section') {
        await startAiOrganize(`只整理知识主题“${section.label}”（${section.key}）；优先检查其中歧义、重复和缺少入口的文档。`)
      } else if (command === 'delete-section') {
        await removeSection(section)
      }
      return
    }

    if (target.kind !== 'document') return
    const entry = target.document
    if (command === 'open') chooseDocument(entry.path)
    else if (command === 'edit') { chooseDocument(entry.path); setViewMode('edit') }
    else if (command === 'read') { chooseDocument(entry.path); setViewMode('preview') }
    else if (command === 'toggle-selection') toggleDocumentSelection(entry.path)
    else if (command === 'copy-path' || command === 'copy-link') {
      const text = command === 'copy-path' ? entry.path : `[${entry.title}](${entry.path.replace(/ /g, '%20')})`
      try { await navigator.clipboard.writeText(text); setMessage('已复制到剪贴板。') }
      catch { setMessage('浏览器未允许写入剪贴板。') }
    } else if (command === 'assign-topic') {
      setDialogState({ mode: 'assign-topic', title: `移动“${entry.title}”到知识主题`, paths: [entry.path], current: organization.manifest.assignments[normalizeDocumentPath(entry.path)] })
    } else if (command === 'assign-governance') {
      setDialogState({ mode: 'assign-governance', title: `调整“${entry.title}”的治理属性`, paths: [entry.path], current: organization.manifest.governance_overrides[normalizeDocumentPath(entry.path)] })
    } else if (command === 'restore-automatic') {
      await organization.assignManyDocuments([entry.path], '', navigationMode)
      setMessage('已恢复当前浏览轴的自动分类。')
    } else if (['move-top', 'move-up', 'move-down', 'move-bottom'].includes(command)) {
      await organization.applyManifest(reorderDocument(
        organization.manifest,
        sortDocuments(sectionDocuments, organization.manifest, 'manual').map((document) => document.path),
        entry.path,
        command.replace('move-', '') as 'top' | 'up' | 'down' | 'bottom',
      ))
      setViewPreferences((current) => ({ ...current, documentSort: 'manual' }))
    } else if (command === 'pin' || command === 'unpin') {
      await organization.applyManifest(pinDocuments(organization.manifest, [entry.path], command === 'pin'))
    } else if (command === 'recommend' || command === 'unrecommend') {
      await organization.applyManifest(setRecommendedDocuments(organization.manifest, [entry.path], command === 'recommend'))
    } else if (command === 'home-entrypoint') {
      await organization.applyManifest(setKnowledgeEntrypoint(organization.manifest, entry.path))
      setMessage(`已将 ${entry.path} 设为知识首页入口。`)
    } else if (command === 'section-entrypoint') {
      try {
        await organization.applyManifest(setKnowledgeEntrypoint(organization.manifest, entry.path, activeSection))
        setMessage(`已将 ${entry.path} 设为当前主题入口。`)
      } catch (error) { setMessage(errorMessage(error, '设置主题入口失败')) }
    } else if (command === 'ai-document') {
      await startAiOrganize(`只整理文档 ${entry.path}；判断主题、治理属性、关系和入口价值，除非确有歧义不要读取其它正文。`)
    } else if (command === 'ai-governance') {
      await startAiOrganize(`评估文档 ${entry.path} 是否应提权。必须先检查真实路径的权威上限、同级冲突、替代关系和 default_retrieval；不能用虚拟分区绕过路径上限。`)
    } else if (command === 'ai-file-name') {
      await startAiOrganize(`只评估文档 ${entry.path} 的文件名和真实路径；如确有价值，用带 source_revision 的 file_operations 建议安全 rename/move。`)
    }
  }

  async function submitCommandDialog(result: ProjectDocumentDialogResult) {
    const state = dialogState
    if (!state) return
    setCommandBusy(true)
    setMessage('')
    try {
      if (result.mode === 'create-section') {
        const key = await organization.addSection(result.label ?? '', result.parentId ?? '', {
          detail: result.detail, color: result.color, icon: result.icon,
        })
        setNavigationMode('knowledge')
        setActiveSection(key)
      } else if (result.mode === 'edit-section' && state.mode === 'edit-section') {
        await organization.applyManifest(updateSectionDefinition(organization.manifest, customSectionKey(state.section.id), {
          label: result.label, detail: result.detail, color: result.color, icon: result.icon,
        }))
      } else if (result.mode === 'move-parent' && state.mode === 'move-parent') {
        await organization.applyManifest(updateSectionDefinition(organization.manifest, customSectionKey(state.section.id), {
          parent_id: result.parentId ?? '',
        }))
      } else if (result.mode === 'merge-section' && state.mode === 'merge-section' && result.targetSectionKey) {
        await organization.applyManifest(mergeSections(organization.manifest, customSectionKey(state.section.id), result.targetSectionKey))
        if (activeSection === customSectionKey(state.section.id)) setActiveSection(result.targetSectionKey)
      } else if ((result.mode === 'assign-topic' || result.mode === 'assign-governance') && result.targetSectionKey) {
        await organization.assignManyDocuments(
          result.paths ?? [],
          result.targetSectionKey,
          result.mode === 'assign-topic' ? 'knowledge' : 'governance',
        )
        setMessage(result.mode === 'assign-topic'
          ? '知识主题已更新；治理权威性和真实文件路径未改变。'
          : '治理显示已更新；没有突破真实路径的权威上限。')
      }
      setDialogState(null)
    } catch (error) {
      setMessage(errorMessage(error, '保存项目文档操作失败'))
    } finally {
      setCommandBusy(false)
    }
  }

  function toggleDocumentSelection(path: string) {
    setSelectedPaths((current) => {
      const next = new Set(current)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }

  async function applyBatchAssignment(sectionKey: string) {
    setCommandBusy(true)
    try {
      await organization.assignManyDocuments([...selectedPaths], sectionKey, navigationMode)
      setSelectedPaths(new Set())
      setMessage(`已批量更新 ${selectedPaths.size} 份文档；真实路径和正文未改变。`)
    } catch (error) { setMessage(errorMessage(error, '批量归类失败')) }
    finally { setCommandBusy(false) }
  }

  async function applyBatchManifest(action: 'pin' | 'recommend' | 'automatic') {
    const paths = [...selectedPaths]
    setCommandBusy(true)
    try {
      if (action === 'pin') await organization.applyManifest(pinDocuments(organization.manifest, paths, true))
      else if (action === 'recommend') await organization.applyManifest(setRecommendedDocuments(organization.manifest, paths, true))
      else await organization.assignManyDocuments(paths, '', navigationMode)
      setMessage(`已批量处理 ${paths.length} 份文档。`)
    } catch (error) { setMessage(errorMessage(error, '批量操作失败')) }
    finally { setCommandBusy(false) }
  }

  async function moveSectionBefore(sourceKey: string, targetKey: string) {
    try {
      await organization.applyManifest(reorderSectionBefore(organization.manifest, sourceKey, targetKey))
      setViewPreferences((current) => ({ ...current, sectionSort: 'manual' }))
    } catch (error) { setMessage(errorMessage(error, '拖动排序失败')) }
  }

  async function moveDocumentBefore(sourcePath: string, targetPath: string) {
    try {
      await organization.applyManifest(reorderDocumentBefore(
        organization.manifest,
        sortDocuments(sectionDocuments, organization.manifest, 'manual').map((document) => document.path),
        sourcePath,
        targetPath,
      ))
      setViewPreferences((current) => ({ ...current, documentSort: 'manual' }))
    } catch (error) { setMessage(errorMessage(error, '拖动文档排序失败')) }
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
        navigationMode={navigationMode}
        canEdit={!!catalog?.can_edit}
        onBack={onBack}
        onNavigationModeChange={changeNavigationMode}
        onSelect={setActiveSection}
        onCreate={() => createSection()}
        onOpenMenu={openCommandMenu}
        onMoveBefore={moveSectionBefore}
        onUndo={() => { void executeCommand('undo', { kind: 'rail' }) }}
        canUndo={organization.canUndo}
      />

      {activeSection === KNOWLEDGE_HOME_SECTION ? (
        <ProjectDocumentKnowledgeHome
          projectName={projectName}
          catalog={catalog}
          manifest={organization.manifest}
          health={architectureHealth}
          sections={knowledgeSections}
          counts={knowledgeSectionCounts(catalog, organization.manifest, knowledgeSections)}
          onOpenDocument={openDocumentFromHome}
          onOpenSection={setActiveSection}
          onOpenSuggestions={() => setActiveSection('suggestions')}
          onProfileChange={(profile) => organization.setProfile(profile).catch((error) => {
            setMessage(errorMessage(error, '保存项目知识模板失败'))
          })}
        />
      ) : activeSection === CAPABILITY_MAP_SECTION ? (
        <Suspense fallback={<div className={styles.capabilityLoading}>正在加载统一项目知识图谱…</div>}>
          <ProjectDocumentCapabilityMap projectName={projectName} catalog={catalog}
            canStartAi={canStartAi} organizing={organizing} onOpenDocument={openDocumentFromHome}
            onOpenSection={setActiveSection} onAiOrganize={organizeCapability} onAiReview={reviewKnowledgeMap} />
        </Suspense>
      ) : activeSection === DOCUMENT_HEALTH_SECTION ? (
        <ProjectDocumentHealthCenter analysis={catalog?.analysis} runtime={organizationTracking} onRefresh={loadCatalog}
          onOpenSuggestions={() => setActiveSection('suggestions')} onRunAi={(instruction) => { void startAiOrganize(instruction) }} />
      ) : activeSection === GOVERNANCE_OVERVIEW_SECTION.key ? (
        <ProjectDocumentGovernanceOverview catalog={catalog} manifest={organization.manifest}
          canEdit={!!catalog?.can_edit} onOpenDocument={openDocumentFromGovernance} onSave={saveGovernance} />
      ) : activeSection === 'suggestions' ? (
        <ProjectDocumentSuggestions
          suggestions={organization.suggestions}
          trace={organization.trace}
          trackingAvailable={organization.trackingAvailable}
          trackingError={organization.trackingError}
          loading={organization.loading}
          error={organization.error}
          canEdit={!!catalog?.can_edit}
          applying={applyingSuggestions}
          applyingFiles={applyingFileOperations}
          canApplyFiles={!!catalog?.can_edit && organization.trackingAvailable && !!organization.trace?.catalog_revision}
          automationMode={automationPolicy.mode}
          onAutomationModeChange={automationPolicy.setMode}
          onRefresh={organization.reload}
          onApply={applySuggestions}
          onApplyFiles={applyFileOperations}
        />
      ) : (
        <>
          <aside className={styles.pageRail}>
            <header className={styles.pageHeader}>
              <div><strong>{activeSectionDefinition?.label}</strong><small>{visibleDocuments.length} 页</small></div>
              <button type="button" title="刷新目录" onClick={loadCatalog} disabled={catalogLoading}>
                <RefreshCw size={15} className={catalogLoading ? styles.spinning : ''} aria-hidden="true" />
              </button>
              <button type="button" title="文档排序" onClick={(event) => openCommandMenu({ kind: 'page-list' }, menuPointForButton(event.currentTarget))}>
                <ArrowUpDown size={15} aria-hidden="true" />
              </button>
              <button type="button" title="新建 Inbox 笔记" onClick={createNote} disabled={!catalog?.can_edit}>
                <FilePlus2 size={16} aria-hidden="true" />
              </button>
            </header>
            <label className={styles.searchBox}>
              <Search size={14} aria-hidden="true" />
              <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索标题或路径" />
            </label>
            {navigationMode === 'knowledge' ? (
              <ProjectDocumentArchitectureHealth
                health={architectureHealth}
                onOpenHome={() => setActiveSection(KNOWLEDGE_HOME_SECTION)}
                onOpenSuggestions={() => setActiveSection('suggestions')}
              />
            ) : (
              <ProjectDocumentHealthSummary
                catalog={catalog}
                unclassified={governanceCounts.unclassified ?? 0}
                suggestions={organization.suggestions}
                onOpenSuggestions={() => setActiveSection('suggestions')}
              />
            )}
            <ProjectDocumentPageList
              documents={visibleDocuments}
              manifest={organization.manifest}
              navigationMode={navigationMode}
              assignmentSections={navigationMode === 'knowledge'
                ? knowledgeSections.filter((section) => section.custom)
                : governanceSections.filter((section) => !section.virtual)}
              selectedPath={selectedPath}
              selectedPaths={selectedPaths}
              loading={catalogLoading}
              error={catalogError}
              commandBusy={commandBusy}
              canEdit={!!catalog?.can_edit}
              onChoose={chooseDocument}
              onToggleSelection={toggleDocumentSelection}
              onOpenMenu={openCommandMenu}
              onBatchAssign={applyBatchAssignment}
              onBatchAction={(action) => { void applyBatchManifest(action) }}
              onBatchAi={(paths) => {
                const listed = paths.slice(0, 40).join(', ')
                void startAiOrganize(`只整理用户批量选择的 ${paths.length} 份文档：${listed}${paths.length > 40 ? '；其余路径由目录中的当前分区范围限定' : ''}。`)
              }}
              onClearSelection={() => setSelectedPaths(new Set())}
              onMoveBefore={moveDocumentBefore}
            />
            <button className={styles.organizeButton} type="button" disabled={!catalog || !canStartAi || organizing} onClick={() => { void startAiOrganize() }}>
              <Sparkles size={16} aria-hidden="true" />
              <span>{organizing ? '正在创建整理任务…' : '让当前 AI 生成整理建议'}</span>
            </button>
          </aside>

          <ProjectDocumentEditorPane
            catalog={catalog}
            document={document}
            selectedEntry={selectedEntry}
            selectedPath={selectedPath}
            selectedAssignment={selectedAssignment}
            automaticSectionLabel={automaticSectionLabel}
            assignmentSections={assignmentSections}
            viewMode={viewMode}
            draft={draft}
            dirty={dirty}
            loading={documentLoading}
            error={documentError}
            message={message}
            saveState={saveState}
            onViewModeChange={setViewMode}
            onSave={saveDocument}
            onAssignmentChange={assignSelectedDocument}
            onDraftChange={(content) => { setDraft(content); setSaveState('idle') }}
            onRetryCatalog={loadCatalog}
            onRetryDocument={() => openDocument(selectedPath)}
          />
        </>
      )}
      <ProjectDocumentCommandMenu
        target={menuTarget}
        point={menuPoint}
        canEdit={!!catalog?.can_edit}
        canUndo={organization.canUndo}
        navigationMode={navigationMode}
        sectionSort={viewPreferences.sectionSort}
        documentSort={viewPreferences.documentSort}
        onCommand={(command, target) => {
          void executeCommand(command, target).catch((error) => setMessage(errorMessage(error, '执行项目文档操作失败')))
        }}
        onClose={() => setMenuTarget(null)}
      />
      <ProjectDocumentCommandDialog
        state={dialogState}
        manifest={organization.manifest}
        busy={commandBusy}
        onSubmit={(result) => { void submitCommandDialog(result) }}
        onClose={() => setDialogState(null)}
      />
    </div>
  )
}

function errorMessage(error: unknown, fallback: string) {
  return (error as { message?: string })?.message ?? fallback
}

function normalizeDocumentPath(path: string) {
  return path.trim().replace(/\\/g, '/')
}
