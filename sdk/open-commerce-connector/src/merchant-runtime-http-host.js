import { createServer as createHttpServer } from 'node:http'
import { createServer as createHttpsServer } from 'node:https'
import { MERCHANT_RUNTIME_MAX_BODY_BYTES } from './merchant-runtime.js'
import {
  closeMerchantRuntimeHttpServer,
  finishMerchantRuntimeHttpRequest,
} from './merchant-runtime-http-lifecycle.js'
export const MERCHANT_RUNTIME_HTTP_HOST_SCHEMA = 'merchant_runtime.http_host.v1'
export const MERCHANT_RUNTIME_HTTP_HEALTH_SCHEMA = 'merchant_runtime.http_health.v1'
export const MERCHANT_RUNTIME_HTTP_ERROR_SCHEMA = 'merchant_runtime.http_error.v1'
export const MERCHANT_RUNTIME_HTTP_INVOKE_PATH = '/commerce/v1/invoke'
export const MERCHANT_RUNTIME_HTTP_HEALTH_PATH = '/healthz'
const DEFAULT_RESPONSE_BYTES = MERCHANT_RUNTIME_MAX_BODY_BYTES + (64 * 1024)
const DEFAULT_HEADERS_TIMEOUT_MS = 5_000
const DEFAULT_REQUEST_TIMEOUT_MS = 15_000
const DEFAULT_KEEP_ALIVE_TIMEOUT_MS = 5_000
const DEFAULT_SHUTDOWN_GRACE_MS = 10_000
class HostRequestError extends Error {
  constructor(status, errorCode, message) {
    super(message)
    this.status = status
    this.errorCode = errorCode
  }
}
export function createMerchantRuntimeHttpHost(options) {
  const config = normalizeOptions(options)
  const lifecycle = {
    phase: 'idle',
    inFlight: 0,
    listenPromise: null,
    closePromise: null,
    closeReceipt: null,
    drainWaiters: new Set(),
  }
  const sockets = new Set()

  const requestListener = (request, response) => {
    lifecycle.inFlight += 1
    handleRequest(request, response, config, lifecycle)
      .catch((error) => {
        if (!response.headersSent) {
          if (error instanceof HostRequestError) {
            sendHostError(response, error.status, error.errorCode, error.message, {
              closeConnection: error.status === 413,
            })
          } else {
            sendHostError(response, 500, 'internal_error', 'merchant runtime host failed')
          }
        } else {
          response.destroy()
        }
      })
      .finally(() => {
        finishMerchantRuntimeHttpRequest(lifecycle)
      })
  }

  const server = config.protocol === 'https'
    ? createHttpsServer(config.tls, requestListener)
    : createHttpServer(requestListener)
  configureServer(server, config)
  server.on('connection', (socket) => {
    sockets.add(socket)
    socket.once('close', () => sockets.delete(socket))
  })
  server.on('checkContinue', (_request, response) => {
    sendHostError(response, 417, 'expectation_failed', 'request expectations are not supported', {
      closeConnection: true,
    })
  })
  server.on('clientError', (_error, socket) => {
    if (!socket.writable) return
    socket.end('HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n')
  })

  async function listen(listenOptions = {}) {
    if (['starting', 'ready'].includes(lifecycle.phase) && lifecycle.listenPromise) {
      return lifecycle.listenPromise
    }
    if (lifecycle.phase !== 'idle') {
      throw new Error(`merchant runtime host cannot listen from ${lifecycle.phase}`)
    }
    const target = normalizeListenOptions(listenOptions)
    lifecycle.phase = 'starting'
    lifecycle.listenPromise = new Promise((resolve, reject) => {
      const onError = (error) => {
        server.off('listening', onListening)
        lifecycle.phase = 'idle'
        lifecycle.listenPromise = null
        reject(error)
      }
      const onListening = () => {
        server.off('error', onError)
        lifecycle.phase = 'ready'
        resolve(listeningReceipt(server, config.protocol))
      }
      server.once('error', onError)
      server.once('listening', onListening)
      server.listen(target)
    })
    return lifecycle.listenPromise
  }

  async function close(closeOptions = {}) {
    if (lifecycle.closePromise) return lifecycle.closePromise
    const graceMs = boundedInteger(
      closeOptions.graceMs ?? config.shutdownGraceMs,
      0,
      120_000,
      'closeOptions.graceMs',
    )
    lifecycle.closePromise = closeMerchantRuntimeHttpServer(server, sockets, lifecycle, graceMs)
    lifecycle.closeReceipt = await lifecycle.closePromise
    return lifecycle.closeReceipt
  }

  return Object.freeze({
    server,
    listen,
    close,
    state: () => stateSnapshot(server, config.protocol, lifecycle, sockets.size),
  })
}

