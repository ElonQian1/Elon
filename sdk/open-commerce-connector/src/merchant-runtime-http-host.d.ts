import type { Server as HttpServer } from 'node:http'
import type { Server as HttpsServer, ServerOptions as HttpsServerOptions } from 'node:https'

export const MERCHANT_RUNTIME_HTTP_HOST_SCHEMA: 'merchant_runtime.http_host.v1'
export const MERCHANT_RUNTIME_HTTP_HEALTH_SCHEMA: 'merchant_runtime.http_health.v1'
export const MERCHANT_RUNTIME_HTTP_ERROR_SCHEMA: 'merchant_runtime.http_error.v1'
export const MERCHANT_RUNTIME_HTTP_INVOKE_PATH: '/commerce/v1/invoke'
export const MERCHANT_RUNTIME_HTTP_HEALTH_PATH: '/healthz'

export interface MerchantRuntimeHttpRuntime {
  handleInvoke(request: {
    headers: Record<string, string | string[] | undefined>
    body: Buffer
  }): Promise<{ status: number; body: Record<string, unknown> }>
}

export interface MerchantRuntimeHttpHostOptions {
  runtime: MerchantRuntimeHttpRuntime
  protocol?: 'http' | 'https'
  tls?: HttpsServerOptions
  invokePath?: string
  healthPath?: string
  maxBodyBytes?: number
  maxResponseBytes?: number
  headersTimeoutMs?: number
  requestTimeoutMs?: number
  keepAliveTimeoutMs?: number
  shutdownGraceMs?: number
  maxConnections?: number
  maxRequestsPerSocket?: number
}

export interface MerchantRuntimeHttpListenOptions {
  host?: string
  port?: number
  backlog?: number
}

export interface MerchantRuntimeHttpListenReceipt {
  schema: 'merchant_runtime.http_host.v1'
  protocol: 'http' | 'https'
  address: string
  family: string
  port: number
  origin: string
}

export interface MerchantRuntimeHttpCloseReceipt {
  schema: 'merchant_runtime.http_host.v1'
  status: 'closed'
  grace_ms: number
  elapsed_ms: number
  forced: boolean
  forced_connections: number
  in_flight_at_start: number
  remaining_in_flight: number
}

export interface MerchantRuntimeHttpHostState {
  schema: 'merchant_runtime.http_host.v1'
  protocol: 'http' | 'https'
  status: 'idle' | 'starting' | 'ready' | 'draining' | 'closed'
  listening: boolean
  in_flight: number
  open_connections: number
}

export interface MerchantRuntimeHttpHost {
  readonly server: HttpServer | HttpsServer
  listen(options?: MerchantRuntimeHttpListenOptions): Promise<MerchantRuntimeHttpListenReceipt>
  close(options?: { graceMs?: number }): Promise<MerchantRuntimeHttpCloseReceipt>
  state(): MerchantRuntimeHttpHostState
}

export function createMerchantRuntimeHttpHost(
  options: MerchantRuntimeHttpHostOptions,
): MerchantRuntimeHttpHost
