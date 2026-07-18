import {
  Archive,
  ArchiveRestore,
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Copy,
  ExternalLink,
  MoreHorizontal,
  Pencil,
  Pin,
  Plus,
  User,
} from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { formatTime } from '../../lib/utils'
import { displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import { LOCAL_TASK_PLACEHOLDER_TITLE, readableTaskTitle } from '../../lib/taskTitle'
import type { MemberConversationEntry } from './memberConversationApi'
import { buildMemberConversationDeepLink, copyTextToClipboard } from './memberConversationLinks'
import {
  cleanMemberConversationPrefs,
  memberConversationPrefsScope,
  readMemberConversationPrefs,
  writeMemberConversationPrefs,
} from './memberConversationPrefs'
import type { MemberConversationPrefs } from './memberConversationPrefs'
import styles from './MemberConversationList.module.css'

const COLLAPSED_CONVERSATION_LIMIT = 8
const CONVERSATION_EXPAND_BATCH = 8

interface Props {
  conversations: MemberConversationEntry[]
  selectedId: string | 'new' | null
  targetName: string
  isOwnTarget: boolean
  onOpen: (conversationId: string) => void
  onStartNew: () => void
  onResetTarget: () => void
}

export default function MemberConversationList({
  conversations,
  selectedId,
  targetName,
  isOwnTarget,
  onOpen,
  onStartNew,
  onResetTarget,
}: Props) {
  const [sectionCollapsed, setSectionCollapsed] = useState(false)
  const [archiveCollapsed, setArchiveCollapsed] = useState(true)
  const [visibleLimit, setVisibleLimit] = useState(COLLAPSED_CONVERSATION_LIMIT)
  const [openMenuId, setOpenMenuId] = useState('')
  const menuRef = useRef<HTMLDivElement | null>(null)
  const sectionTitle = isOwnTarget ? '我的会话' : `${targetName} 的会话`
  const scope = useMemo(() => conversationScope(conversations), [conversations])
  const scopeKey = memberConversationPrefsScope(scope.projectId, scope.targetUserId)
  const conversationIds = useMemo(() => conversations.map((conversation) => conversation.id), [conversations])
  const conversationIdsKey = conversationIds.join('\n')
  const [prefs, setPrefs] = useState<MemberConversationPrefs>(() => readMemberConversationPrefs())
  const rows = useMemo(() => buildRows(conversations, prefs), [conversations, prefs])
  const activeRows = rows.filter((row) => !row.archived)
  const archivedRows = rows.filter((row) => row.archived)
  const visibleActiveRows = useMemo(
    () => visibleConversationRows(activeRows, visibleLimit, selectedId),
    [activeRows, selectedId, visibleLimit],
  )
  const hiddenActiveCount = Math.max(0, activeRows.length - visibleActiveRows.length)
  const canCollapseVisibleRows = visibleLimit > COLLAPSED_CONVERSATION_LIMIT

  useEffect(() => {
    const next = cleanMemberConversationPrefs(
      readMemberConversationPrefs(scope.projectId, scope.targetUserId),
      conversationIds,
    )
    setPrefs(next)
    writeMemberConversationPrefs(scope.projectId, scope.targetUserId, next)
    setVisibleLimit(COLLAPSED_CONVERSATION_LIMIT)
  }, [scopeKey, conversationIdsKey])

  useEffect(() => {
    if (selectedId && archivedRows.some((row) => row.conversation.id === selectedId)) {
      setArchiveCollapsed(false)
    }
  }, [selectedId, archivedRows])

  useEffect(() => {
    if (!openMenuId) return
    function closeMenu(event: PointerEvent) {
      if (!menuRef.current?.contains(event.target as Node)) setOpenMenuId('')
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === 'Escape') setOpenMenuId('')
    }
    window.addEventListener('pointerdown', closeMenu)
    window.addEventListener('keydown', closeOnEscape)
    return () => {
      window.removeEventListener('pointerdown', closeMenu)
      window.removeEventListener('keydown', closeOnEscape)
    }
  }, [openMenuId])

  function savePrefs(update: (current: MemberConversationPrefs) => MemberConversationPrefs) {
    setPrefs((current) => {
      const next = cleanMemberConversationPrefs(update(current), conversationIds)
      writeMemberConversationPrefs(scope.projectId, scope.targetUserId, next)
      return next
    })
  }

  function togglePinned(conversationId: string) {
    savePrefs((current) => {
      const pinned = current.pinnedIds.includes(conversationId)
      return {
        ...current,
        pinnedIds: pinned
          ? current.pinnedIds.filter((id) => id !== conversationId)
          : [conversationId, ...current.pinnedIds],
      }
    })
    setOpenMenuId('')
  }

  function renameConversation(conversation: MemberConversationEntry, title: string) {
    const nextTitle = window.prompt('重命名对话', title)
    if (nextTitle === null) return
    const normalized = nextTitle.trim().slice(0, 34)
    savePrefs((current) => {
      const renamedTitles = { ...current.renamedTitles }
      if (normalized && normalized !== conversationDisplayTitle(conversation)) {
        renamedTitles[conversation.id] = normalized
      } else {
        delete renamedTitles[conversation.id]
      }
      return { ...current, renamedTitles }
    })
    setOpenMenuId('')
  }

  function toggleArchived(conversationId: string, archived: boolean) {
    savePrefs((current) => ({
      ...current,
      pinnedIds: current.pinnedIds.filter((id) => id !== conversationId),
      archivedIds: archived
        ? current.archivedIds.filter((id) => id !== conversationId)
        : [conversationId, ...current.archivedIds],
    }))
    setOpenMenuId('')
  }

  function copyConversationId(conversationId: string) {
    copyMenuText(conversationId, '复制会话 ID')
    setOpenMenuId('')
  }

  function copyDeepLink(conversation: MemberConversationEntry) {
    copyMenuText(buildMemberConversationDeepLink(conversation), '复制深度链接')
    setOpenMenuId('')
  }

  function openInNewWindow(conversation: MemberConversationEntry) {
    const url = buildMemberConversationDeepLink(conversation)
    if (url) window.open(url, '_blank', 'noopener')
    setOpenMenuId('')
  }

  function renderConversationRow(row: ConversationRow) {
    const conversation = row.conversation
    const failed = conversation.last_task_status === 'error' || conversation.last_task_status === 'failed'
    const active = conversation.id === selectedId
    const menuOpen = openMenuId === conversation.id
    return (
      <div
        key={conversation.id}
        className={[styles.itemRow, active ? styles.itemRowActive : ''].join(' ')}
      >
        <button
          type="button"
          className={styles.itemMain}
          onClick={() => onOpen(conversation.id)}
        >
          <span className={styles.itemTitleRow}>
            <span className={styles.itemTitle}>{row.title}</span>
            <span className={styles.itemBadges}>
              {row.pinned && <span className={styles.pinPill}>置顶</span>}
              {failed && <span className={styles.statusPill}>失败</span>}
            </span>
          </span>
          <span className={styles.itemMeta}>
            {formatConversationActivityTime(conversation)}
            {typeof conversation.message_count === 'number' && ` · ${conversation.message_count} 条`}
          </span>
        </button>
        <div className={styles.moreWrap} ref={menuOpen ? menuRef : null}>
          <button
            type="button"
            className={styles.moreBtn}
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            aria-label={`更多操作：${row.title}`}
            onClick={(event) => {
              event.stopPropagation()
              setOpenMenuId(menuOpen ? '' : conversation.id)
            }}
          >
            <MoreHorizontal size={15} strokeWidth={2.3} aria-hidden="true" />
          </button>
          {menuOpen && (
            <div className={styles.menu} role="menu">
              <button type="button" className={styles.menuItem} role="menuitem" onClick={() => togglePinned(conversation.id)}>
                <Pin size={13} strokeWidth={2.2} aria-hidden="true" />
                {row.pinned ? '取消置顶' : '置顶对话'}
              </button>
              <button type="button" className={styles.menuItem} role="menuitem" onClick={() => renameConversation(conversation, row.title)}>
                <Pencil size={13} strokeWidth={2.2} aria-hidden="true" />
                重命名对话
              </button>
              <button type="button" className={styles.menuItem} role="menuitem" onClick={() => toggleArchived(conversation.id, row.archived)}>
                {row.archived
                  ? <ArchiveRestore size={13} strokeWidth={2.2} aria-hidden="true" />
                  : <Archive size={13} strokeWidth={2.2} aria-hidden="true" />}
                {row.archived ? '取消归档' : '归档对话'}
              </button>
              <span className={styles.menuDivider} aria-hidden="true" />
              <button type="button" className={styles.menuItem} role="menuitem" onClick={() => copyConversationId(conversation.id)}>
                <Copy size={13} strokeWidth={2.2} aria-hidden="true" />
                复制会话 ID
              </button>
              <button type="button" className={styles.menuItem} role="menuitem" onClick={() => copyDeepLink(conversation)}>
                <Copy size={13} strokeWidth={2.2} aria-hidden="true" />
                复制深度链接
              </button>
              <button type="button" className={styles.menuItem} role="menuitem" onClick={() => openInNewWindow(conversation)}>
                <ExternalLink size={13} strokeWidth={2.2} aria-hidden="true" />
                在新窗口打开
              </button>
            </div>
          )}
        </div>
      </div>
    )
  }

  return (
    <section className={styles.section} aria-label={`${targetName} 的会话`}>
      <div className={styles.header}>
        <button
          aria-expanded={!sectionCollapsed}
          className={styles.headerToggle}
          onClick={() => setSectionCollapsed((value) => !value)}
          type="button"
        >
          {sectionCollapsed
            ? <ChevronRight size={13} strokeWidth={2.2} aria-hidden="true" />
            : <ChevronDown size={13} strokeWidth={2.2} aria-hidden="true" />}
          <span className={styles.headerCopy}>
            <span>{sectionTitle}</span>
            {!isOwnTarget && <em>以你的账号继续协助</em>}
          </span>
        </button>
        <div className={styles.headerMeta}>
          <small>{activeRows.length}</small>
        </div>
        <div className={styles.actions}>
          {!isOwnTarget && (
            <button
              className={styles.actionBtn}
              type="button"
              onClick={onResetTarget}
              title="回到我的会话"
              aria-label="回到我的会话"
            >
              <User size={14} strokeWidth={2.2} aria-hidden="true" />
            </button>
          )}
          {isOwnTarget && (
            <button
              className={[styles.actionBtn, selectedId === 'new' ? styles.actionBtnActive : ''].join(' ')}
              type="button"
              onClick={onStartNew}
              title="新建会话"
              aria-label="新建会话"
            >
              <Plus size={15} strokeWidth={2.4} aria-hidden="true" />
            </button>
          )}
        </div>
      </div>

      {!sectionCollapsed && activeRows.length === 0 && (
        <div className={styles.empty}>
          {isOwnTarget ? '发送第一条消息自动创建会话' : '该成员暂无可见会话'}
        </div>
      )}

      {!sectionCollapsed && (
        <>
          <div className={styles.list}>
            {visibleActiveRows.map(renderConversationRow)}
          </div>
          {(hiddenActiveCount > 0 || canCollapseVisibleRows) && (
            <div className={styles.listControls}>
              {hiddenActiveCount > 0 && (
                <button
                  className={styles.listToggleBtn}
                  type="button"
                  onClick={() => setVisibleLimit((limit) => Math.min(activeRows.length, limit + CONVERSATION_EXPAND_BATCH))}
                >
                  <ChevronDown size={13} strokeWidth={2.2} aria-hidden="true" />
                  <span>展开显示</span>
                  <small>{hiddenActiveCount}</small>
                </button>
              )}
              {canCollapseVisibleRows && (
                <button
                  className={styles.listToggleBtn}
                  type="button"
                  onClick={() => setVisibleLimit(COLLAPSED_CONVERSATION_LIMIT)}
                >
                  <ChevronUp size={13} strokeWidth={2.2} aria-hidden="true" />
                  <span>折叠显示</span>
                </button>
              )}
            </div>
          )}
        </>
      )}

      <div className={styles.archiveSection}>
        <button
          type="button"
          className={styles.archiveHeader}
          aria-expanded={!archiveCollapsed}
          onClick={() => setArchiveCollapsed((value) => !value)}
        >
          {archiveCollapsed
            ? <ChevronRight size={13} strokeWidth={2.2} aria-hidden="true" />
            : <ChevronDown size={13} strokeWidth={2.2} aria-hidden="true" />}
          <span>归档</span>
          <small>{archivedRows.length}</small>
        </button>
        {!archiveCollapsed && archivedRows.length === 0 && (
          <div className={styles.empty}>暂无归档会话</div>
        )}
        {!archiveCollapsed && archivedRows.length > 0 && (
          <div className={styles.list}>
            {archivedRows.map(renderConversationRow)}
          </div>
        )}
      </div>
    </section>
  )
}

