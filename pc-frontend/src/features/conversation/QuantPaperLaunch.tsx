import { useEffect, useRef, useState } from 'react'
import { AlertTriangle, CircleCheck, LoaderCircle, RefreshCw, ShieldCheck } from 'lucide-react'
import { api } from '../../api/client'
import { eskAssetApi, type EskQuantAllocationRequest } from '../assets/eskAssetApi'
import type { ProjectLandingPaperLaunch } from './types'
import styles from './QuantPaperLaunch.module.css'

const PROTOCOL = 'yilong.quant.paper_launch.v1'
const READINESS_SCHEMA = 'yilong.quant.paper_launch_readiness.v1'
const TICKET_SCHEMA = 'yilong.quant.paper_launch_ticket.v1'
const READY_SCHEMA = 'yilong.quant.paper_launch.ready.v1'
const GRANT_SCHEMA = 'yilong.quant.paper_launch.grant.v1'
const CONSUMED_SCHEMA = 'yilong.quant.paper_launch.consumed.v1'
const ERROR_SCHEMA = 'yilong.quant.paper_launch.error.v1'
const ALLOCATION_RECEIPT_MESSAGE_SCHEMA = 'yilong.quant.paper_launch.allocation_receipt.v1'
const ESK_PROJECTION_SCHEMA_V1 = 'yilong.esk.asset_projection.v1'
const ESK_PROJECTION_SCHEMA_V2 = 'yilong.esk.asset_projection.v2'
const ESK_PROJECTION_SCHEMAS = [ESK_PROJECTION_SCHEMA_V2, ESK_PROJECTION_SCHEMA_V1] as const
const ESK_ALLOCATION_AUTHORIZATION_SCHEMA = 'yilong.esk.quant_allocation_authorization.v1'
const READY_CAPABILITIES = [...ESK_PROJECTION_SCHEMAS, ESK_ALLOCATION_AUTHORIZATION_SCHEMA] as const
const NONCE_PATTERN = /^[A-Za-z0-9_-]{32,128}$/
const GRANT_PATTERN = /^ypg1\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/
const ESK_PROJECTION_PATTERN = /^yep[12]\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/
const ESK_ALLOCATION_AUTHORIZATION_PATTERN = /^yeqa1\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/
const ESK_ALLOCATION_RECEIPT_PATTERN = /^yqar1\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/

type Stage = 'checking' | 'unavailable' | 'ready' | 'authorizing' | 'loading_page' | 'waiting' | 'connected' | 'error'
type Readiness = {
  schema: typeof READINESS_SCHEMA
  protocol: typeof PROTOCOL
  enabled: boolean
  simulated: true
  reason: 'ready' | 'configuration_required' | 'configuration_invalid'
  launch_origin?: string
}
type LaunchTicket = {
  schema: typeof TICKET_SCHEMA
  protocol: typeof PROTOCOL
  launch_url: string
  access_token: string
  esk_asset_projection?: string
  esk_quant_allocation_authorization?: string
  expires_in: number
  simulated: true
}

