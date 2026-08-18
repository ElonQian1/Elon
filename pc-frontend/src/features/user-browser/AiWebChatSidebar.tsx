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
} from 'lucide-react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import { requestOfficialAiTab, requestReturnToAiChat } from './internalBrowserApi'
import styles from './AiWebChatSidebar.module.css'

export default function AiWebChatSidebar({ web }: { web: AiWebChatBackend }) {
  const busy = Boolean(web.controller.busyAction)
  const officialVisible = Boolean(web.controller.sessionState?.windowVisible)
  const officialProviderId = web.officialRequest?.providerId
  const officialProviderName = web.officialRequest?.providerName
  const officialOwnerKey = web.officialRequest?.ownerKey
  const directory = web.controller.navigationSnapshot
  const conversations = directory?.conversations ?? []
  const [query, setQuery] = useState('')
  const normalizedQuery = query.trim().toLocaleLowerCase()
  const filtered = normalizedQuery
    ? conversations.filter((item) => item.title.toLocaleLowerCase().includes(normalizedQuery))
    : conversations
  const pinned = filtered.filter((item) => item.pinned || /pinned|置顶/i.test(item.groupLabel))
  const recent = filtered.filter((item) => !item.pinned && !/pinned|置顶/i.test(item.groupLabel))
  const projects = directory?.projects ?? []
  const cachedConversations = web.controller.sessionState?.localConversations ?? []
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
  }, [cancelSyncRetry, web.provider?.id])

  useEffect(() => () => cancelSyncRetry(), [cancelSyncRetry])

  useEffect(() => {
    if (officialVisible && officialProviderId && officialProviderName && officialOwnerKey) {
      requestOfficialAiTab({
        providerId: officialProviderId,
        providerName: officialProviderName,
        ownerKey: officialOwnerKey,
      })
    }
  }, [officialOwnerKey, officialProviderId, officialProviderName, officialVisible])

  useEffect(() => {
    if (!web.userState.canConversationHistory
      || !web.controller.sessionOpen
      || busy) return
    const key = web.controller.sessionState?.windowLabel || web.provider.id
    if (autoSyncKey.current === key) return
    autoSyncKey.current = key
    syncDirectory()
  }, [
    busy,
    syncDirectory,
    web.controller.sessionOpen,
    web.controller.sessionState?.windowLabel,
    web.provider.id,
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
          <span><strong>新聊天</strong><small>在 {web.provider?.displayName || '网页 AI'} 新建会话</small></span>
        </button>
        <button type="button" onClick={() => void web.controller.openOfficial()} disabled={!web.ready || busy}>
          <MonitorUp size={16} />
          <span><strong>显示官方页（登录可选）</strong><small>仅检查限制、登录、验证或故障回退时显示</small></span>
        </button>
        <button
          type="button"
          onClick={() => requestReturnToAiChat(web.officialRequest)}
          disabled={!web.ready || busy}
        >
          <EyeOff size={16} />
          <span><strong>收起官方页到后台</strong><small>继续使用当前一龙聊天界面</small></span>
        </button>
      </section>
      <div className={styles.providerPane}>
        <div className={styles.heading}>聊天来源</div>
        <div className={styles.providerTabs} role="tablist" aria-label="网页 AI 来源">
          {web.providers.map((provider) => (
            <button
              className={styles.provider}
              data-active={provider.id === web.provider?.id}
              key={provider.id}
              type="button"
              role="tab"
              aria-selected={provider.id === web.provider?.id}
              onClick={() => web.selectProvider(provider.id)}
            >
              <span className={styles.logo}>{provider.id === 'chatgpt' ? '◎' : 'G'}</span>
              <span><strong>{provider.id === 'chatgpt' ? 'ChatGPT' : 'Google AI'}</strong></span>
            </button>
          ))}
        </div>
        {web.provider?.id === 'chatgpt' ? (
          <nav className={styles.directory} aria-label="ChatGPT 网页聊天项目与会话">
            <div className={styles.directoryTitle}>
              <span>ChatGPT 网页聊天</span>
              {directory?.collection && (
                <small>{directory.collection.complete ? '已完整同步' : '后台同步中'}</small>
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
              empty={directory ? '官网暂无可见置顶聊天' : '登录后可同步官网置顶聊天'}
              action="open_conversation"
              web={web}
            />
            <DirectorySection
              icon={<FolderClosed size={13} />}
              title="项目"
              items={projects}
              empty={directory ? '官网暂无可见项目' : '登录后可同步官网项目'}
              action="open_project"
              web={web}
            />
            <DirectorySection
              icon={<MessageSquare size={13} />}
              title="聊天"
              items={recent}
              empty={directory ? '官网暂无可见聊天' : '访客可直接聊天；登录后自动同步历史'}
              action="open_conversation"
              web={web}
            />
            {!conversations.length && cachedConversations.length > 0 && (
              <CachedConversationSection items={cachedConversations} web={web} />
            )}
          </nav>
        ) : (
          <section className={styles.googleSession} aria-label="Google AI 搜索会话">
            <div className={styles.sectionHeading}><MessageSquare size={13} /><span>搜索会话</span></div>
            <button type="button" data-active disabled>
              <strong>{web.controller.snapshot?.title || '新 AI 搜索'}</strong>
              <small>Google AI 模式当前网页会话</small>
            </button>
            <CachedConversationSection items={cachedConversations} web={web} />
          </section>
        )}
        <div className={styles.status} data-error={Boolean(web.controller.sessionState?.lastError)}>
          <strong>{web.userState.title}</strong>
          {web.ready && <small data-complete={web.historyWindow.complete}>{web.historyWindow.label}</small>}
          <span>
            {web.controller.sessionState?.contextReady === false
              ? web.contextSummary
              : web.controller.sessionState?.navigationCacheStatus === 'cached'
              ? '已立即显示本机缓存；正在后台同步官网会话与项目。'
              : directory?.collection && !directory.collection.complete
              ? `已显示 ${conversations.length} 个缓存/可见会话；完整官网目录正在后台同步。`
              : web.message || web.contextSummary || web.userState.detail}
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
          disabled={Boolean(web.controller.busyAction)}
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
  items?: Array<{
    id: string
    title: string
    path: string
    active: boolean
  }>
  empty: string
  action?: 'open_conversation' | 'open_project'
  web: AiWebChatBackend
}) {
  const visibleItems = items ?? []
  return (
    <section className={styles.directorySection}>
      <div className={styles.sectionHeading}>{icon}<span>{title}</span>{visibleItems.length > 0 && <em>{visibleItems.length}</em>}</div>
      {visibleItems.length ? visibleItems.map((item) => (
        <button
          className={styles.directoryItem}
          type="button"
          key={item.path}
          data-active={item.active}
          title={item.title}
          onClick={() => action && void web.controller.run(action, item.path)}
          disabled={Boolean(web.controller.busyAction)}
        >
          <span>{item.title}</span>
        </button>
      )) : <p className={styles.empty}>{empty}</p>}
    </section>
  )
}