async function handleRequest(request, response, config, lifecycle) {
  const path = request.url ?? ''
  if (path === config.healthPath) {
    request.resume()
    if (request.method !== 'GET') {
      sendHostError(response, 405, 'method_not_allowed', 'health endpoint requires GET', {
        headers: { allow: 'GET' },
      })
      return
    }
    const ready = lifecycle.phase === 'ready'
    sendJson(response, ready ? 200 : 503, {
      schema: MERCHANT_RUNTIME_HTTP_HEALTH_SCHEMA,
      status: ready ? 'ready' : 'draining',
    }, { closeConnection: !ready })
    return
  }

  if (path !== config.invokePath) {
    request.resume()
    sendHostError(response, 404, 'route_not_found', 'merchant runtime route was not found')
    return
  }
  if (request.method !== 'POST') {
    request.resume()
    sendHostError(response, 405, 'method_not_allowed', 'merchant runtime invocation requires POST', {
      headers: { allow: 'POST' },
    })
    return
  }
  if (lifecycle.phase !== 'ready') {
    request.resume()
    sendHostError(response, 503, 'host_draining', 'merchant runtime host is draining', {
      closeConnection: true,
    })
    return
  }

  validateRequestHeaders(request.headers, config.maxBodyBytes)
  const body = await readRequestBody(request, config.maxBodyBytes)
  let runtimeResponse
  try {
    runtimeResponse = await config.runtime.handleInvoke({
      headers: request.headers,
      body,
    })
  } catch {
    sendHostError(response, 500, 'runtime_failed', 'merchant runtime handler failed')
    return
  }

  let normalized
  try {
    normalized = normalizeRuntimeResponse(runtimeResponse, config.maxResponseBytes)
  } catch {
    sendHostError(response, 500, 'invalid_runtime_response', 'merchant runtime returned an invalid response')
    return
  }
  sendEncodedJson(response, normalized.status, normalized.bytes)
}

function validateRequestHeaders(headers, maxBodyBytes) {
  const mediaType = firstHeader(headers, 'content-type')
    ?.split(';', 1)[0]
    .trim()
    .toLowerCase()
  if (mediaType !== 'application/json') {
    throw new HostRequestError(415, 'unsupported_media_type', 'content type must be application/json')
  }
  const encoding = firstHeader(headers, 'content-encoding')?.trim().toLowerCase()
  if (encoding && encoding !== 'identity') {
    throw new HostRequestError(415, 'unsupported_content_encoding', 'encoded request bodies are not supported')
  }
  const length = firstHeader(headers, 'content-length')
  if (length === undefined) return
  if (!/^\d+$/.test(length)) {
    throw new HostRequestError(400, 'invalid_content_length', 'content length is invalid')
  }
  const value = Number(length)
  if (!Number.isSafeInteger(value)) {
    throw new HostRequestError(400, 'invalid_content_length', 'content length is invalid')
  }
  if (value > maxBodyBytes) {
    throw new HostRequestError(413, 'request_too_large', 'merchant runtime request is too large')
  }
}

