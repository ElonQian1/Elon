import { useEffect, useState, type FormEvent } from 'react'
import { FolderInput, FolderPlus, Merge, Palette, ShieldAlert } from 'lucide-react'

import {
  SYSTEM_DOCUMENT_SECTIONS,
  customSectionKey,
  type CustomDocumentSection,
  type DocumentSectionManifest,
} from './projectDocumentSections'
import { canCreateSectionUnder, canMoveSectionToParent } from './projectDocumentCommands'
import styles from './ProjectDocumentsWorkspace.module.css'

export type ProjectDocumentDialogState =
  | { mode: 'create-section'; title: string; parentId: string }
  | { mode: 'edit-section'; title: string; section: CustomDocumentSection }
  | { mode: 'move-parent'; title: string; section: CustomDocumentSection }
  | { mode: 'merge-section'; title: string; section: CustomDocumentSection }
  | { mode: 'assign-topic'; title: string; paths: string[]; current?: string }
  | { mode: 'assign-governance'; title: string; paths: string[]; current?: string }

export interface ProjectDocumentDialogResult {
  mode: ProjectDocumentDialogState['mode']
  label?: string
  detail?: string
  color?: string
  icon?: string
  parentId?: string
  targetSectionKey?: string
  paths?: string[]
}

interface Props {
  state: ProjectDocumentDialogState | null
  manifest: DocumentSectionManifest
  busy: boolean
  onSubmit: (result: ProjectDocumentDialogResult) => void
  onClose: () => void
}