export default function QuantPaperLaunch({
  integration,
  previewMode,
}: {
  integration: ProjectLandingPaperLaunch
  previewMode?: 'ready'
}) {
  const [stage, setStage] = useState<Stage>('checking')
  const [readiness, setReadiness] = useState<Readiness>()
  const [frameUrl, setFrameUrl] = useState('')
  const [message, setMessage] = useState('正在检查签名与量化 Web 配置。')
  const [allocationRequests, setAllocationRequests] = useState<EskQuantAllocationRequest[]>([])
  const [selectedRequestId, setSelectedRequestId] = useState('')
  const frameRef = useRef<HTMLIFrameElement>(null)
  const ticketRef = useRef<{
    grant: string
    eskProjection?: string
    eskAllocationAuthorization?: string
    expiresAtUnix: number
  }>()
  const expectedOriginRef = useRef('')
  const attemptIdRef = useRef('')
  const channelNonceRef = useRef('')
  const timeoutRef = useRef<number>()

  const clearTicket = () => {
    ticketRef.current = undefined
    if (timeoutRef.current !== undefined) window.clearTimeout(timeoutRef.current)
    timeoutRef.current = undefined
  }
  const clearSensitiveState = () => {
    clearTicket()
    channelNonceRef.current = ''
    attemptIdRef.current = ''
    expectedOriginRef.current = ''
  }

  const inspectReadiness = async () => {
    clearSensitiveState()
    setFrameUrl('')
    setStage('checking')
    setMessage('正在检查签名与量化 Web 配置。')
    if (previewMode === 'ready' && import.meta.env.DEV) {
      setReadiness({
        schema: READINESS_SCHEMA,
        protocol: PROTOCOL,
        enabled: true,
        simulated: true,
        reason: 'ready',
        launch_origin: 'https://quant.example',
      })
      setStage('ready')
      setMessage('配置已就绪。点击后授权只通过当前页面内存传递。')
      setAllocationRequests([])
      return
    }
    try {
      const value = await api.get<unknown>('/api/me/quant/paper-launch')
      const parsed = parseReadiness(value)
      if (!parsed) throw new Error('服务器返回了无法识别的启动配置。')
      setReadiness(parsed)
      if (!parsed.enabled) {
        setStage('unavailable')
        setMessage(parsed.reason === 'configuration_invalid'
          ? '启动配置无效，管理员需要检查量化地址与签名材料。'
          : '量化 Web 地址或签名材料尚未配置，当前保持关闭。')
        return
      }
      setStage('ready')
      setMessage('配置已就绪。点击后授权只通过当前页面内存传递。')
      void eskAssetApi.quantAllocationRequests().then((value) => {
        const launchable = value.requests.filter((item) => ['submitted', 'accepted'].includes(item.status))
        setAllocationRequests(launchable)
        setSelectedRequestId((current) => launchable.some((item) => item.request_id === current)
          ? current
          : launchable[0]?.request_id ?? '')
      }).catch(() => undefined)
    } catch (error) {
      setStage('error')
      setMessage(errorMessage(error, '暂时无法检查量化 Paper 启动配置。'))
    }
  }

  useEffect(() => {
    void inspectReadiness()
    return clearSensitiveState
    // Readiness is intentionally checked once for this project-home mount.
  }, [previewMode])

  useEffect(() => {
    const receive = (event: MessageEvent) => {
      const frameWindow = frameRef.current?.contentWindow
      const expectedOrigin = expectedOriginRef.current
      if (!frameWindow || !expectedOrigin || event.source !== frameWindow || event.origin !== expectedOrigin) return
      if (isReadyMessage(event.data)) {
        const ticket = ticketRef.current
        if (!ticket || ticket.expiresAtUnix <= Math.floor(Date.now() / 1000)) {
          failLaunch('短期授权已过期，请重新进入。')
          return
        }
        channelNonceRef.current = event.data.channel_nonce
        const projectionSchema = projectionCapability(ticket.eskProjection)
        const supportsEskProjection = projectionSchema !== null
          && event.data.capabilities?.includes(projectionSchema) === true
        const supportsEskAllocation = event.data.capabilities?.includes(ESK_ALLOCATION_AUTHORIZATION_SCHEMA) === true
        if (event.data.capabilities?.length && (!ticket.eskProjection || !supportsEskProjection)) {
          failLaunch('ESK 资产投影未能随本次授权生成，请重新进入。')
          return
        }
        if (ticket.eskAllocationAuthorization && !supportsEskAllocation) {
          failLaunch('量化页面未声明 ESK 申请接收能力，请重新进入。')
          return
        }
        const grantMessage: Record<string, unknown> = {
          schema: GRANT_SCHEMA,
          protocol: PROTOCOL,
          channel_nonce: event.data.channel_nonce,
          attempt_id: attemptIdRef.current,
          access_grant: ticket.grant,
          expires_at_unix: ticket.expiresAtUnix,
        }
        if (supportsEskProjection) grantMessage.esk_asset_projection = ticket.eskProjection
        if (ticket.eskAllocationAuthorization) {
          grantMessage.esk_quant_allocation_authorization = ticket.eskAllocationAuthorization
        }
        frameWindow.postMessage(grantMessage, expectedOrigin)
        ticketRef.current = undefined
        setStage('waiting')
        setMessage('授权已交给量化页面，正在读取本人模拟仓位。')
        return
      }
      if (isAllocationReceiptMessage(event.data, channelNonceRef.current, attemptIdRef.current)) {
        void eskAssetApi.applyQuantAllocationReceipt(event.data.receipt_token)
          .then((request) => {
            setAllocationRequests((items) => items.map((item) => item.request_id === request.request_id ? request : item))
            setMessage(request.status === 'released'
              ? '主项目已验签量化释放回执，对应 ESK 已恢复为可用。'
              : '主项目已验签量化接收回执，对应 ESK 继续作为 Paper 绑定占用。')
          })
          .catch((error) => setMessage(errorMessage(error, '量化回执同步失败；保留当前页面后可重试。')))
        return
      }
      if (!isTerminalMessage(event.data, channelNonceRef.current, attemptIdRef.current)) return
      clearTicket()
      if (event.data.schema === CONSUMED_SCHEMA) {
        setStage('connected')
        setMessage('已安全连接。下方量化页面正在展示本人 Paper 模拟仓位。')
      } else {
        setStage('error')
        setMessage('量化页面拒绝了这次授权，请重新进入。')
      }
    }
    window.addEventListener('message', receive)
    return () => window.removeEventListener('message', receive)
  }, [])

  const launch = async () => {
    if (stage !== 'ready' || !readiness?.launch_origin) return
    clearSensitiveState()
    setStage('authorizing')
    setMessage('正在为当前一龙账号签发五分钟 Paper 授权。')
    try {
      const value = await api.post<unknown>('/api/me/quant/paper-launches', {
        capabilities: [...READY_CAPABILITIES],
        ...(selectedRequestId ? { esk_quant_allocation_request_id: selectedRequestId } : {}),
      })
      const ticket = parseTicket(value)
      if (!ticket) throw new Error('服务器返回了无法识别的启动票据。')
      const target = normalizeLaunchUrl(ticket.launch_url)
      if (!target || target.origin !== readiness.launch_origin) {
        throw new Error('量化页面来源与启动配置不一致。')
      }
      expectedOriginRef.current = target.origin
      attemptIdRef.current = `qpl_${crypto.randomUUID().replace(/-/g, '')}`
      ticketRef.current = {
        grant: ticket.access_token,
        eskProjection: ticket.esk_asset_projection,
        eskAllocationAuthorization: ticket.esk_quant_allocation_authorization,
        expiresAtUnix: Math.floor(Date.now() / 1000) + ticket.expires_in,
      }
      setFrameUrl(target.url)
      setStage('loading_page')
      setMessage('正在加载量化 Paper 页面，授权尚未发送。')
      timeoutRef.current = window.setTimeout(() => failLaunch('量化页面连接超时，请重试。'), 20_000)
    } catch (error) {
      failLaunch(errorMessage(error, '暂时无法创建量化 Paper 启动票据。'))
    }
  }

  const failLaunch = (detail: string) => {
    clearSensitiveState()
    setFrameUrl('')
    setStage('error')
    setMessage(detail)
  }

  const copy = stageCopy(stage)
  return (
    <section className={styles.launchCard} aria-labelledby="quant-paper-launch-title">
      <div className={styles.launchHeader}>
        <span className={styles.shield}><ShieldCheck size={22} aria-hidden="true" /></span>
        <div>
          <span className={styles.eyebrow}>PAPER / SIMULATED</span>
          <h3 id="quant-paper-launch-title">{integration.label || '进入 Paper 模拟持仓'}</h3>
          <p>{integration.description || '使用一龙账号短期授权查看本人模拟仓位。'}</p>
        </div>
        <div className={styles.riskPills} aria-label="量化 Paper 风险边界">
          <span>6% 预计 / 非保证</span><span>不移动真实资金</span><span>盈亏自负</span>
        </div>
      </div>

      <div className={styles.launchAction} data-tone={copy.tone}>
        <span className={styles.stateIcon}>{stageIcon(stage)}</span>
        <div className={styles.stateCopy}><strong>{copy.title}</strong><small>{message}</small></div>
        {stage === 'ready' && <button type="button" onClick={() => { void launch() }}>安全进入</button>}
        {(stage === 'unavailable' || stage === 'error') && (
          <button className={styles.retryButton} type="button" onClick={() => { void inspectReadiness() }}>
            <RefreshCw size={14} aria-hidden="true" />重新检查
          </button>
        )}
      </div>

      {stage === 'ready' && allocationRequests.length > 0 && (
        <label className={styles.requestPicker} htmlFor="esk-quant-launch-request">
          <span>本次要进入的 ESK Paper 申请</span>
          <select id="esk-quant-launch-request" value={selectedRequestId} onChange={(event) => setSelectedRequestId(event.target.value)}>
            <option value="">仅查看已有 Paper 记录</option>
            {allocationRequests.map((request) => (
              <option key={request.request_id} value={request.request_id}>
                {request.amount} ESK · {request.status === 'submitted' ? '等待接收' : '已接收，可恢复同步'} · …{request.request_id.slice(-8)}
              </option>
            ))}
          </select>
          <small>量化页面仍会要求你再次确认；这里只建立模拟绑定，不入金、不成交、不开始收益。</small>
        </label>
      )}

      {frameUrl && (
        <div className={styles.frameShell}>
          <div className={styles.frameBar}><span>一龙量化交易 · Paper</span><em>安全嵌入 / 短期内存授权</em></div>
          <iframe
            ref={frameRef}
            className={styles.frame}
            src={frameUrl}
            title="一龙量化交易 Paper 模拟持仓"
            sandbox="allow-scripts allow-same-origin"
            referrerPolicy="strict-origin"
            onLoad={() => {
              setStage((current) => current === 'loading_page' ? 'waiting' : current)
              setMessage((current) => current === '正在加载量化 Paper 页面，授权尚未发送。'
                ? '量化页面已加载，等待来源绑定握手。'
                : current)
            }}
          />
        </div>
      )}
    </section>
  )
}

