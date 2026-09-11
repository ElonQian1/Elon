const UPGRADE_REQUIRED = /command.+not found|unknown command|not allowed|allowlist|permission denied/i
const UPGRADE_MESSAGE = '当前 Win 客户端版本较旧，还不支持交易所 WebView。请更新并完全退出旧客户端后重新打开。'
const FALLBACK_MESSAGE = '交易所官网窗口打开失败，请重试。'

export function exchangeWebviewErrorMessage(error) {
  const raw = error instanceof Error
    ? error.message
    : typeof error === 'string'
      ? error
      : ''
  if (UPGRADE_REQUIRED.test(raw)) return UPGRADE_MESSAGE
  return raw.trim().slice(0, 240) || FALLBACK_MESSAGE
}

export function normalizeExchangeWebviewError(error) {
  return new Error(exchangeWebviewErrorMessage(error))
}
