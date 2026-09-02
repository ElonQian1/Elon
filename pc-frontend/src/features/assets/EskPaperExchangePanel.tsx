import { useCallback, useEffect, useState } from 'react'
import { ArrowRightLeft, Clock3, ShieldCheck } from 'lucide-react'

import {
  eskAssetApi,
  type EskExchangeAccount,
  type EskExchangeDirection,
  type EskExchangeExecution,
  type EskExchangeQuote,
} from './eskAssetApi'
import styles from './EskPaperExchangePanel.module.css'

interface Props {
  previewMode?: boolean
  onChanged: () => void | Promise<void>
}

const PREVIEW_ACCOUNT: EskExchangeAccount = {
  schema: 'yilong.esk.paper_exchange_account.v1',
  mode: 'paper', enabled: true, simulated: true, funds_moved: false,
  on_chain_settlement: false, trading_mode: 'paper',
  balances: {
    esk: { total: '1280.000000', available: '780.000000', revision: 4, updated_at: null },
    usdt: { total: '600.000000', available: '600.000000', revision: 1, updated_at: null },
  },
  pricing: {
    usdt_per_esk: '1.000000', fee_bps: 30, fee_percent: '0.30%',
    config_revision: 'preview', quote_ttl_seconds: 60,
  },
  status_message: 'Paper 模拟兑换已启用；不会移动真实 USDT 或 ESK。',
}

