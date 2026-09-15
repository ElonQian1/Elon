import { useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react'
import type { User } from '../../store/auth'
import type { ActiveConversation, SocialMessage } from './socialMessageTypes'
import { conversationId } from './socialChatCache'
import { quoteText, sendSocialMessage, socialRequest } from './socialChatOperations'
import { useSocialAttachments } from './useSocialAttachments'
import { socialLocalId } from './socialLocalId'
import SocialDialog from './SocialDialog'
import styles from './SocialTools.module.css'
import pageStyles from './FriendsPage.module.css'

export interface QuoteRequest { conversation: string; message: SocialMessage; author: string; nonce: number }
interface Props {
  conversation: ActiveConversation; title: string; me: User; input: string; setInput: Dispatch<SetStateAction<string>>
  setMessages: Dispatch<SetStateAction<SocialMessage[]>>; onSent: () => void; quote: QuoteRequest | null
}
interface Member { id: string; display_name: string }

export default function SocialComposer({ conversation, title, me, input, setInput, setMessages, onSent, quote }: Props) {
  const key = conversationId(conversation)
  const currentKey = useRef(key); currentKey.current = key
  const textarea = useRef<HTMLTextAreaElement>(null)
  const chooser = useRef<HTMLInputElement>(null)
  const mounted = useRef(true)
  const inFlight = useRef(new Set<string>())
  const memberEpoch = useRef(0)
  const [quotes, setQuotes] = useState<Record<string, QuoteRequest | undefined>>({})
  const [busy, setBusy] = useState<Record<string, boolean>>({})
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [members, setMembers] = useState<Member[] | null>(null)
  const [memberError, setMemberError] = useState('')
  const [memberLoading, setMemberLoading] = useState(false)
  const [memberQuery, setMemberQuery] = useState('')
  const media = useSocialAttachments(me.id)
  const files = media.byConversation[key] ?? []
  const pendingQuote = quotes[key]
  const sending = !!busy[key]
  const blocked = files.some(file => file.status !== 'ready')
  useEffect(() => { mounted.current = true; return () => { mounted.current = false } }, [])
  useEffect(() => {
    if (quote) { setQuotes(old => ({ ...old, [quote.conversation]: quote })); if (quote.conversation === key) textarea.current?.focus() }
  }, [quote])
  useEffect(() => { memberEpoch.current++; setMembers(null); setMemberError(''); setMemberQuery('') }, [key])
  function showError(value: string, scope = key) { if (mounted.current) setErrors(old => ({ ...old, [scope]: value })) }
  function addFiles(files: File[]) { showError(media.add(conversation, files)) }
  async function loadMembers() {
    const request = ++memberEpoch.current
    setMembers([]); setMemberLoading(true); setMemberError('')
    try {
      const data = await socialRequest<{ members: Member[]; ai_members?: Member[] }>(`/api/me/groups/${encodeURIComponent(conversation.id)}/members`)
      if (currentKey.current === key && mounted.current && memberEpoch.current === request) setMembers([...data.members, ...(data.ai_members ?? [])].filter((m, i, all) => m.id !== me.id && all.findIndex(x => x.id === m.id) === i))
    } catch (error) { if (currentKey.current === key && mounted.current && memberEpoch.current === request) setMemberError((error as Error).message) }
    finally { if (currentKey.current === key && mounted.current && memberEpoch.current === request) setMemberLoading(false) }
  }
  async function send(event: React.FormEvent) {
    event.preventDefault()
    const text = input.trim()
    if (inFlight.current.has(key) || sending || blocked || (!text && !files.length)) return
    const content = pendingQuote ? `${quoteText(pendingQuote.message, pendingQuote.author)}\n\n${text}` : text
    if (Array.from(content).length > 4000) { showError('正文与引用合计不能超过 4000 字，请精简后发送'); return }
    inFlight.current.add(key)
    let id = ''
    try {
      const capturedFiles = [...files]
      const attachments = capturedFiles.map(file => file.attachment!)
      id = `tmp-${socialLocalId()}`
      setBusy(old => ({ ...old, [key]: true })); showError('')
      setInput(''); setQuotes(old => ({ ...old, [key]: undefined }))
      const optimistic: SocialMessage = { id, content, attachments, created_at: new Date().toISOString(), sender_user_id: me.id, sender_name: me.nickname || me.account, outgoing: true }
      setMessages(old => [...old, optimistic])
      const data = await sendSocialMessage(conversation, { content, attachments })
      if (!data.message?.id) throw new Error('发送结果未确认，请同步消息后检查')
      if (mounted.current) {
        setMessages(old => {
          const received = old.find(m => m.id === data.message.id) ?? data.message
          return old.filter(m => m.id !== received.id).map(m => m.id === id ? received : m)
        })
        media.remove(conversation, capturedFiles.map(file => file.id)); onSent()
      }
    } catch (failure) {
      if (mounted.current) {
        showError((failure as Error).message, key)
        setInput(old => old && old !== content ? `${content}\n${old}` : content)
        setMessages(old => old.filter(m => m.id !== id))
      }
    } finally { inFlight.current.delete(key); if (mounted.current) setBusy(old => ({ ...old, [key]: false })) }
  }
  return <section className={styles.compose} aria-label="消息编辑器" onDragOver={event => { if (event.dataTransfer.types.includes('Files')) event.preventDefault() }}
    onDrop={event => { if (event.dataTransfer.files.length) { event.preventDefault(); addFiles(Array.from(event.dataTransfer.files)) } }}>
    <div className={styles.toolbar}>
      <input ref={chooser} type="file" multiple hidden aria-label="选择聊天附件" onChange={event => { const files = Array.from(event.target.files ?? []); event.target.value = ''; addFiles(files) }} />
      <button type="button" disabled={sending} onClick={() => chooser.current?.click()}>图片 / 文件 / 语音</button>
      {conversation.kind === 'group' && <button type="button" onClick={() => void loadMembers()}>@ 群成员</button>}
      <span className={styles.hint}>可粘贴图片或拖入文件 · 每个最多 12 MB</span>
    </div>
    {pendingQuote && <div className={styles.quote} aria-label="待发送引用"><button type="button" className={styles.more} onClick={() => setQuotes(old => ({ ...old, [key]: undefined }))}>取消引用</button>{quoteText(pendingQuote.message, pendingQuote.author)}</div>}
    {files.length > 0 && <div className={styles.status}>{files.map(file => <div className={styles.attachment} key={file.id}>
      <span>{file.file.name} · {file.status === 'uploading' ? '上传中…' : file.status === 'error' ? '上传失败' : '待发送'}</span>
      {file.status === 'error' && <button type="button" title={file.error} onClick={() => void media.retry(conversation, file)}>重试上传</button>}
      <button type="button" disabled={sending} aria-label={`移除附件 ${file.file.name}`} onClick={() => media.remove(conversation, [file.id])}>×</button>
    </div>)}</div>}
    <form className={pageStyles.composer} onSubmit={event => void send(event)}>
      <textarea ref={textarea} className={pageStyles.composerInput} value={input} rows={2} placeholder={`发送消息到 ${title}…`}
        onChange={event => { setInput(event.target.value); event.target.style.height = '46px'; event.target.style.height = `${Math.min(event.target.scrollHeight,150)}px` }}
        onPaste={event => { const files = Array.from(event.clipboardData.files); if (files.length) { event.preventDefault(); addFiles(files) } }}
        onKeyDown={event => { if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing && event.keyCode !== 229) { event.preventDefault(); void send(event) } }} />
      <button type="submit" className={pageStyles.sendBtn} disabled={sending || blocked || (!input.trim() && !files.length)}>{sending ? '发送中…' : '发送'}</button>
    </form>
    {errors[key] && <p className={`${styles.error} ${styles.status}`} role="alert">{errors[key]}</p>}
    {members && <SocialDialog title="选择要 @ 的群成员" onClose={() => { memberEpoch.current++; setMembers(null) }}>
      <input aria-label="查找群成员" placeholder="查找群成员" value={memberQuery} onChange={event => setMemberQuery(event.target.value)} />
      {memberLoading && <p role="status">正在读取群成员…</p>}{memberError && <p className={styles.error} role="alert">{memberError} <button type="button" onClick={() => void loadMembers()}>重试</button></p>}
      {members.filter(m => m.display_name.toLowerCase().includes(memberQuery.toLowerCase())).map(member => <div className={styles.item} key={member.id}><button type="button" onClick={() => { setInput(old => `${old}${old && !/\s$/.test(old) ? ' ' : ''}@${member.display_name} `); setMembers(null); textarea.current?.focus() }}>{member.display_name}</button></div>)}
      {!memberLoading && !memberError && !members.length && <p>暂无可选择成员</p>}
    </SocialDialog>}
  </section>
}
