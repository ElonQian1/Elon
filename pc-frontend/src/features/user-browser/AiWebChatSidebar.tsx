import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import {
  EyeOff,
  FolderClosed,
  MessageSquare,
  MonitorUp,
  Pin,
  RefreshCw,
  Search,
  ShieldCheck,
  SquarePen,
  MoreHorizontal,
  ChevronDown,
  ChevronRight,
} from 'lucide-react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import { requestReturnToAiChat } from './internalBrowserApi'
import {
  localAiDirectoryAutoSyncKey,
  localAiDirectoryNeedsAutoSync,
} from './localAiDirectoryAutoSync'
import styles from './AiWebChatSidebar.module.css'
import { localAiDirectoryModel, type LocalAiDirectoryRow } from './localAiDirectoryModel'
import AiWebDirectoryProjectRow from './AiWebDirectoryProjectRow'
import AiWebProviderAvatar from './AiWebProviderAvatar'

export default function AiWebChatSidebar({ web }: { web: AiWebChatBackend }) {
  const busy = Boolean(web.controller.busyAction)
  const directory = web.controller.navigationSnapshot
  const conversations = directory?.conversations ?? []
  const [query, setQuery] = useState('')
  const { pinned, recent, projects } = localAiDirectoryModel(directory, query)
  const [menuOpen, setMenuOpen] = useState(false)
  const cachedConversations = web.controller.sessionState?.localConversations ?? []
  const directoryNeedsAutoSync = localAiDirectoryNeedsAutoSync({
    navigationEvent: directory,
    navigationUpdatedAtMs: web.controller.sessionState?.navigationUpdatedAtMs,
  })
  const autoSyncKey = useRef('')
  const syncRetryAttempt = useRef(0)
  const syncRetryTimer = useRef(0)
  const webRef = useRef(web)
  const syncDirectoryRef = useRef<(manual?: boolean) => void>(() => {})
  webRef.current = web

  const cancelSyncRetry = useCallback(() => {
    window.clearTimeout(syncRetryTimer.current)
    syncRetryTimer.current = 0
  }, [])

  const syncDirectory = useCallback((manual = false) => {
    const current = webRef.current
    if (!current.userState.canConversationHistory
      || !current.controller.sessionOpen
      || current.controller.busyAction) return
    if (manual) {
      syncRetryAttempt.current = 0
      cancelSyncRetry()
    }
    void current.controller.run('list_conversations').then((next) => {
      const result = next?.commandResult
      if (result?.action === 'list_conversations' && result.ok) {
        syncRetryAttempt.current = 0
        cancelSyncRetry()
        return
      }
      const delay = DIRECTORY_RETRY_DELAYS_MS[syncRetryAttempt.current]
      if (delay === undefined) return
      syncRetryAttempt.current += 1
      cancelSyncRetry()
      syncRetryTimer.current = window.setTimeout(() => syncDirectoryRef.current(), delay)
    })
  }, [cancelSyncRetry])
  syncDirectoryRef.current = syncDirectory

  useEffect(() => {
    cancelSyncRetry()
    autoSyncKey.current = ''
    syncRetryAttempt.current = 0
  }, [cancelSyncRetry, web.controller.sessionIdentity])

  useEffect(() => () => cancelSyncRetry(), [cancelSyncRetry])

  useEffect(() => {
    const key = localAiDirectoryAutoSyncKey({
      sessionIdentity: web.controller.sessionIdentity,
      windowLabel: web.controller.sessionState?.windowLabel,
      sessionOpen: web.controller.sessionOpen,
      navigationUpdatedAtMs: web.controller.sessionState?.navigationUpdatedAtMs,
      directoryComplete: directory?.collection?.complete,
    })
    if (!key) {
      autoSyncKey.current = ''
      syncRetryAttempt.current = 0
      cancelSyncRetry()
      return
    }
    if (!web.userState.canConversationHistory || busy) return
    if (autoSyncKey.current === key) return
    if (!directoryNeedsAutoSync) {
      // Keep the fresh cache visible without marking the identity as synced
      // forever. Idle session updates will re-evaluate the two-minute TTL.
      autoSyncKey.current = ''
      return
    }
    autoSyncKey.current = key
    syncDirectory()
  }, [
    busy,
    cancelSyncRetry,
    syncDirectory,
    web.controller.sessionIdentity,
    web.controller.sessionOpen,
    web.controller.sessionState?.navigationUpdatedAtMs,
    web.controller.sessionState?.windowLabel,
    directory,
    directoryNeedsAutoSync,
    web.userState.canConversationHistory,
  ])

  return (
    <>
      <section className={styles.quickActions} aria-label="网页 AI 会话操作">
        <button
          className={styles.newChat}
          type="button"
          onClick={() => void web.controller.run('new_conversation')}
          disabled={!web.userState.canNewConversation || busy}
        >
          <SquarePen size={17} />
          <span><strong>新聊天</strong></span>
        </button>
        <button type="button" title="ChatGPT 会话选项" aria-label="会话选项" aria-expanded={menuOpen}
          onClick={() => setMenuOpen(value => !value)}><MoreHorizontal size={18} /></button>
      </section>
      {menuOpen && <section className={styles.sessionMenu} aria-label="会话选项">
        <button type="button" onClick={() => void web.controller.openOfficial()} disabled={!web.ready || busy}>
          <MonitorUp size={16} />
          <span>打开官网 / 登录账号</span>
        </button>
        <button
          type="button"
          onClick={() => requestReturnToAiChat(web.officialRequest)}
          disabled={!web.ready || busy}
        >
          <EyeOff size={16} />
          <span>收起官网</span>
        </button>
      </section>}
      <div className={styles.providerPane}>
        <div className={styles.heading}>聊天来源</div>
        <div className={styles.providerTabs} role="tablist" aria-label="网页 AI 来源">
          {web.providers.map((provider) => {
            const activity = web.providerActivities[provider.id]
            return (
              <button
                className={styles.provider}
                data-active={provider.id === web.provider?.id}
                key={provider.id}
                type="button"
                role="tab"
                aria-selected={provider.id === web.provider?.id}
                onClick={() => web.selectProvider(provider.id)}
              >
                <span className={styles.logo}><AiWebProviderAvatar providerId={provider.id} /></span>
                <span className={styles.providerLabel}>
                  <strong>{provider.id === 'chatgpt' ? 'ChatGPT' : 'Google AI'}</strong>
                  {activity?.label && (
                    <small data-phase={activity.phase}>{activity.label}</small>
                  )}
                </span>
                {activity && activity.phase !== 'idle' && (
                  <i
                    className={styles.providerActivity}
                    data-phase={activity.phase}
                    aria-label={activity.label}
                    title={activity.label}
                  />
                )}
              </button>
            )
          })}
        </div>
        {web.provider?.id === 'chatgpt' ? (
          <nav className={styles.directory} aria-label="ChatGPT 网页聊天项目与会话">
            <div className={styles.directoryTitle}>
              <span>ChatGPT 网页聊天</span>
              {directory?.collection && (
                <small>{web.controller.busyAction === 'list_conversations' ? '同步中' : directory.collection.refreshSettled || directory.collection.complete ? '已更新' : '本机目录'}</small>
              )}
              <button
                type="button"
                title="同步官网侧栏"
                aria-label="同步 ChatGPT 官网侧栏"
                onClick={() => syncDirectory(true)}
                disabled={!web.userState.canConversationHistory || busy}
              >
                <RefreshCw size={13} className={web.controller.busyAction === 'list_conversations' ? styles.spinning : ''} />
              </button>
            </div>
            <label className={styles.search}>
              <Search size={13} aria-hidden="true" />
              <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索本机已同步聊天" />
              {query && <button type="button" onClick={() => setQuery('')} aria-label="清除聊天搜索">×</button>}
            </label>
            <DirectorySection
              icon={<Pin size={13} />}
              title="置顶"
              items={pinned}
              empty="暂无已同步的置顶"
              action="open_conversation"
              web={web}
            />
            <DirectorySection
              icon={<FolderClosed size={13} />}
              title="项目"
              items={projects}
              empty="尚无已同步项目"
              action="open_project"
              web={web}
            />
            <DirectorySection
              icon={<MessageSquare size={13} />}
              title="最近"
              items={recent}
              empty="尚无已同步聊天"
              action="open_conversation"
              web={web}
            />
            <button className={styles.directoryItem} type="button" disabled={busy || !web.userState.canConversationHistory}
              onClick={() => void web.controller.run('list_conversations', 'history')}>
              <ChevronDown size={16} /><span>加载更早会话</span>
            </button>
            {!conversations.length && cachedConversations.length > 0 && (
              <CachedConversationSection items={cachedConversations} web={web} />
            )}
          </nav>
        ) : (
          <section className={[styles.googleSession, styles.directory].join(' ')} aria-label="Google AI 搜索会话">
            <div className={styles.directoryTitle}>
              <span>Google AI 官网会话</span>
              {directory?.collection && (
                <small>{directory.collection.complete ? '已完整同步' : '后台同步中'}</small>
              )}
              <button
                type="button"
                title="同步 Google AI 官网会话"
                aria-label="同步 Google AI 官网会话"
                onClick={() => syncDirectory(true)}
                disabled={!web.userState.canConversationHistory || busy}
              >
                <RefreshCw size={13} className={web.controller.busyAction === 'list_conversations' ? styles.spinning : ''} />
              </button>
            </div>
            {conversations.length ? (
              <>
                <label className={styles.search}>
                  <Search size={13} aria-hidden="true" />
                  <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索 Google AI 官网会话" />
                  {query && <button type="button" onClick={() => setQuery('')} aria-label="清除聊天搜索">×</button>}
                </label>
                <DirectorySection
                  icon={<MessageSquare size={13} />}
                  title="搜索记录"
                  items={recent}
                  empty="Google AI 官网暂无可恢复会话"
                  action="open_conversation"
                  web={web}
                />
              </>
            ) : (
              <button type="button" data-active disabled>
                <strong>{web.controller.snapshot?.title || '新 AI 搜索'}</strong>
                <small>{web.userState.canConversationHistory ? '正在后台同步官网会话' : 'Google AI 模式当前网页会话'}</small>
              </button>
            )}
            <CachedConversationSection items={cachedConversations} web={web} />
          </section>
        )}
        <div className={styles.status} data-error={Boolean(web.controller.sessionState?.lastError)}>
          <strong>{web.userState.title}</strong>
          {web.ready && <small data-complete={web.historyWindow.complete}>{web.historyWindow.label}</small>}
          <span>
            {web.controller.newConversationRecoveryActive && web.message
              ? web.message
              : web.controller.sessionState?.contextReady === false
              ? web.contextSummary
              : web.controller.sessionState?.navigationCacheStatus === 'cached'
              ? directoryNeedsAutoSync
                ? '已立即显示本机缓存；正在后台同步官网会话与项目。'
                : '已立即显示本机缓存；官网目录最近已验证，无需重复同步。'
              : directory?.collection && !directory.collection.complete
              ? `本机已同步 ${conversations.length} 个会话。`
              : web.contextSummary || web.userState.detail}
          </span>
        </div>
        <p className={styles.privacy}><ShieldCheck size={14} />Cookie 仅保存在这台电脑的 WebView2 Profile</p>
      </div>
    </>
  )
}