export default function EskPaperExchangePanel({ previewMode = false, onChanged }: Props) {
  const [account, setAccount] = useState<EskExchangeAccount | null>(previewMode ? PREVIEW_ACCOUNT : null)
  const [history, setHistory] = useState<EskExchangeExecution[]>([])
  const [direction, setDirection] = useState<EskExchangeDirection>('usdt_to_esk')
  const [inputAmount, setInputAmount] = useState('')
  const [quote, setQuote] = useState<EskExchangeQuote | null>(null)
  const [working, setWorking] = useState<'load' | 'quote' | 'execute' | null>(null)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    if (previewMode) return
    setWorking('load')
    setError('')
    try {
      const [nextAccount, nextHistory] = await Promise.all([
        eskAssetApi.exchangeAccount(),
        eskAssetApi.exchangeHistory(),
      ])
      assertSafeAccount(nextAccount)
      nextHistory.executions.forEach(assertSafeExecution)
      setAccount(nextAccount)
      setHistory(nextHistory.executions)
    } catch (reason) {
      setError(errorMessage(reason, 'Paper 兑换账户暂不可用'))
    } finally {
      setWorking(null)
    }
  }, [previewMode])

  useEffect(() => { void load() }, [load])

  function changeDirection(value: EskExchangeDirection) {
    setDirection(value)
    setInputAmount('')
    setQuote(null)
    setMessage('')
    setError('')
  }

  async function requestQuote(event: React.FormEvent) {
    event.preventDefault()
    const amount = inputAmount.trim()
    if (!/^\d+(\.\d{1,6})?$/.test(amount) || /^0+(\.0+)?$/.test(amount)) {
      setError('请输入大于 0、最多六位小数的兑换数量')
      return
    }
    setWorking('quote')
    setQuote(null)
    setMessage('')
    setError('')
    try {
      if (previewMode) {
        setMessage('预览模式不会创建真实报价或流水。')
        return
      }
      const nextQuote = await eskAssetApi.createExchangeQuote(direction, amount)
      assertSafeQuote(nextQuote)
      setQuote(nextQuote)
    } catch (reason) {
      setError(errorMessage(reason, '获取 Paper 报价失败'))
    } finally {
      setWorking(null)
    }
  }

  async function execute() {
    if (!quote) return
    setWorking('execute')
    setMessage('')
    setError('')
    try {
      if (previewMode) {
        setMessage('预览模式不会提交兑换。')
        return
      }
      const execution = await eskAssetApi.executeExchange(quote.quote_id, newIdempotencyKey())
      assertSafeExecution(execution)
      setMessage(`Paper 兑换完成：${execution.quote.net_output_amount} ${execution.quote.output_asset} 已记入模拟账本。`)
      setQuote(null)
      setInputAmount('')
      await Promise.all([load(), onChanged()])
    } catch (reason) {
      setError(errorMessage(reason, 'Paper 兑换失败；余额没有被乐观修改'))
    } finally {
      setWorking(null)
    }
  }

  const source = direction === 'usdt_to_esk' ? 'USDT' : 'ESK'
  const target = direction === 'usdt_to_esk' ? 'ESK' : 'USDT'
  const available = direction === 'usdt_to_esk'
    ? account?.balances.usdt.available
    : account?.balances.esk.available
  const enabled = Boolean(account?.enabled && account.pricing)

  return (
    <section className={styles.panel} aria-label="ESK 与 USDT Paper 兑换">
      <header>
        <div><ArrowRightLeft size={18} /><strong>USDT / ESK 兑换</strong></div>
        <span>Paper 模拟</span>
      </header>

      <div className={styles.balances}>
        <div><span>Paper USDT</span><strong>{account?.balances.usdt.available ?? '—'} USDT</strong></div>
        <div><span>可用 ESK</span><strong>{account?.balances.esk.available ?? '—'} ESK</strong></div>
      </div>

      <div className={styles.policy}>
        <ShieldCheck size={15} />
        <span>未上链 · 不移动真实资金 · 当前费率 {account?.pricing?.fee_percent ?? '未配置'} · 报价 60 秒有效</span>
      </div>

      <div className={styles.directions} role="group" aria-label="兑换方向">
        <button type="button" className={direction === 'usdt_to_esk' ? styles.active : ''} onClick={() => changeDirection('usdt_to_esk')}>USDT → ESK</button>
        <button type="button" className={direction === 'esk_to_usdt' ? styles.active : ''} onClick={() => changeDirection('esk_to_usdt')}>ESK → USDT</button>
      </div>

      <form className={styles.quoteForm} onSubmit={requestQuote}>
        <label htmlFor="esk-exchange-input">支付 {source}<span>可用 {available ?? '—'} {source}</span></label>
        <div>
          <input id="esk-exchange-input" value={inputAmount} onChange={(event) => { setInputAmount(event.target.value); setQuote(null) }} placeholder="0.000000" inputMode="decimal" autoComplete="off" disabled={!enabled || working !== null} />
          <button type="submit" disabled={!enabled || working !== null || !inputAmount.trim()}>{working === 'quote' ? '报价中…' : `预览 ${target} 报价`}</button>
        </div>
      </form>

      {quote && (
        <div className={styles.quote} role="status">
          <div><span>支付</span><strong>{quote.input_amount} {quote.input_asset}</strong></div>
          <div><span>兑换毛额</span><strong>{quote.gross_output_amount} {quote.output_asset}</strong></div>
          <div><span>平台手续费</span><strong>- {quote.fee_amount} {quote.fee_asset}</strong></div>
          <div className={styles.net}><span>预计到账</span><strong>{quote.net_output_amount} {quote.output_asset}</strong></div>
          <p><Clock3 size={13} />报价有效至 {formatDateTime(quote.expires_at)}，过期会拒绝成交。</p>
          <button type="button" onClick={() => void execute()} disabled={working !== null}>{working === 'execute' ? '确认中…' : '确认 Paper 模拟兑换'}</button>
        </div>
      )}

      {!enabled && <p className={styles.disabled}>{account?.status_message || (working === 'load' ? '正在读取兑换账户…' : 'Paper 兑换尚未启用。')}</p>}
      {message && <p className={styles.success}>{message}</p>}
      {error && <p className={styles.error} role="alert">{error}</p>}

      {history.length > 0 && (
        <div className={styles.history}>
          <strong>最近 Paper 兑换</strong>
          {history.slice(0, 3).map((execution) => (
            <p key={execution.execution_id}>{execution.quote.input_amount} {execution.quote.input_asset} → {execution.quote.net_output_amount} {execution.quote.output_asset}<span>{formatDateTime(execution.executed_at)}</span></p>
          ))}
        </div>
      )}
    </section>
  )
}

function assertSafeAccount(value: EskExchangeAccount) {
  if (value.schema !== 'yilong.esk.paper_exchange_account.v1' || !value.simulated || value.funds_moved || value.on_chain_settlement || value.trading_mode !== 'paper') throw new Error('Paper 兑换账户安全标识不匹配')
}

function assertSafeQuote(value: EskExchangeQuote) {
  if (value.schema !== 'yilong.esk.paper_exchange_quote.v1' || !value.simulated || value.funds_moved || value.on_chain_settlement || value.trading_mode !== 'paper') throw new Error('Paper 兑换报价安全标识不匹配')
}

function assertSafeExecution(value: EskExchangeExecution) {
  if (value.schema !== 'yilong.esk.paper_exchange_execution.v1' || !value.simulated || value.funds_moved || value.on_chain_settlement || value.trading_mode !== 'paper') throw new Error('Paper 兑换回执安全标识不匹配')
  assertSafeQuote(value.quote)
}

function newIdempotencyKey() {
  return `esk-paper-exchange-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`}`
}

function errorMessage(reason: unknown, fallback: string) {
  return (reason as { message?: string } | null)?.message || fallback
}

function formatDateTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { dateStyle: 'short', timeStyle: 'short' })
}