function copyMenuText(text: string, title: string) {
  void copyTextToClipboard(text).then((copied) => {
    if (!copied) window.prompt(title, text)
  })
}

interface ConversationRow {
  conversation: MemberConversationEntry
  title: string
  pinned: boolean
  archived: boolean
  index: number
}

function buildRows(conversations: MemberConversationEntry[], prefs: MemberConversationPrefs): ConversationRow[] {
  const pinned = new Set(prefs.pinnedIds)
  const archived = new Set(prefs.archivedIds)
  return conversations.map((conversation, index) => ({
    conversation,
    title: prefs.renamedTitles[conversation.id] || conversationDisplayTitle(conversation),
    pinned: pinned.has(conversation.id),
    archived: archived.has(conversation.id),
    index,
  })).sort((left, right) => {
    if (left.archived !== right.archived) return left.archived ? 1 : -1
    if (left.pinned !== right.pinned) return left.pinned ? -1 : 1
    return left.index - right.index
  })
}

function visibleConversationRows(
  rows: ConversationRow[],
  visibleLimit: number,
  selectedId: string | 'new' | null,
): ConversationRow[] {
  if (rows.length <= COLLAPSED_CONVERSATION_LIMIT + 1 || visibleLimit >= rows.length) return rows
  const visibleIds = new Set<string>()
  for (const row of rows) {
    if (visibleIds.size < visibleLimit) visibleIds.add(row.conversation.id)
    if (shouldKeepConversationVisible(row, selectedId)) visibleIds.add(row.conversation.id)
  }
  return rows.filter((row) => visibleIds.has(row.conversation.id))
}