function readRequestBody(request, maxBodyBytes) {
  return new Promise((resolve, reject) => {
    const chunks = []
    let total = 0
    let settled = false

    const cleanup = () => {
      request.off('data', onData)
      request.off('end', onEnd)
      request.off('aborted', onAborted)
      request.off('error', onError)
    }
    const finish = (operation) => {
      if (settled) return
      settled = true
      cleanup()
      operation()
    }
    const onData = (chunk) => {
      const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk)
      total += bytes.length
      if (total > maxBodyBytes) {
        finish(() => {
          request.on('error', () => {})
          request.resume()
          reject(new HostRequestError(413, 'request_too_large', 'merchant runtime request is too large'))
        })
        return
      }
      chunks.push(bytes)
    }
    const onEnd = () => finish(() => {
      if (!request.complete) {
        reject(new HostRequestError(400, 'request_aborted', 'merchant runtime request was incomplete'))
        return
      }
      resolve(Buffer.concat(chunks, total))
    })
    const onAborted = () => finish(() => reject(
      new HostRequestError(400, 'request_aborted', 'merchant runtime request was aborted'),
    ))
    const onError = () => finish(() => reject(
      new HostRequestError(400, 'request_failed', 'merchant runtime request could not be read'),
    ))

    request.on('data', onData)
    request.once('end', onEnd)
    request.once('aborted', onAborted)
    request.once('error', onError)
  })
}

function normalizeRuntimeResponse(value, maxResponseBytes) {
  if (!value || Array.isArray(value) || typeof value !== 'object') throw new TypeError()
  if (!Number.isInteger(value.status)
    || (value.status !== 200 && (value.status < 400 || value.status > 599))) {
    throw new TypeError()
  }
  if (!value.body || Array.isArray(value.body) || typeof value.body !== 'object') {
    throw new TypeError()
  }
  const bytes = Buffer.from(JSON.stringify(value.body), 'utf8')
  if (bytes.length > maxResponseBytes) throw new TypeError()
  return { status: value.status, bytes }
}

function sendHostError(response, status, errorCode, message, options = {}) {
  sendJson(response, status, {
    schema: MERCHANT_RUNTIME_HTTP_ERROR_SCHEMA,
    error_code: errorCode,
    message,
  }, options)
}

function sendJson(response, status, body, options = {}) {
  sendEncodedJson(response, status, Buffer.from(JSON.stringify(body), 'utf8'), options)
}

function sendEncodedJson(response, status, bytes, options = {}) {
  if (response.destroyed || response.writableEnded) return
  const headers = {
    'cache-control': 'no-store',
    'content-length': String(bytes.length),
    'content-type': 'application/json; charset=utf-8',
    'x-content-type-options': 'nosniff',
    ...(options.headers ?? {}),
  }
  if (options.closeConnection) {
    headers.connection = 'close'
    response.shouldKeepAlive = false
  }
  response.writeHead(status, headers)
  response.end(bytes)
}

