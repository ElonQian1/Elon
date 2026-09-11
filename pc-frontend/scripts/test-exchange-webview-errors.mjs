import assert from 'node:assert/strict'
import {
  exchangeWebviewErrorMessage,
  normalizeExchangeWebviewError,
} from '../src/features/exchange-webview/exchangeWebviewErrors.js'

const missingCommand = 'Command list_exchange_web_providers not found'
assert.match(exchangeWebviewErrorMessage(missingCommand), /客户端版本较旧/)
assert.match(normalizeExchangeWebviewError(missingCommand).message, /客户端版本较旧/)

const webviewFailure = '无法创建 Binance 合约网格 WebView2 窗口'
assert.equal(exchangeWebviewErrorMessage(new Error(webviewFailure)), webviewFailure)
assert.equal(normalizeExchangeWebviewError(webviewFailure).message, webviewFailure)

assert.equal(exchangeWebviewErrorMessage({ reason: 'hidden' }), '交易所官网窗口打开失败，请重试。')

console.log('Windows exchange WebView error contracts passed')
