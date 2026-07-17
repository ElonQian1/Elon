import { useEffect, useMemo, useRef } from 'react'
import {
  ArrowDown, ArrowUp, BookOpenCheck, Bot, CheckSquare2, ChevronDown, ChevronUp,
  Clipboard, FileInput, FilePenLine, FolderInput, FolderPlus, GitCompareArrows,
  Home, ListTree, Merge, MoreHorizontal, Palette, Pin, PinOff, Plus, RotateCcw,
  ShieldCheck, Sparkles, Star, StarOff, Trash2, Undo2,
  type LucideIcon,
} from 'lucide-react'

import type { ProjectDocumentEntry } from './projectDocumentModel'
import type { DocumentNavigationMode } from './projectDocumentArchitecture'
import type { DocumentSortMode, SectionSortMode } from './projectDocumentCommands'
import type { DocumentSection } from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'

export type ProjectDocumentCommandId =
  | 'open' | 'edit' | 'read' | 'toggle-selection' | 'copy-path' | 'copy-link'
  | 'new-root' | 'new-child' | 'new-sibling' | 'edit-section' | 'move-parent'
  | 'move-top' | 'move-up' | 'move-down' | 'move-bottom' | 'merge-section' | 'delete-section'
  | 'assign-topic' | 'assign-governance' | 'restore-automatic' | 'pin' | 'unpin'
  | 'recommend' | 'unrecommend' | 'home-entrypoint' | 'section-entrypoint'
  | 'ai-section' | 'ai-document' | 'ai-governance' | 'ai-file-name'
  | 'undo' | `section-sort:${SectionSortMode}` | `document-sort:${DocumentSortMode}`

export type ProjectDocumentMenuTarget =
  | { kind: 'rail' }
  | { kind: 'page-list' }
  | { kind: 'section'; section: DocumentSection }
  | { kind: 'document'; document: ProjectDocumentEntry; selected: boolean; pinned: boolean; recommended: boolean }

interface Props {
  target: ProjectDocumentMenuTarget | null
  point: { x: number; y: number }
  canEdit: boolean
  canUndo: boolean
  navigationMode: DocumentNavigationMode
  sectionSort: SectionSortMode
  documentSort: DocumentSortMode
  onCommand: (command: ProjectDocumentCommandId, target: ProjectDocumentMenuTarget) => void
  onClose: () => void
}

interface MenuItem {
  id: ProjectDocumentCommandId
  label: string
  detail?: string
  icon: LucideIcon
  disabled?: boolean
  checked?: boolean
  danger?: boolean
}

export default function ProjectDocumentCommandMenu({
  target, point, canEdit, canUndo, navigationMode, sectionSort, documentSort, onCommand, onClose,
}: Props) {
  const menuRef = useRef<HTMLDivElement>(null)
  const groups = useMemo(() => target ? menuGroups(target, {
    canEdit, canUndo, navigationMode, sectionSort, documentSort,
  }) : [], [canEdit, canUndo, documentSort, navigationMode, sectionSort, target])

  useEffect(() => {
    if (!target) return
    const onPointerDown = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) onClose()
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
      if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return
      event.preventDefault()
      const buttons = [...(menuRef.current?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])]
      if (!buttons.length) return
      const current = buttons.indexOf(document.activeElement as HTMLButtonElement)
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1
        : event.key === 'ArrowDown' ? (current + 1 + buttons.length) % buttons.length
          : (current - 1 + buttons.length) % buttons.length
      buttons[next]?.focus()
    }
    window.addEventListener('pointerdown', onPointerDown)
    window.addEventListener('keydown', onKeyDown)
    window.setTimeout(() => menuRef.current?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus(), 0)
    return () => {
      window.removeEventListener('pointerdown', onPointerDown)
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [onClose, target])

  if (!target) return null
  const left = Math.max(8, Math.min(point.x, window.innerWidth - 292))
  const top = Math.max(8, Math.min(point.y, window.innerHeight - Math.min(620, groups.flat().length * 42 + 40)))
  return (
    <div className={styles.commandMenu} ref={menuRef} role="menu" aria-label="项目文档操作" style={{ left, top }}>
      {groups.map((group, groupIndex) => (
        <div className={styles.commandGroup} key={groupIndex}>
          {group.map((item) => (
            <button
              type="button"
              role="menuitem"
              key={item.id}
              disabled={item.disabled}
              data-danger={item.danger || undefined}
              onClick={() => { onCommand(item.id, target); onClose() }}
            >
              <item.icon size={15} />
              <span><strong>{item.label}</strong>{item.detail && <small>{item.detail}</small>}</span>
              {item.checked && <BookOpenCheck size={14} />}
            </button>
          ))}
        </div>
      ))}
      <footer><MoreHorizontal size={13} />右键、⋯、Shift+F10 或触摸长按均可打开</footer>
    </div>
  )
}