function parseReadiness(value: unknown): Readiness | null {
  if (!isRecord(value) || !exactKeys(value, ['schema', 'protocol', 'enabled', 'simulated', 'reason'], ['launch_origin'])) return null
  if (value.schema !== READINESS_SCHEMA || value.protocol !== PROTOCOL || typeof value.enabled !== 'boolean' || value.simulated !== true) return null
  if (!['ready', 'configuration_required', 'configuration_invalid'].includes(String(value.reason))) return null
  if (value.enabled && (value.reason !== 'ready' || typeof value.launch_origin !== 'string' || !normalizeExactOrigin(value.launch_origin))) return null
  return value as Readiness
}

function parseTicket(value: unknown): LaunchTicket | null {
  if (!isRecord(value) || !exactKeys(value, ['schema', 'protocol', 'launch_url', 'access_token', 'expires_in', 'simulated'], ['esk_asset_projection', 'esk_quant_allocation_authorization'])) return null
  if (value.schema !== TICKET_SCHEMA || value.protocol !== PROTOCOL || value.simulated !== true || typeof value.launch_url !== 'string') return null
  if (typeof value.access_token !== 'string' || value.access_token.length > 8192 || !GRANT_PATTERN.test(value.access_token)) return null
  if (value.esk_asset_projection !== undefined
    && (typeof value.esk_asset_projection !== 'string'
      || value.esk_asset_projection.length > 8192
      || !ESK_PROJECTION_PATTERN.test(value.esk_asset_projection))) return null
  if (value.esk_quant_allocation_authorization !== undefined
    && (typeof value.esk_quant_allocation_authorization !== 'string'
      || value.esk_quant_allocation_authorization.length > 8192
      || !ESK_ALLOCATION_AUTHORIZATION_PATTERN.test(value.esk_quant_allocation_authorization))) return null
  if (!Number.isInteger(value.expires_in) || Number(value.expires_in) < 1 || Number(value.expires_in) > 300) return null
  return value as LaunchTicket
}

