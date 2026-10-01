import { useEffect, useRef, useState } from 'react'
import { getAuthToken } from '../../api/client'
import SocialDialog from '../friends/SocialDialog'
import useLocalAiOwnerIdentity from '../user-browser/useLocalAiOwnerIdentity'
import { isExchangeWebviewAvailable, openExchangeWebSession } from '../exchange-webview/exchangeWebviewApi'
import { gridReadPort, readGridAttachment, readGridSelection } from '../grid-chat/readGridChatSnapshot'
import { gridSource, sameGridSource, type GridSelection } from '../grid-chat/gridChatSnapshot'
import { matchesBinding, publishShare, readBinding, rememberShare } from './gridShareApi'
import { publicGrid, labels, value } from './gridShareModel'
import GridShareSummary from './GridShareSummary'
import { readSharePositions, type ShareReadSnapshot } from './readSharePositions'
import styles from './GridShare.module.css'
import GridSharePicker from './GridSharePicker'
import { displayProfit, profitTone } from './gridSharePickerModel'

interface Props { owner: string; group: string; title: string; previous?: string; onClose: () => void; onSent: () => void }
export function GridShareComposeDialog(props: Props) {
  const identity = useLocalAiOwnerIdentity()
  const ownerKey = identity.ownerKey
  const [selection, setSelection] = useState<GridSelection | null>(null)
  const [attachment, setAttachment] = useState<ShareReadSnapshot | null>(null)
  const [amounts, setAmounts] = useState(false), [withNote, setWithNote] = useState(false), [note, setNote] = useState('')
  const [busy, setBusy] = useState(false), [error, setError] = useState('')
  const [sending, setSending] = useState(false)
  const control = useRef(new AbortController()), token = useRef(getAuthToken()), flight = useRef(false)
  const attempted = useRef(false)
  const scope = `${props.owner}:${props.group}`
  const available = isExchangeWebviewAvailable() && ownerKey === props.owner && identity.source !== 'conflict'
  useEffect(() => { control.current = new AbortController(); return () => control.current.abort() }, [])
  const active = () => !control.current.signal.aborted && getAuthToken() === token.current
  async function read(id?: string) {
    if (!available || flight.current) return
    flight.current = true; attempted.current = false; setBusy(true); setError(''); setAttachment(null)
    try {
      const port = gridReadPort(ownerKey)
      if (id && selection) {
        const result = await readSharePositions(ownerKey, await readGridAttachment(port, selection, id, scope, active), active)
        if (active()) { setAttachment(result); setSelection(null) }
      } else {
        const result = await readGridSelection(port, active)
        if (props.previous) {
          const binding = readBinding(props.owner, props.group, props.previous)
          if (!binding || !matchesBinding(binding, result.source)) throw Error('此设备没有原网格的账户绑定，请在原分享设备和原币安账户更新；可另行分享新卡片')
          if (!result.rows.some(row => row.id === binding.strategy)) throw Error('原网格已不在运行列表，不能用另一条网格替代更新')
          const next = await readSharePositions(ownerKey, await readGridAttachment(port, result, binding.strategy, scope, active), active)
          if (active()) setAttachment(next)
        } else if (active()) setSelection(result)
      }
    } catch (reason) { if (active()) setError((reason as Error).message) }
    finally { flight.current = false; if (active()) setBusy(false) }
  }
  async function submit() {
    if (!attachment || flight.current || !active() || !available) return
    flight.current = true; setBusy(true); setSending(true); setError('')
    try {
      const source = gridSource(await gridReadPort(ownerKey).get())
      if (!active() || !sameGridSource(source, attachment.source)) throw Error('币安账号或页面已变化，请重新读取')
      if (!attempted.current && Date.now() - attachment.observedAtMs > 300_000) throw Error('快照已超过五分钟，请重新读取')
      attempted.current = true
      const result = await publishShare(props.group, publicGrid(attachment, amounts, withNote ? note : '', props.previous), control.current.signal)
      if (active()) { rememberShare(props.owner, props.group, result.snapshot_id, attachment); props.onSent(); props.onClose() }
    } catch (reason) { if (active()) setError((reason as Error).message) }
    finally { flight.current = false; if (active()) { setBusy(false); setSending(false) } }
  }
  return <SocialDialog busy={sending} title={props.previous ? '更新网格快照' : '分享网格到群聊'} onClose={() => { if (!sending) props.onClose() }} footer={<>
    <button type="button" disabled={sending} onClick={props.onClose}>取消</button><button type="button" disabled={!attachment || busy || !available} onClick={() => void submit()}>{busy ? '处理中…' : '确认发送到此群'}</button>
  </>}>
    <p>发送到：<strong>{props.title}</strong></p>
    {!available && <p className={styles.notice}>请在已登录同一一龙账号的 Win 客户端读取币安网格。</p>}
    <div className={styles.actions}><button type="button" disabled={busy || !available} onClick={() => void read()}>{attachment ? '重新读取' : '读取当前币安网格'}</button><button type="button" disabled={busy || !available} onClick={() => void openExchangeWebSession('binance', ownerKey).catch(() => setError('币安官网打开失败，请检查客户端'))}>打开币安官网</button></div>
    {busy && <p role="status">正在处理，请稍候…</p>}
    {selection && <GridSharePicker selection={selection} busy={busy} onSelect={id => void read(id)} />}
    {attachment && <section className={styles.privateMetrics} aria-label="本次读取的收益，仅供自己预览">
      <small>本人收益预览 · 公开范围由下方开关决定</small>
      <div>{(['profit', 'unrealizedPnl'] as const).map(key => {
        const metric = (attachment.facts as Record<string, string | null>)[key]
        return <span key={key}>{key === 'profit' ? '网格利润' : '未实现盈亏'}<strong data-tone={profitTone(metric)}>{displayProfit(metric)}{metric ? ' USDT' : ''}</strong></span>
      })}</div>
    </section>}
    {attachment && <><div className={styles.options}><label><input type="checkbox" checked={amounts} disabled={busy} onChange={e => setAmounts(e.target.checked)} />公开金额与数量</label><label><input type="checkbox" checked={withNote} disabled={busy} onChange={e => setWithNote(e.target.checked)} />附加个人说明</label>{withNote && <textarea aria-label="个人说明" maxLength={200} value={note} disabled={busy} onChange={e => setNote(e.target.value)} />}</div><div className={styles.card}><GridShareSummary grid={publicGrid(attachment, amounts)} /></div><p className={styles.notice}>仅分享此次快照。接收者无法操作你的币安账户；未读取的持仓与收益不会推算。{props.previous ? '将发送一张新卡片，旧内容保留。' : ''}</p></>}
    {error && <p role="alert" className={styles.error}>{error}</p>}
    {attachment?.positionNotice && <p role="status">{attachment.positionNotice}</p>}
    {attachment && <details><summary>检查全部公开字段</summary><dl className={styles.rows}>{Object.keys(publicGrid(attachment, amounts).fields).map(key => <div key={key} style={{ display: 'contents' }}><dt>{labels[key]}</dt><dd>{value(publicGrid(attachment, amounts), key)}</dd></div>)}</dl></details>}
  </SocialDialog>
}
export default function GridShareComposer(props: Omit<Props, 'onClose'>) {
  const [open, setOpen] = useState(false)
  return <><button type="button" onClick={() => setOpen(true)}>分享网格</button>{open && <GridShareComposeDialog key={`${props.owner}:${props.group}`} {...props} onClose={() => setOpen(false)} />}</>
}