function shouldKeepConversationVisible(row: ConversationRow, selectedId: string | 'new' | null): boolean {
  if (row.pinned || row.conversation.id === selectedId) return true
  const status = String(row.conversation.last_task_status ?? '').toLowerCase()
  return ['queued', 'pending', 'running', 'in_progress', 'processing', 'failed', 'error'].includes(status)
}

function conversationScope(conversations: MemberConversationEntry[]) {
  const scoped = conversations.find((conversation) => conversation.project_id || conversation.user_id)
  return {
    projectId: scoped?.project_id ?? '',
    targetUserId: scoped?.user_id ?? '',
  }
}

function conversationDisplayTitle(conversation: MemberConversationEntry): string {
  if (conversation.title === LOCAL_TASK_PLACEHOLDER_TITLE) {
    return readableTaskTitle(conversation.last_message, '本机 Codex 任务')
  }
  const raw = displayMessageContentOrAttachment(conversation.title || conversation.last_message)
  if (!raw) return '新会话'

  const normalized = raw
    .replace(/^MCP\s+Display\s*/i, 'MCP 验收 ')
    .replace(/\bmcp_display_e2e_\d+_\d+\b/gi, '')
    .replace(/\bmcp_native_e2e_\d+_\d+(?:_[a-z]+)?\b/gi, '')
    .replace(/\bpch_[a-f0-9]+\b/gi, '')
    .replace(/\bforce-cli-parallel-[ab]-\d+\b/gi, '并行任务测试')
    .replace(/^Force\s+CLI\s+cancellation\s+smoke\s+test\.?.*/i, 'CLI 取消验证')
    .replace(/\bpost-publish-casual(?:-lookup)?-\d+\b/gi, '发布后验证')
    .replace(/\bparallel\s+real\s+([ab])\s+\d+\b/gi, '并行会话 $1')
    .replace(/\bsingle\s+node\s+lock\s+\d+\b/gi, '单节点锁定验证')
    .replace(/^MCP\s+Native\s+Absolute\s+Pub\S*/i, 'MCP 原生发布验证')
    .replace(/\s+/g, ' ')
    .replace(/[·\-\s]+$/g, '')
    .trim()

  if (normalized) return normalized.slice(0, 34)
  if (/mcp/i.test(raw)) return 'MCP 验收会话'
  if (/pch_[a-f0-9]+/i.test(raw) || raw.includes('项目频道')) return '项目频道会话'
  return raw.slice(0, 34)
}

function formatConversationActivityTime(conversation: MemberConversationEntry): string {
  const value = conversation.last_message_at || conversation.created_at || conversation.updated_at
  return value ? formatTime(value) : '暂无消息'
}
