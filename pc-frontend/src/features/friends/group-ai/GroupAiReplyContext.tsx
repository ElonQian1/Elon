import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { ArrowUpRight, MessageSquare, Settings2 } from 'lucide-react'
import { api } from '../../../api/client'
import { useAuthStore } from '../../../store/auth'
import MarkdownContent from '../../markdown/MarkdownContent'
import SocialDialog from '../SocialDialog'
import SocialAvatar from '../SocialAvatar'
import SocialMessageAttachments from '../SocialMessageAttachments'
import { TextSourceCard } from '../source-links/SourceLinkView'
import { contextPath, continuationDraft, prepareGroupContinuation, type GroupAiDiscussion, type GroupAiReplyMetadata, type GroupAiSources } from './groupAiContext'
import styles from './GroupAiReplyContext.module.css'

interface Props { owner: string; group: string; message: string; metadata: GroupAiReplyMetadata; part: 'footer' | 'sources' }
export default function GroupAiReplyContext(props: Props) {
  const { owner, group, message, metadata, part } = props
  const [open, setOpen] = useState(false)
  const [sources, setSources] = useState<GroupAiSources | null>(null)
  const [discussion, setDiscussion] = useState<GroupAiDiscussion | null>(null)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const mounted = useRef(true)
  const navigate = useNavigate()
  useEffect(() => { mounted.current = true; return () => { mounted.current = false } }, [])
  const current = () => mounted.current && useAuthStore.getState().user?.id === owner
  async function load(continueChat = false) {
    if (busy) return
    setOpen(true); setError(''); setBusy(true); setSources(null); setDiscussion(null)
    try {
      if (continueChat) {
        const result = await api.get<GroupAiDiscussion>(contextPath(group, message) + '/ai-context', { cache: 'no-store' })
        if (current() && result.group_id === group) { continuationDraft(result); setDiscussion(result) }
      } else {
        const result = await api.get<GroupAiSources>(contextPath(group, message) + '/ai-sources', { cache: 'no-store' })
        if (current() && result.group_id === group && result.message_id === message) setSources(result)
      }
    } catch { if (current()) setError('记录未开放、已撤回或网络暂不可用，请重试') }
    finally { if (current()) setBusy(false) }
  }
  async function share(allowed: boolean) {
    if (!sources || busy) return
    const previous = sources
    setBusy(true); setError(''); setSources({ ...sources, allow_continue: allowed })
    try {
      const next = await api.patch<GroupAiSources>(contextPath(group, message) + '/ai-sources', { allow_continue: allowed, version: sources.version })
      if (current()) setSources(next)
    } catch { if (current()) { setSources(previous); setError('设置未确认，请重新打开后重试') } }
    finally { if (current()) setBusy(false) }
  }
  function start() {
    if (!discussion || !current()) return
    const id = prepareGroupContinuation(owner, group, discussion.document.title, continuationDraft(discussion))
    navigate(`/ai?groupDiscussion=${encodeURIComponent(id)}`)
  }
  return <>
    {part === 'footer' ? metadata.provider === 'chatgpt_web' && <div className={styles.footer}>
      <button type="button" onClick={() => void load(true)}><ArrowUpRight size={16} />使用 ChatGPT 继续讨论</button>
      {metadata.requester_id === owner && <button type="button" aria-label="讨论分享设置" title="讨论分享设置" onClick={() => void load()}><Settings2 size={16} /></button>}
    </div> : <button type="button" className={styles.source} onClick={() => void load()}>
      <strong><MessageSquare size={16} />{metadata.source_count === 1 ? '引用的消息' : '群聊的聊天记录'}</strong>
      {metadata.previews.map((p, i) => <span key={i}>{p.sender_name}：{p.text || '[附件]'}</span>)}
      <small>{metadata.source_count} 条来源消息</small>
    </button>}
    {open && <SocialDialog title={discussion ? '使用 ChatGPT 继续讨论' : '群聊的聊天记录'} onClose={() => { setOpen(false); setDiscussion(null); setSources(null) }} footer={<>
      <button type="button" onClick={() => setOpen(false)}>返回群聊</button>
      {discussion && <button type="button" onClick={start}>打开我的 ChatGPT</button>}
      {error && <button type="button" onClick={() => void load(part === 'footer')}>重试</button>}
    </>}>
      {busy && <p role="status">正在读取记录…</p>}
      {error && <p role="alert">{error}</p>}
      {discussion && <p>将在你自己的 ChatGPT 中开始一段私人讨论，带入这些精选文字和 AI 回答，不上传附件原文件。后续内容不会自动发送到群里。</p>}
      {sources?.requester_id === owner && sources.provider === 'chatgpt_web' && <label className={styles.sharing}>
        <input type="checkbox" checked={sources.allow_continue} disabled={busy} onChange={e => void share(e.target.checked)} />允许群成员使用自己的 ChatGPT 继续讨论
        <small>仅分享所选记录、问题与本条回答。不包含其他私人聊天；关闭后不会删除别人已保存的内容。</small>
      </label>}
      {sources?.sources.map(source => <article className={styles.record} key={source.id}>
        <header><SocialAvatar userId={source.sender_user_id} name={source.sender_name || '群成员'} /><strong>{source.sender_name}</strong><time>{new Date(source.created_at).toLocaleString()}</time></header>
        <MarkdownContent content={source.content} copy={false} />
        {!source.recalled_at && <><TextSourceCard text={source.content} /><SocialMessageAttachments attachments={source.attachments} /></>}
      </article>)}
    </SocialDialog>}
  </>
}