function normalizeOptions(options) {
  expectObject(options, 'options')
  if (typeof options.runtime?.handleInvoke !== 'function') {
    throw new TypeError('options.runtime.handleInvoke is required')
  }
  const protocol = options.protocol ?? 'http'
  if (!['http', 'https'].includes(protocol)) {
    throw new TypeError('options.protocol must be http or https')
  }
  if (protocol === 'https') expectObject(options.tls, 'options.tls')
  if (protocol === 'http' && options.tls !== undefined) {
    throw new TypeError('options.tls is only valid for https')
  }
  const requestTimeoutMs = boundedInteger(
    options.requestTimeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS,
    1_000,
    120_000,
    'options.requestTimeoutMs',
  )
  const headersTimeoutMs = boundedInteger(
    options.headersTimeoutMs ?? DEFAULT_HEADERS_TIMEOUT_MS,
    1_000,
    requestTimeoutMs,
    'options.headersTimeoutMs',
  )
  return Object.freeze({
    runtime: options.runtime,
    protocol,
    tls: options.tls,
    invokePath: exactPath(options.invokePath ?? MERCHANT_RUNTIME_HTTP_INVOKE_PATH, 'options.invokePath'),
    healthPath: exactPath(options.healthPath ?? MERCHANT_RUNTIME_HTTP_HEALTH_PATH, 'options.healthPath'),
    maxBodyBytes: boundedInteger(
      options.maxBodyBytes ?? MERCHANT_RUNTIME_MAX_BODY_BYTES,
      1_024,
      MERCHANT_RUNTIME_MAX_BODY_BYTES,
      'options.maxBodyBytes',
    ),
    maxResponseBytes: boundedInteger(
      options.maxResponseBytes ?? DEFAULT_RESPONSE_BYTES,
      1_024,
      2 * 1024 * 1024,
      'options.maxResponseBytes',
    ),
    headersTimeoutMs,
    requestTimeoutMs,
    keepAliveTimeoutMs: boundedInteger(
      options.keepAliveTimeoutMs ?? DEFAULT_KEEP_ALIVE_TIMEOUT_MS,
      500,
      30_000,
      'options.keepAliveTimeoutMs',
    ),
    shutdownGraceMs: boundedInteger(
      options.shutdownGraceMs ?? DEFAULT_SHUTDOWN_GRACE_MS,
      0,
      120_000,
      'options.shutdownGraceMs',
    ),
    maxConnections: boundedInteger(options.maxConnections ?? 128, 1, 10_000, 'options.maxConnections'),
    maxRequestsPerSocket: boundedInteger(
      options.maxRequestsPerSocket ?? 100,
      1,
      10_000,
      'options.maxRequestsPerSocket',
    ),
  })
}

function configureServer(server, config) {
  server.headersTimeout = config.headersTimeoutMs
  server.requestTimeout = config.requestTimeoutMs
  server.keepAliveTimeout = config.keepAliveTimeoutMs
  server.maxConnections = config.maxConnections
  server.maxHeadersCount = 64
  server.maxRequestsPerSocket = config.maxRequestsPerSocket
}

function normalizeListenOptions(value) {
  expectObject(value, 'listenOptions')
  const host = value.host ?? '127.0.0.1'
  if (typeof host !== 'string' || !host.trim() || host.length > 253) {
    throw new TypeError('listenOptions.host must be a non-empty host')
  }
  const port = boundedInteger(value.port ?? 0, 0, 65_535, 'listenOptions.port')
  const normalized = { host: host.trim(), port, exclusive: true }
  if (value.backlog !== undefined) {
    normalized.backlog = boundedInteger(value.backlog, 1, 65_535, 'listenOptions.backlog')
  }
  return normalized
}

function listeningReceipt(server, protocol) {
  const address = server.address()
  if (!address || typeof address === 'string') throw new Error('merchant runtime host has no TCP address')
  const displayAddress = address.address.includes(':') ? `[${address.address}]` : address.address
  return Object.freeze({
    schema: MERCHANT_RUNTIME_HTTP_HOST_SCHEMA,
    protocol,
    address: address.address,
    family: address.family,
    port: address.port,
    origin: `${protocol}://${displayAddress}:${address.port}`,
  })
}

function stateSnapshot(server, protocol, lifecycle, openConnections) {
  return Object.freeze({
    schema: MERCHANT_RUNTIME_HTTP_HOST_SCHEMA,
    protocol,
    status: lifecycle.phase,
    listening: server.listening,
    in_flight: lifecycle.inFlight,
    open_connections: openConnections,
  })
}

function firstHeader(headers, name) {
  const value = headers?.[name]
  return Array.isArray(value) ? value[0] : value
}

function exactPath(value, name) {
  if (typeof value !== 'string' || !/^\/[A-Za-z0-9._~!$&'()*+,;=:@%/-]*$/.test(value)) {
    throw new TypeError(`${name} must be one exact absolute path without query or fragment`)
  }
  return value
}

function boundedInteger(value, minimum, maximum, name) {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new TypeError(`${name} must be an integer between ${minimum} and ${maximum}`)
  }
  return value
}

function expectObject(value, name) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    throw new TypeError(`${name} must be an object`)
  }
}