function isReadyMessage(value: unknown): value is { schema: string; protocol: string; channel_nonce: string; capabilities?: string[] } {
  if (!isRecord(value) || !exactKeys(value, ['schema', 'protocol', 'channel_nonce'], ['capabilities'])) return false
  const capabilities = value.capabilities
  if (capabilities !== undefined
    && (!Array.isArray(capabilities)
      || capabilities.length < 1
      || capabilities.length > READY_CAPABILITIES.length
      || new Set(capabilities).size !== capabilities.length
      || capabilities.some((capability) => !READY_CAPABILITIES.includes(capability as typeof READY_CAPABILITIES[number])))) return false
  return value.schema === READY_SCHEMA && value.protocol === PROTOCOL
    && typeof value.channel_nonce === 'string' && NONCE_PATTERN.test(value.channel_nonce)
}

function projectionCapability(token?: string): string | null {
  if (token?.startsWith('yep2.')) return ESK_PROJECTION_SCHEMA_V2
  if (token?.startsWith('yep1.')) return ESK_PROJECTION_SCHEMA_V1
  return null
}

function isTerminalMessage(value: unknown, nonce: string, attemptId: string): value is { schema: string } {
  if (!isRecord(value) || value.protocol !== PROTOCOL || value.channel_nonce !== nonce || value.attempt_id !== attemptId) return false
  if (value.schema === CONSUMED_SCHEMA) return exactKeys(value, ['schema', 'protocol', 'channel_nonce', 'attempt_id'])
  return value.schema === ERROR_SCHEMA && exactKeys(value, ['schema', 'protocol', 'channel_nonce', 'attempt_id', 'code'])
}

