import { useCallback, useEffect, useRef, useState } from 'react'
import { Eye, RefreshCw, Rows3 } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  DeveloperTerminalEventDetail,
  DeveloperTerminalEventSummary,
} from './openCommerceClientTypes'
import { errorText, formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

export default function DeveloperInvocationEvents({
  testToken,
  refreshKey,
}: {
  testToken: string
  refreshKey: number
}) {
  const [events, setEvents] = useState<DeveloperTerminalEventSummary[]>([])
  const [cursor, setCursor] = useState<string | undefined>()
  const cursorRef = useRef<string | undefined>()
  const [hasMore, setHasMore] = useState(false)
  const [detail, setDetail] = useState<DeveloperTerminalEventDetail | null>(null)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const load = useCallback(async (pageCursor: string | undefined, append: boolean) => {
    const token = testToken.trim()
    if (!token) {
      setMessage('请先输入当前 App 的测试凭据。')
      return
    }
    setBusy(true)
    setMessage('')
    if (!append) setDetail(null)
    try {
      const page = await openCommerceClientApi.listDeveloperTerminalEvents(
        token,
        pageCursor,
      )
      setEvents((current) => append ? [...current, ...page.events] : page.events)
      const nextCursor = page.next_cursor ?? undefined
      setCursor(nextCursor)
      cursorRef.current = nextCursor
      setHasMore(page.has_more)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }, [testToken])

  useEffect(() => {
    setEvents([])
    setCursor(undefined)
    cursorRef.current = undefined
    setHasMore(false)
    setDetail(null)
    setMessage('')
  }, [testToken])

  useEffect(() => {
    if (refreshKey > 0 && testToken.trim()) {
      const checkpoint = cursorRef.current
      void load(checkpoint, Boolean(checkpoint))
    }
  }, [load, refreshKey, testToken])

  async function inspect(invocationId: string) {
    const token = testToken.trim()
    if (!token) return
    setBusy(true)
    setMessage('')
    try {
      setDetail(await openCommerceClientApi.getDeveloperTerminalEvent(token, invocationId))
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <span>
          <strong>App 调用结果流</strong>
          <small>按游标读取当前测试凭据所属 App 的终态；不是 Webhook，也未扣真实资金。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={() => load(cursorRef.current, Boolean(cursorRef.current))} disabled={busy} title="检查新结果">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        {[...events].reverse().map((event) => (
          <article key={event.event_id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{event.capability_key}</strong>
              <span style={badgeStyle(event.status === 'succeeded' ? 'neutral' : 'warn')}>
                {event.status === 'succeeded' ? '已完成' : '已失败'}
              </span>
            </header>
            <p style={commerceStyles.itemText}>商户 {event.merchant_id} · 幂等键 {event.idempotency_key}</p>
            <small style={commerceStyles.itemMeta}>
              {formatMicros(event.amount_micros, event.currency)} · 未扣真实资金 · {new Date(event.completed_at).toLocaleString()}
            </small>
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <span style={commerceStyles.itemMeta}>
                {event.result_available ? '含商户结果' : event.error_code ?? '无返回结果'}
              </span>
              <button style={actionStyle('secondary', busy)} type="button" onClick={() => inspect(event.invocation_id)} disabled={busy}>
                <Eye size={13} />查看结果
              </button>
            </footer>
          </article>
        ))}
        {events.length === 0 && <p className={base.empty}>输入测试凭据后刷新，即可读取该 App 的终态调用。</p>}
        {hasMore && (
          <button style={actionStyle('secondary', busy)} type="button" onClick={() => load(cursor, true)} disabled={busy}>
            <Rows3 size={13} />继续读取
          </button>
        )}
        {detail && <pre className={base.result}>{JSON.stringify(detail, null, 2)}</pre>}
      </div>
      {message && (
        <div style={{ ...commerceStyles.message, ...(message.includes('失败') ? errorMessageStyle : {}) }}>
          {message}
        </div>
      )}
    </section>
  )
}
