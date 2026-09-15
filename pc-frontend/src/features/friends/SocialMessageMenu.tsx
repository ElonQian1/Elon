import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { copyRichTextToClipboard, copyTextToClipboard, sanitizedRichHtmlFromElement } from '../../lib/clipboard'
import type { ActiveConversation, SocialMessage } from './socialMessageTypes'
import type { MessageEdit } from './groupMessageRevisions'
import GroupMessageRevisionActions from './GroupMessageRevisionActions'
import SocialDialog from './SocialDialog'
import { canRecall, isPending, isRecalled, messageEndpoint, messageText, socialRequest } from './socialChatOperations'
import styles from './SocialTools.module.css'

export interface SocialMenuRequest { id: string; x: number; y: number; nonce: number }
interface Props {
  conversation: ActiveConversation; message: SocialMessage; own: boolean; special?: boolean; copySourceId: string
  request: SocialMenuRequest | null; favorite: boolean
  onQuote: () => void; onForward: () => void; onFavorite: () => void; onHide: () => void; onSelect: () => void; onMention?: () => void
  onSaved: (patch: MessageEdit) => void; onRecalled: () => void
}
interface Item { label: string; action: () => void; disabled?: boolean }

export default function SocialMessageMenu(props: Props) {
  const { message, conversation, own, request, special } = props
  const [point, setPoint] = useState<{ x: number; y: number } | null>(null)
  const [dialog, setDialog] = useState<'recall' | 'time' | null>(null)
  const [notice, setNotice] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const trigger = useRef<HTMLButtonElement>(null)
  const menu = useRef<HTMLDivElement>(null)
  const alive = useRef(true)
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  useEffect(() => { setPoint(request?.id === message.id ? { x: request.x, y: request.y } : null) }, [request, message.id])
  useLayoutEffect(() => {
    if (!point || !menu.current) return
    const node = menu.current, rect = node.getBoundingClientRect()
    node.style.left = `${Math.max(8, Math.min(point.x, innerWidth - rect.width - 8))}px`
    node.style.top = `${Math.max(8, Math.min(point.y, innerHeight - rect.height - 8))}px`
    node.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus()
  }, [point])
  useEffect(() => {
    if (!point) return
    const outside = (event: PointerEvent) => { if (!menu.current?.contains(event.target as Node)) setPoint(null) }
    const close = () => setPoint(null)
    document.addEventListener('pointerdown', outside)
    window.addEventListener('resize', close)
    return () => { document.removeEventListener('pointerdown', outside); window.removeEventListener('resize', close) }
  }, [point])
  function dismiss() { setPoint(null); trigger.current?.focus() }
  const usable = !isRecalled(message) && !isPending(message) && !special
  async function copy(rich = false) {
    const text = messageText(message)
    const result = rich ? await copyRichTextToClipboard(sanitizedRichHtmlFromElement(document.getElementById(props.copySourceId)), text) : await copyTextToClipboard(text)
    if (alive.current) setNotice(result && result !== 'failed' ? '已复制' : '复制失败，请选中文字复制')
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
    const items: Item[] = []
    if (usable) {
      items.push({ label: '复制', action: () => void copy() }, { label: '复制富文本', action: () => void copy(true) },
        { label: '引用回复', action: props.onQuote }, { label: '转发…', action: props.onForward },
        { label: props.favorite ? '取消本机收藏' : '收藏到本机', action: props.onFavorite }, { label: '多选', action: props.onSelect })
      if (props.onMention) items.push({ label: '@ 发送者', action: props.onMention })
      if (edit) items.push({ label: '编辑消息', action: edit })
      if (history) items.push({ label: '查看修改记录', action: history })
    }
    if (own && !isRecalled(message) && !isPending(message) && !special) items.push({ label: canRecall(message, own) ? '撤回消息…' : '撤回（限发送后 1 分钟）', disabled: !canRecall(message, own), action: () => { setError(''); setDialog('recall') } })
    items.push({ label: '消息时间', action: () => setDialog('time') }, { label: '仅本机隐藏', action: props.onHide })
    return <>
      <div className={styles.buttons}>
        <button type="button" ref={trigger} className={styles.more} aria-label="更多消息操作" aria-haspopup="menu" aria-expanded={!!point} onClick={() => { const rect = trigger.current!.getBoundingClientRect(); setPoint({ x: rect.left, y: rect.bottom + 4 }) }}>···</button>
        {history && <button type="button" onClick={history}>已编辑 · {(message.revision ?? 1) - 1} 次</button>}
        {notice && <span className={styles.hint} role="status">{notice}</span>}
      </div>
      {point && createPortal(<div ref={menu} className={styles.menu} role="menu" aria-label="社交消息操作" style={{ left: point.x, top: point.y }} onKeyDown={event => {
        if (event.key === 'Escape' || event.key === 'Tab') { dismiss(); if (event.key === 'Escape') event.preventDefault() }
        if (['ArrowDown','ArrowUp','Home','End'].includes(event.key)) {
          event.preventDefault(); const buttons = Array.from(menu.current!.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'))
          const index = buttons.indexOf(document.activeElement as HTMLButtonElement)
          buttons[event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length]?.focus()
        }
      }}>{items.map(item => <button type="button" role="menuitem" key={item.label} disabled={item.disabled} onClick={() => { dismiss(); item.action() }}>{item.label}</button>)}</div>, document.body)}
    </>
  }
  return <>
    {conversation.kind === 'group' && usable ? <GroupMessageRevisionActions groupId={conversation.id} message={message} own={own} onSaved={props.onSaved}
      renderActions={actions => render(actions.editable ? actions.edit : undefined, actions.edited ? actions.history : undefined)} /> : render()}
    {dialog && <SocialDialog title={dialog === 'recall' ? '撤回消息' : '消息时间'} onClose={() => setDialog(null)} busy={busy}
      footer={dialog === 'recall' ? <><button type="button" disabled={busy} onClick={() => setDialog(null)}>取消</button><button type="button" disabled={busy} onClick={() => void recall()}>{busy ? '正在撤回…' : '确认撤回'}</button></> : undefined}>
      {dialog === 'recall' ? <><p>撤回后，所有参与者将看到撤回提示。仅支持本人发送后 1 分钟内的消息。</p><pre>{messageText(message)}</pre></> : <><p>{new Date(message.created_at).toLocaleString()}</p><p className={styles.hint}>{message.created_at}</p>{message.edited_at && <p>最后修改：{new Date(message.edited_at).toLocaleString()}</p>}</>}
      {error && <p className={styles.error} role="alert">{error}</p>}
    </SocialDialog>}
  </>
}