function CachedConversationSection({
  items,
  web,
}: {
  items: Array<{ id: string; title: string; active: boolean; updatedAtMs: number }>
  web: AiWebChatBackend
}) {
  if (!items.length) return <p className={styles.empty}>还没有可恢复的本机会话缓存</p>
  return (
    <section className={styles.directorySection} aria-label="本机加密会话缓存">
      <div className={styles.sectionHeading}><MessageSquare size={13} /><span>本机最近会话</span><em>{items.length}</em></div>
      {items.map((item) => (
        <button
          className={styles.directoryItem}
          type="button"
          key={item.id}
          data-active={item.active}
          title={`${item.title} · 本机加密缓存`}
          onClick={() => void web.controller.openCachedConversation(item.id)}
        >
          <span>{item.title}</span>
        </button>
      ))}
    </section>
  )
}

const DIRECTORY_RETRY_DELAYS_MS = [2_000, 5_000, 15_000, 30_000] as const

function DirectorySection({
  icon,
  title,
  items,
  empty,
  action,
  web,
}: {
  icon: ReactNode
  title: string
  items?: LocalAiDirectoryRow[]
  empty: string
  action?: 'open_conversation' | 'open_project'
  web: AiWebChatBackend
}) {
  const visibleItems = items ?? []
  const [expanded, setExpanded] = useState(false)
  const shown = title === '项目' && !expanded ? visibleItems.slice(0, 5) : visibleItems
  return (
    <section className={styles.directorySection}>
      <div className={styles.sectionHeading}>{icon}<span>{title}</span>{visibleItems.length > 0 && <em>{visibleItems.length}</em>}</div>
      {visibleItems.length ? shown.map((item) => item.kind === 'project' ? <AiWebDirectoryProjectRow key={`${web.controller.sessionIdentity}:${item.id}`} item={item} web={web} /> : (
        <button
          className={styles.directoryItem}
          type="button"
          key={item.path}
          data-active={item.active}
          title={item.title}
          onClick={() => web.provider?.id === 'chatgpt'
              ? void web.controller.run('open_conversation', item.path)
              : action && void web.controller.run(action, item.path)}
        >
          <MessageSquare size={16} />
          <span>{item.title}</span>
        </button>
      )) : <p className={styles.empty}>{empty}</p>}
      {title === '项目' && visibleItems.length > 5 && <button className={styles.directoryItem}
        type="button" aria-expanded={expanded} onClick={() => setExpanded(value => !value)}>
        {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
        <span>{expanded ? '收起' : '展开显示'}</span>
      </button>}
    </section>
  )
}