function menuGroups(target: ProjectDocumentMenuTarget, state: {
  canEdit: boolean
  canUndo: boolean
  navigationMode: DocumentNavigationMode
  sectionSort: SectionSortMode
  documentSort: DocumentSortMode
}): MenuItem[][] {
  if (target.kind === 'rail') return [
    [{ id: 'new-root', label: '新建一级分区', detail: '建立新的项目知识主题', icon: FolderPlus, disabled: !state.canEdit }],
    [
      { id: 'section-sort:manual', label: '项目手动顺序', detail: '显示共享知识架构中的顺序', icon: ListTree, checked: state.sectionSort === 'manual' },
      { id: 'section-sort:name', label: '按名称显示', detail: '仅影响我的界面', icon: ArrowDown, checked: state.sectionSort === 'name' },
      { id: 'section-sort:count', label: '按文档数显示', detail: '仅影响我的界面', icon: ArrowUp, checked: state.sectionSort === 'count' },
    ],
    [{ id: 'undo', label: '撤销上一次架构操作', icon: Undo2, disabled: !state.canUndo }],
  ]
  if (target.kind === 'page-list') return [[
    { id: 'document-sort:manual', label: '项目手动顺序', detail: '显示共享清单中的页面顺序', icon: ListTree, checked: state.documentSort === 'manual' },
    { id: 'document-sort:name', label: '按标题显示', icon: ArrowDown, checked: state.documentSort === 'name' },
    { id: 'document-sort:path', label: '按路径显示', icon: FolderInput, checked: state.documentSort === 'path' },
    { id: 'document-sort:authority', label: '按权威性显示', icon: ShieldCheck, checked: state.documentSort === 'authority' },
  ]]
  if (target.kind === 'section') {
    const editable = state.canEdit && !!target.section.custom && state.navigationMode === 'knowledge'
    return [
      [{ id: 'open', label: '打开分区', icon: ListTree }],
      [
        { id: 'new-child', label: '新建子分区', icon: FolderPlus, disabled: !editable || (target.section.depth ?? 0) >= 3 },
        { id: 'new-sibling', label: '新建同级分区', icon: Plus, disabled: !state.canEdit || state.navigationMode !== 'knowledge' },
        { id: 'edit-section', label: '重命名与外观', detail: '名称、说明、颜色和图标', icon: Palette, disabled: !editable },
        { id: 'move-parent', label: '更改父分区', detail: '最多四层，自动防止循环', icon: FolderInput, disabled: !editable },
      ],
      [
        { id: 'move-top', label: '移到顶部', icon: ChevronUp, disabled: !editable },
        { id: 'move-up', label: '上移', icon: ArrowUp, disabled: !editable },
        { id: 'move-down', label: '下移', icon: ArrowDown, disabled: !editable },
        { id: 'move-bottom', label: '移到底部', icon: ChevronDown, disabled: !editable },
      ],
      [
        { id: 'section-entrypoint', label: '用当前文档作为分区入口', icon: Home, disabled: !editable },
        { id: 'ai-section', label: '让 AI 整理此分区', detail: '只按需读取本分区歧义材料', icon: Sparkles },
        { id: 'merge-section', label: '合并到其他分区', icon: Merge, disabled: !editable },
      ],
      [{ id: 'delete-section', label: '删除分区与子分区', detail: '不删除 Markdown', icon: Trash2, disabled: !editable, danger: true }],
    ]
  }
  return [
    [
      { id: 'open', label: '打开', icon: FileInput },
      { id: 'edit', label: '编辑 Markdown', icon: FilePenLine },
      { id: 'read', label: '阅读模式', icon: BookOpenCheck },
      { id: 'toggle-selection', label: target.selected ? '取消选择' : '加入批量选择', icon: CheckSquare2 },
    ],
    [
      { id: 'assign-topic', label: '移动到知识主题', detail: '只改变虚拟主题，不移动文件', icon: FolderInput, disabled: !state.canEdit },
      { id: 'assign-governance', label: '调整治理属性', detail: '不突破真实路径的权威上限', icon: ShieldCheck, disabled: !state.canEdit },
      { id: 'restore-automatic', label: '恢复自动分类', icon: RotateCcw, disabled: !state.canEdit },
    ],
    [
      { id: 'move-top', label: '页面移到顶部', detail: '写入项目共同顺序', icon: ChevronUp, disabled: !state.canEdit },
      { id: 'move-up', label: '页面上移', icon: ArrowUp, disabled: !state.canEdit },
      { id: 'move-down', label: '页面下移', icon: ArrowDown, disabled: !state.canEdit },
      { id: 'move-bottom', label: '页面移到底部', icon: ChevronDown, disabled: !state.canEdit },
    ],
    [
      { id: target.pinned ? 'unpin' : 'pin', label: target.pinned ? '取消固定' : '固定到顶部', icon: target.pinned ? PinOff : Pin, disabled: !state.canEdit },
      { id: target.recommended ? 'unrecommend' : 'recommend', label: target.recommended ? '移出推荐阅读' : '加入推荐阅读', icon: target.recommended ? StarOff : Star, disabled: !state.canEdit },
      { id: 'home-entrypoint', label: '设为知识首页入口', icon: Home, disabled: !state.canEdit },
      { id: 'section-entrypoint', label: '设为当前主题入口', icon: ListTree, disabled: !state.canEdit || state.navigationMode !== 'knowledge' },
    ],
    [
      { id: 'ai-document', label: '让 AI 整理这篇文档', icon: Bot },
      { id: 'ai-governance', label: '让 AI 评估提权', detail: '检查路径上限、冲突和替代关系', icon: GitCompareArrows },
      { id: 'ai-file-name', label: '让 AI 建议重命名或移动', detail: '使用安全 Git 双提交事务', icon: Sparkles },
    ],
    [
      { id: 'copy-path', label: '复制项目路径', icon: Clipboard },
      { id: 'copy-link', label: '复制 Markdown 链接', icon: Clipboard },
    ],
  ]
}
