import { useEffect, useRef, useState } from 'react'
import { Copy, FileText, Reply, Forward, Star, ListChecks, AtSign, Pencil, History, Undo2, Info, EyeOff, Image, Download, Play, Pause } from 'lucide-react'
import { copyRichTextToClipboard, copyTextToClipboard, sanitizedRichHtmlFromElement } from '../../lib/clipboard'
import type { ActiveConversation, SocialMessage } from './socialMessageTypes'
import type { MessageEdit } from './groupMessageRevisions'
import GroupMessageRevisionActions from './GroupMessageRevisionActions'
import SocialDialog from './SocialDialog'
import SocialContextMenu, { type SocialMenuItem } from './SocialContextMenu'
import { attachmentKind, copyImageAttachment, downloadAttachment, messageMenuRequest, type SocialMenuRequest } from './socialMessageContext'
import { canRecall, isPending, isRecalled, messageEndpoint, messageText, socialRequest } from './socialChatOperations'
import styles from './SocialMessageMenu.module.css'
import tools from './SocialTools.module.css'

interface Props {
  conversation: ActiveConversation; message: SocialMessage; own: boolean; special?: boolean; copySourceId: string
  request: SocialMenuRequest | null; onMenu: (request: SocialMenuRequest | null) => void; favorite: boolean
  onQuote: () => void; onForward: () => void; onFavorite: () => void; onHide: () => void; onSelect: () => void; onMention?: () => void
  onSaved: (patch: MessageEdit) => void; onRecalled: () => void
}

