import { useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react'
import type { User } from '../../store/auth'
import { formatTime } from '../../lib/utils'
import { displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import { messageCopySourceId } from '../message-actions/MessageActions'
import MarkdownContent from '../markdown/MarkdownContent'
import { TextSourceCard } from './source-links/SourceLinkView'
import ArticleMessage from '../articles/ArticleMessage'
import { articleReference } from '../articles/articleApi'
import type { ActiveConversation, Friend, FriendGroup, SocialMessage } from './socialMessageTypes'
import { conversationId } from './socialChatCache'
import { isPending, isRecalled, messageText } from './socialChatOperations'
import { localMessageKey, useSocialLocalState, type SavedSocialMessage } from './socialLocalState'
import SocialAvatar from './SocialAvatar'
import SocialLinkCards from './SocialLinkCards'
import SocialMessageAttachments from './SocialMessageAttachments'
import SocialMessageMenu from './SocialMessageMenu'
import { messageMenuRequest, type SocialMenuRequest } from './socialMessageContext'
import SocialComposer, { type QuoteRequest } from './SocialComposer'
import SocialForwardDialog, { type SocialTarget } from './SocialForwardDialog'
import SocialConversationTools from './SocialConversationTools'
import styles from './FriendsPage.module.css'
import tools from './SocialTools.module.css'

interface Props {
  conversation: ActiveConversation; title: string; me: User; friend?: Friend; group?: FriendGroup
  messages: SocialMessage[]; setMessages: Dispatch<SetStateAction<SocialMessage[]>>
  input: string; setInput: Dispatch<SetStateAction<string>>; targets: SocialTarget[]
  loading: boolean; error: string; retry: () => void; onSent: () => void
}
const specialMessage = (m: SocialMessage) => !!articleReference(m.content) || m.content.startsWith('【一龙项目卡片】')
const selectable = (m: SocialMessage) => !isPending(m) && !isRecalled(m) && !specialMessage(m)

export default function SocialConversation(props: Props) {
  const { conversation, title, me, messages, setMessages, input, setInput } = props
  const key = conversationId(conversation)
  const [query, setQuery] = useState('')
  const [selectedIds, setSelectedIds] = useState<string[]>([])
  const [selectionMode, setSelectionMode] = useState(false)
  const [notice, setNotice] = useState('')
  const [quote, setQuote] = useState<QuoteRequest | null>(null)
  const [forward, setForward] = useState<SavedSocialMessage[] | null>(null)
  const [menu, setMenu] = useState<SocialMenuRequest | null>(null)
  const local = useSocialLocalState(me.id)
  const feed = useRef<HTMLDivElement>(null)
  const follow = useRef(true)
  const [newMessages, setNewMessages] = useState(false)
  useEffect(() => {
    setQuery(''); setSelectedIds([]); setSelectionMode(false); setMenu(null); setNotice(''); follow.current = true; setNewMessages(false)
    if (feed.current) feed.current.scrollTop = feed.current.scrollHeight
  }, [key])
  useEffect(() => {
    if (follow.current && feed.current) feed.current.scrollTop = feed.current.scrollHeight
    else setNewMessages(true)
  }, [messages[messages.length - 1]?.id])
  const saveItem = (message: SocialMessage): SavedSocialMessage => ({ conversation, title, message })
  const selected = messages.filter(m => selectedIds.includes(m.id) && selectable(m)).map(saveItem)
  const hidden = new Set(local.hidden)
  const favorites = new Set(local.favorites.map(item => localMessageKey(item.conversation, item.message.id)))
  const shown = messages.filter(m => !hidden.has(localMessageKey(conversation, m.id)) && (!query.trim() || `${m.sender_name || ''}\n${messageText(m)}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())))
  function select(message: SocialMessage) {
    setSelectionMode(true)
    if (selectedIds.includes(message.id)) setSelectedIds(old => old.filter(id => id !== message.id))
    else if (selectedIds.length < 20) setSelectedIds(old => [...old, message.id])
    else setNotice('一次最多选择 20 条消息，请分批操作')
  }
  function latest() { follow.current = true; setNewMessages(false); if (feed.current) feed.current.scrollTop = feed.current.scrollHeight }
  return <div className={tools.conversation}>
    <SocialConversationTools conversation={conversation} query={query} onQuery={setQuery} messages={messages}
      selected={selected} selectionMode={selectionMode} onClearSelection={() => { setSelectedIds([]); setSelectionMode(false); setNotice('') }}
      onForward={setForward} onSave={local.save} favorites={local.favorites} onRemoveFavorite={local.remove}
      hiddenCount={local.hidden.filter(id => id.startsWith(`${key}:`)).length} onRestore={() => local.restore(`${key}:`)} />
    {(notice || local.error) && <p className={tools.status} role="status">{local.error || notice}</p>}
    <div className={`${styles.feed} ${tools.feed}`} ref={feed} onScroll={() => { const node = feed.current!; follow.current = node.scrollHeight - node.clientHeight - node.scrollTop < 80; if (follow.current) setNewMessages(false) }}>
      {props.error && <p className={styles.syncStatus} role="status">{props.error} <button type="button" className={styles.syncRetry} onClick={props.retry}>重试</button></p>}
      {props.loading && <p className={styles.hint}>读取消息…</p>}
      {!props.loading && !props.error && !shown.length && <p className={styles.hint}>{query ? '已加载消息中没有匹配内容' : messages.length ? '本机会话中的消息已隐藏，可通过上方按钮恢复' : '还没有消息，发送第一条消息吧'}</p>}
      {shown.map(m => {
        const own = m.outgoing || m.sender_user_id === me.id
        const recalled = isRecalled(m)
        const name = own ? me.nickname || me.account : conversation.kind === 'group' ? m.sender_name || '群成员' : title
        const avatar = own ? me.avatar_data_url : conversation.kind === 'group' ? props.group?.members?.find(member => member.id === m.sender_user_id)?.avatar_data_url : props.friend?.avatar_data_url
        const content = recalled ? (own ? '你撤回了一条消息' : `${name} 撤回了一条消息`) : displayMessageContentOrAttachment(m.content)
        const copyId = messageCopySourceId(`friends:${key}`, m.id)
        const savedKey = localMessageKey(conversation, m.id)
        return <div key={`${key}:${m.id}`} data-message-id={m.id} tabIndex={0} aria-label={`${name}的消息`}
          className={[styles.msgRow, own ? styles.ownRow : '', selectedIds.includes(m.id) ? tools.selected : ''].join(' ')}
          onContextMenu={event => {
            const target = event.target as HTMLElement
            if (!target.closest('[data-social-content]') || target.closest('dialog')) return
            event.preventDefault(); event.stopPropagation()
            setMenu(messageMenuRequest(m.id, event.currentTarget, event.clientX, event.clientY, target))
          }}
          onKeyDown={event => { if ((event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) && !(event.target as HTMLElement).closest('dialog')) {
            event.preventDefault(); event.stopPropagation(); const rect = event.currentTarget.querySelector('[data-social-content]')!.getBoundingClientRect()
            setMenu(messageMenuRequest(m.id, event.currentTarget, rect.left + 20, rect.top + 20, event.currentTarget))
          } }}>
          {selectionMode && selectable(m) && <input type="checkbox" aria-label={`选择消息 ${m.id}`} checked={selectedIds.includes(m.id)} onChange={() => select(m)} />}
          <div className={styles.avatar}><SocialAvatar userId={m.sender_user_id} name={name} avatar={avatar} /></div>
          <div className={styles.msgBody} data-social-content>
            <div className={styles.msgMeta}><strong>{name}</strong><span>{formatTime(m.created_at)}</span></div>
            {content && (articleReference(content) ? <ArticleMessage content={content} /> : <div id={copyId} className={styles.msgContent}>
              {(!own || content.startsWith('>')) && /[#*`\[\]>|]/.test(content) ? <MarkdownContent content={content} copy={false} /> : content}
            </div>)}
            {!recalled && !m.attachments?.length && <TextSourceCard text={m.content} />}
            {!recalled && <SocialMessageAttachments attachments={m.attachments} />}
            {!recalled && !specialMessage(m) && <SocialLinkCards text={content} owner={`${me.id}:${key}`} />}
            <SocialMessageMenu conversation={conversation} message={m} own={own} special={specialMessage(m)} copySourceId={copyId} request={menu} onMenu={setMenu} favorite={favorites.has(savedKey)}
              onQuote={() => setQuote({ conversation: key, message: m, author: name, nonce: Date.now() })}
              onForward={() => setForward([saveItem(m)])} onSelect={() => select(m)}
              onFavorite={() => favorites.has(savedKey) ? local.remove(savedKey) : local.save([saveItem(m)])}
              onHide={() => { local.hide([savedKey]); setSelectedIds(old => old.filter(id => id !== m.id)) }}
              onMention={conversation.kind === 'group' && !own ? () => setInput(old => `${old}${old && !/\s$/.test(old) ? ' ' : ''}@${name} `) : undefined}
              onSaved={patch => setMessages(old => old.map(row => row.id === patch.id ? { ...row, ...patch } : row))}
              onRecalled={() => { setMessages(old => old.map(row => row.id === m.id ? { ...row, content: '', attachments: null, recalled_at: new Date().toISOString(), recalled_by: me.id } : row)); props.onSent() }} />
          </div>
        </div>
      })}
    </div>
    {newMessages && <button type="button" className={tools.latest} onClick={latest}>有新消息 · 回到最新</button>}
    <SocialComposer conversation={conversation} title={title} me={me} input={input} setInput={setInput} setMessages={setMessages} onSent={() => { latest(); props.onSent() }} quote={quote} />
    {forward && <SocialForwardDialog messages={forward} targets={props.targets} onClose={() => setForward(null)} onSent={props.retry} />}
  </div>
}