export default function ProjectDocumentCommandDialog({ state, manifest, busy, onSubmit, onClose }: Props) {
  const [label, setLabel] = useState('')
  const [detail, setDetail] = useState('')
  const [color, setColor] = useState('#7f8fb3')
  const [icon, setIcon] = useState('')
  const [selection, setSelection] = useState('')

  useEffect(() => {
    if (!state) return
    setLabel(state.mode === 'edit-section' ? state.section.label : '')
    setDetail(state.mode === 'edit-section' ? state.section.detail : '')
    setColor(state.mode === 'edit-section' ? state.section.color : '#7f8fb3')
    setIcon(state.mode === 'edit-section' ? state.section.icon : '')
    setSelection(state.mode === 'create-section' ? state.parentId
      : state.mode === 'move-parent' ? state.section.parent_id
        : state.mode === 'assign-topic' || state.mode === 'assign-governance' ? state.current ?? '' : '')
  }, [state])

  if (!state) return null
  const sectionOptions = manifest.sections.filter((section) => {
    if (state.mode === 'create-section') return canCreateSectionUnder(manifest.sections, section.id)
    if (state.mode === 'move-parent') return section.id !== state.section.id
      && canMoveSectionToParent(manifest.sections, state.section.id, section.id)
    return !('section' in state) || section.id !== state.section.id
  })
  const isSectionEditor = state.mode === 'create-section' || state.mode === 'edit-section'
  const isParentPicker = state.mode === 'create-section' || state.mode === 'move-parent'
  const isTopicPicker = state.mode === 'assign-topic'
  const isGovernancePicker = state.mode === 'assign-governance'
  const isMergePicker = state.mode === 'merge-section'
  const submitDisabled = busy
    || (isSectionEditor && !label.trim())
    || ((isTopicPicker || isGovernancePicker || isMergePicker) && !selection)

  function submit(event: FormEvent) {
    event.preventDefault()
    if (submitDisabled) return
    onSubmit({
      mode: state!.mode,
      label: label.trim(),
      detail: detail.trim(),
      color,
      icon: icon.trim(),
      parentId: isParentPicker ? selection : undefined,
      targetSectionKey: isTopicPicker || isGovernancePicker || isMergePicker
        ? selection
        : undefined,
      paths: 'paths' in state! ? state!.paths : undefined,
    })
  }

  const DialogIcon = state.mode === 'create-section' ? FolderPlus
    : state.mode === 'edit-section' ? Palette
      : state.mode === 'merge-section' ? Merge
        : state.mode === 'assign-governance' ? ShieldAlert : FolderInput
  return (
    <div className={styles.commandDialogBackdrop} role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose()
    }}>
      <form className={styles.commandDialog} role="dialog" aria-modal="true" aria-labelledby="project-document-dialog-title" onSubmit={submit}>
        <header>
          <span><DialogIcon size={18} /></span>
          <div><strong id="project-document-dialog-title">{state.title}</strong><small>{dialogSubtitle(state.mode)}</small></div>
        </header>

        {isSectionEditor && (
          <>
            <label>分区名称<input autoFocus maxLength={40} value={label} onChange={(event) => setLabel(event.target.value)} /></label>
            <label>用途说明<input maxLength={120} value={detail} onChange={(event) => setDetail(event.target.value)} placeholder="帮助用户和 AI 理解这里放什么" /></label>
            <div className={styles.dialogFieldGrid}>
              <label>颜色<input type="color" value={color} onChange={(event) => setColor(event.target.value)} /></label>
              <label>图标文字<input maxLength={32} value={icon} onChange={(event) => setIcon(event.target.value)} placeholder="可选" /></label>
            </div>
          </>
        )}

        {isParentPicker && (
          <label>父分区
            <select value={selection} onChange={(event) => setSelection(event.target.value)}>
              <option value="">一级分区（知识树根级）</option>
              {sectionOptions.map((section) => <option value={section.id} key={section.id}>{section.label}</option>)}
            </select>
          </label>
        )}

        {isTopicPicker && (
          <label>目标知识主题
            <select autoFocus value={selection} onChange={(event) => setSelection(event.target.value)}>
              <option value="">请选择</option>
              {manifest.sections.map((section) => <option value={customSectionKey(section.id)} key={section.id}>{section.label}</option>)}
            </select>
          </label>
        )}

        {isGovernancePicker && (
          <>
            <div className={styles.governanceWarning}><ShieldAlert size={17} /><span><strong>治理标记不会绕过路径权威上限</strong>归档、讨论和草稿路径不能仅靠菜单成为最高权威事实；需要提权时可继续让 AI 建议迁移或生成新的当前规范。</span></div>
            <label>治理状态
              <select autoFocus value={selection} onChange={(event) => setSelection(event.target.value)}>
                <option value="">请选择</option>
                {SYSTEM_DOCUMENT_SECTIONS.map((section) => <option value={section.key} key={section.key}>{section.label} — {section.detail}</option>)}
              </select>
            </label>
          </>
        )}

        {isMergePicker && (
          <label>合并到
            <select autoFocus value={selection} onChange={(event) => setSelection(event.target.value)}>
              <option value="">请选择目标分区</option>
              {sectionOptions.map((section) => <option value={customSectionKey(section.id)} key={section.id}>{section.label}</option>)}
            </select>
          </label>
        )}

        <footer>
          <button type="button" onClick={onClose}>取消</button>
          <button type="submit" disabled={submitDisabled}>{busy ? '保存中…' : submitLabel(state.mode)}</button>
        </footer>
      </form>
    </div>
  )
}

function dialogSubtitle(mode: ProjectDocumentDialogState['mode']) {
  if (mode === 'assign-governance') return '调整“能否作为当前事实”，不改变知识主题'
  if (mode === 'assign-topic') return '调整“这份文档讲什么”，不改变权威性'
  if (mode === 'merge-section') return '文档归类与子分区将一并转移，Markdown 不删除'
  if (mode === 'move-parent') return '更改层级关系，自动检查循环与四层上限'
  return '项目共同结构会写入 .elon/document-sections.json'
}

function submitLabel(mode: ProjectDocumentDialogState['mode']) {
  if (mode === 'merge-section') return '确认合并'
  if (mode === 'assign-governance' || mode === 'assign-topic') return '应用归类'
  return '保存'
}