export default function SocialMessageMenu(props: Props) {
  const { message, conversation, own, special } = props
  const request = props.request?.id === message.id ? props.request : null
  const [dialog, setDialog] = useState<'recall' | 'time' | null>(null)
  const [notice, setNotice] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const trigger = useRef<HTMLButtonElement>(null)
  const alive = useRef(true)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  useEffect(() => { if (notice) { const timer = setTimeout(() => setNotice(''), 6000); return () => clearTimeout(timer) } }, [notice])
  const usable = !isRecalled(message) && !isPending(message) && !special
  async function copy(rich = false) {
    const text = request?.selection || messageText(message)
    const result = rich ? await copyRichTextToClipboard(sanitizedRichHtmlFromElement(document.getElementById(props.copySourceId)), text) : await copyTextToClipboard(text)
    if (alive.current) setNotice(result && result !== 'failed' ? '已复制' : '复制失败，请选中文字复制')
  }
  async function mediaAction(action: () => Promise<unknown>, success: string) {
    try { await action(); if (alive.current) setNotice(success) }
    catch (failure) { if (alive.current) setNotice(failure instanceof Error && /[\u4e00-\u9fff]/.test(failure.message) ? failure.message : '操作失败，请检查网络后重试；媒体也可通过附件下载入口保存') }
  }
  async function recall() {
    if (busy) return
    setBusy(true); setError('')
    try {
      await socialRequest(`${messageEndpoint(conversation)}/${encodeURIComponent(message.id)}`, { method: 'DELETE' })
      if (alive.current) { props.onRecalled(); setDialog(null); setNotice('已撤回') }
    } catch (failure) { if (alive.current) setError((failure as Error).message) }
    finally { if (alive.current) setBusy(false) }
  }
  function render(edit?: () => void, history?: () => void) {
    const common: SocialMenuItem[] = [], revisions: SocialMenuItem[] = [], details: SocialMenuItem[] = []
    if (usable && request) {
      const attachment = request.media ? message.attachments?.[Number(request.media.dataset.socialAttachment)] : undefined
      if (attachment) {
        const kind = attachmentKind(attachment)
        if (kind === 'image') {
          const preview = request.media?.querySelector<HTMLButtonElement>('[data-preview-image]')
          if (preview) common.push({ label: '查看图片', icon: <Image />, action: () => preview.click() })
          common.push({ label: '复制图片', icon: <Copy />, action: () => void mediaAction(() => copyImageAttachment(attachment), '已复制图片') })
        }
        if (kind === 'audio') {
          const audio = request.media?.querySelector('audio')
          if (audio) common.push({ label: audio.paused ? '播放语音' : '暂停语音', icon: audio.paused ? <Play /> : <Pause />,
            action: () => { if (audio.paused) void mediaAction(() => audio.play(), ''); else audio.pause() } })
          if (attachment.transcription) common.push({ label: '复制转写文字', icon: <Copy />, action: () => void mediaAction(async () => {
            if (!await copyTextToClipboard(attachment.transcription!)) throw new Error('复制失败，请重试')
          }, '已复制转写文字') })
        }
        common.push({ label: kind === 'image' ? '下载图片' : kind === 'audio' ? '下载语音' : '下载文件', icon: <Download />,
          action: () => void mediaAction(() => downloadAttachment(attachment), '已交给下载管理器') })
      }
      if (request.selection || (!attachment && message.content.trim())) common.push({ label: request.selection ? '复制选中文字' : '复制', icon: <Copy />, action: () => void copy() })
      if (!attachment && !request.selection && document.getElementById(props.copySourceId)?.querySelector('p,pre,blockquote,ul,ol,table')) common.push({ label: '复制富文本', icon: <FileText />, action: () => void copy(true) })
      common.push({ label: '引用', icon: <Reply />, action: props.onQuote }, { label: '转发…', icon: <Forward />, action: props.onForward },
        { label: props.favorite ? '取消收藏' : '收藏', hint: '仅此设备', icon: <Star />, action: props.onFavorite })
      if (edit) revisions.push({ label: '编辑', icon: <Pencil />, action: edit })
      if (history) revisions.push({ label: '修改记录', icon: <History />, action: history })
      if (canRecall(message, own)) revisions.push({ label: '撤回…', icon: <Undo2 />, action: () => { setError(''); setDialog('recall') } })
      details.push({ label: '多选', icon: <ListChecks />, action: props.onSelect })
      if (props.onMention) details.push({ label: '@ 发送者', icon: <AtSign />, action: props.onMention })
    }
    details.push({ label: '详细信息', icon: <Info />, action: () => setDialog('time') },
      { label: '隐藏消息', hint: '仅此设备', icon: <EyeOff />, action: props.onHide })
    return <>
      <div className={styles.actions}>
        {history && <button type="button" className={styles.edited} onClick={history}>已编辑 · {(message.revision ?? 1) - 1} 次</button>}
        <button type="button" ref={trigger} className={styles.more} aria-label="更多消息操作" aria-haspopup="menu" aria-expanded={!!request}
          onClick={() => { const node = trigger.current!, rect = node.getBoundingClientRect(); props.onMenu(request ? null : messageMenuRequest(message.id, node, rect.left, rect.bottom + 4, node)) }}>···</button>
        {notice && <span className={styles.notice} role="status">{notice}</span>}
      </div>
      {request && <SocialContextMenu request={request} groups={[common, revisions, details]} onClose={() => props.onMenu(null)} />}
    </>
  }
  return <>
    {conversation.kind === 'group' && usable ? <GroupMessageRevisionActions groupId={conversation.id} message={message} own={own} onSaved={props.onSaved}
      renderActions={actions => render(actions.editable ? actions.edit : undefined, actions.edited ? actions.history : undefined)} /> : render()}
    {dialog && <SocialDialog title={dialog === 'recall' ? '撤回消息' : '消息详情'} onClose={() => setDialog(null)} busy={busy}
      footer={dialog === 'recall' ? <><button type="button" disabled={busy} onClick={() => setDialog(null)}>取消</button><button type="button" disabled={busy} onClick={() => void recall()}>{busy ? '正在撤回…' : '确认撤回'}</button></> : undefined}>
      {dialog === 'recall' ? <><p>撤回后，所有参与者将看到撤回提示。仅支持本人发送后 1 分钟内的消息。</p><pre>{messageText(message)}</pre></> : <><p>发送时间：{new Date(message.created_at).toLocaleString()}</p>{message.edited_at && <p>最后修改：{new Date(message.edited_at).toLocaleString()}</p>}<p className={tools.hint}>本人消息发送后 1 分钟内可撤回。收藏与隐藏只保存在此设备。</p></>}
      {error && <p className={tools.error} role="alert">{error}</p>}
    </SocialDialog>}
  </>
}