function isAllocationReceiptMessage(
  value: unknown,
  nonce: string,
  attemptId: string,
): value is { schema: string; receipt_token: string } {
  return isRecord(value)
    && exactKeys(value, ['schema', 'protocol', 'channel_nonce', 'attempt_id', 'receipt_token'])
    && value.schema === ALLOCATION_RECEIPT_MESSAGE_SCHEMA
    && value.protocol === PROTOCOL
    && value.channel_nonce === nonce
    && value.attempt_id === attemptId
    && typeof value.receipt_token === 'string'
    && value.receipt_token.length <= 8192
    && ESK_ALLOCATION_RECEIPT_PATTERN.test(value.receipt_token)
}

function normalizeLaunchUrl(raw: string): { url: string; origin: string } | null {
  try {
    const parsed = new URL(raw)
    if (parsed.username || parsed.password || parsed.search || parsed.hash) return null
    const loopback = ['localhost', '127.0.0.1', '[::1]'].includes(parsed.hostname)
    if (parsed.protocol !== 'https:' && !(parsed.protocol === 'http:' && loopback)) return null
    return { url: parsed.href, origin: parsed.origin }
  } catch { return null }
}

function normalizeExactOrigin(raw: string): string | null {
  const parsed = normalizeLaunchUrl(raw)
  return parsed && new URL(parsed.url).pathname === '/' ? parsed.origin : null
}

function exactKeys(value: Record<string, unknown>, required: string[], optional: string[] = []) {
  const keys = Object.keys(value)
  return required.every((key) => keys.includes(key)) && keys.every((key) => required.includes(key) || optional.includes(key))
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error && error.message ? error.message : fallback
}

function stageCopy(stage: Stage) {
  if (stage === 'ready') return { title: '可以安全进入', tone: 'ready' }
  if (stage === 'connected') return { title: '量化页面已连接', tone: 'success' }
  if (stage === 'unavailable' || stage === 'error') return { title: '当前不可用', tone: 'warning' }
  return { title: stage === 'checking' ? '正在检查' : '正在安全连接', tone: 'pending' }
}

function stageIcon(stage: Stage) {
  if (stage === 'connected' || stage === 'ready') return <CircleCheck size={19} aria-hidden="true" />
  if (stage === 'unavailable' || stage === 'error') return <AlertTriangle size={19} aria-hidden="true" />
  return <LoaderCircle className={styles.spin} size={19} aria-hidden="true" />
}
