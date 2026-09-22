import { useCallback, useEffect, useState } from 'react'
import { RefreshCw } from 'lucide-react'
import { listenLocalAiNativeSessionUpdates } from '../user-browser/localAiNativeSessionUpdates'
import {
  getExchangeWebObservation,
  runExchangeWebAdapterCommand,
  type ExchangeWebObservation,
} from './exchangeWebviewApi'
import { exchangeWebviewErrorMessage } from './exchangeWebviewErrors.js'
import styles from './WindowsExchangeWebviewLaunch.module.css'

/** Latest adapter-validated facts from the exchange login window: identity scope, grid list, wallet. */
export default function ExchangeObservationSummary({
  providerId,
  ownerKey,
}: {
  providerId: string
  ownerKey: string
}) {
  const [observation, setObservation] = useState<ExchangeWebObservation | null>(null)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      setObservation(await getExchangeWebObservation(providerId, ownerKey))
      setError('')
    } catch (caught) {
      setError(exchangeWebviewErrorMessage(caught))
    }
  }, [providerId, ownerKey])

  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | null = null
    void load()
    void listenLocalAiNativeSessionUpdates((update) => {
      if (update.providerId === providerId) void load()
    }).then((stop) => { if (disposed) stop(); else unlisten = stop })
    return () => { disposed = true; unlisten?.() }
  }, [providerId, load])

  async function refresh() {
    if (busy) return
    setBusy(true)
    try {
      await runExchangeWebAdapterCommand(providerId, ownerKey, 'refresh')
    } catch (caught) {
      setError(exchangeWebviewErrorMessage(caught))
    } finally {
      setBusy(false)
    }
  }

  if (!observation) return error ? <p className={styles.error} role="alert">{error}</p> : null
  const rows = Array.isArray(observation.list?.rows) ? (observation.list.rows as unknown[]).length : null
  const accountKind = typeof observation.identity?.account_kind === 'string' ? observation.identity.account_kind : null
  return (
    <div className={styles.observation} aria-label="交易所只读观察">
      <span>
        {!observation.windowOpen ? '官网窗口未打开。'
          : !observation.adapterReady ? '读取适配器等待页面就绪…'
          : rows === null ? (observation.unavailableAtMs ? '官网私有接口返回异常，等待页面重新读取。' : '已就绪，等待官网加载网格列表。')
          : `${accountKind === 'sub' ? '子账户' : accountKind === 'primary' ? '主账户' : '账户'} · ${rows} 个运行中网格 · ${Object.keys(observation.details).length} 份详情已读`}
      </span>
      <button type="button" disabled={busy || !observation.windowOpen || !observation.adapterReady} onClick={() => void refresh()} aria-label="刷新只读观察">
        <RefreshCw size={13} aria-hidden="true" />刷新
      </button>
      {error && <p className={styles.error} role="alert">{error}</p>}
      {observation.lastError && <p className={styles.error}>{observation.lastError}</p>}
    </div>
  )
}
