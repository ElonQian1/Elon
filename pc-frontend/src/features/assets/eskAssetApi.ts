import { api } from '../../api/client'

export interface EskAssetIdentity {
  asset_id: 'esk'
  symbol: 'ESK'
  name: string
  decimals: 6
  issuance_mode: 'paper_recorded'
  chain_status: 'not_deployed'
  contract_address: null
}

export interface EskAssetBalance {
  total: string
  available: string
  reserved_for_sellback: string
  reserved_for_quant: string
  reserved_total: string
  total_base_units: string
  available_base_units: string
  sellback_reserved_base_units: string
  quant_reserved_base_units: string
  reserved_base_units: string
  revision: number
  updated_at: string | null
}

export interface EskAssetSnapshot {
  schema: 'yilong.esk.asset_account.v2'
  mode: 'disabled' | 'paper' | 'invalid'
  enabled: boolean
  simulated: true
  funds_moved: false
  asset: EskAssetIdentity
  balance: EskAssetBalance
  sellback: {
    application_only: true
    request_enabled: boolean
    settlement_enabled: false
    pricing_status: 'not_defined'
  }
  status_message: string
}

export interface EskQuantAllocationRequest {
  request_id: string
  amount: string
  amount_base_units: string
  risk_disclosure_revision: 'esk-quant-paper-allocation-v2'
  status: 'submitted' | 'canceled' | 'accepted' | 'released'
  revision: number
  submitted_at: string
  updated_at: string
  simulated: true
  funds_moved: false
  position_created: false
  allocation_binding_created: boolean
  binding_id?: string
  quant_binding_revision?: number
  occurred_at_unix?: number
  replayed: boolean
}

interface EskQuantAllocationList {
  schema: 'yilong.esk.quant_allocation_request_list.v2'
  simulated: true
  funds_moved: false
  position_created: false
  requests: EskQuantAllocationRequest[]
}

export interface EskSellbackRequest {
  request_id: string
  amount: string
  amount_base_units: string
  status: 'submitted' | 'canceled'
  revision: number
  submitted_at: string
  updated_at: string
  simulated: true
  funds_moved: false
  replayed: boolean
}

interface EskSellbackList {
  schema: 'yilong.esk.sellback_request_list.v1'
  simulated: true
  funds_moved: false
  requests: EskSellbackRequest[]
}

export type EskExchangeDirection = 'usdt_to_esk' | 'esk_to_usdt'

export interface EskExchangeAccount {
  schema: 'yilong.esk.paper_exchange_account.v1'
  mode: 'disabled' | 'paper' | 'invalid'
  enabled: boolean
  simulated: true
  funds_moved: false
  on_chain_settlement: false
  trading_mode: 'paper'
  balances: {
    esk: { total: string; available: string; revision: number; updated_at: string | null }
    usdt: { total: string; available: string; revision: number; updated_at: string | null }
  }
  pricing: null | {
    usdt_per_esk: string
    fee_bps: number
    fee_percent: string
    config_revision: string
    quote_ttl_seconds: 60
  }
  status_message: string
}

export interface EskExchangeQuote {
  schema: 'yilong.esk.paper_exchange_quote.v1'
  quote_id: string
  direction: EskExchangeDirection
  input_asset: 'ESK' | 'USDT'
  output_asset: 'ESK' | 'USDT'
  input_amount: string
  gross_output_amount: string
  fee_asset: 'ESK' | 'USDT'
  fee_amount: string
  net_output_amount: string
  usdt_per_esk: string
  fee_bps: number
  created_at: string
  expires_at: string
  simulated: true
  funds_moved: false
  on_chain_settlement: false
  trading_mode: 'paper'
}

export interface EskExchangeExecution {
  schema: 'yilong.esk.paper_exchange_execution.v1'
  execution_id: string
  executed_at: string
  replayed: boolean
  quote: EskExchangeQuote
  simulated: true
  funds_moved: false
  on_chain_settlement: false
  trading_mode: 'paper'
}

interface EskExchangeExecutionList {
  schema: 'yilong.esk.paper_exchange_execution_list.v1'
  simulated: true
  funds_moved: false
  on_chain_settlement: false
  trading_mode: 'paper'
  executions: EskExchangeExecution[]
}

export const eskAssetApi = {
  account: () => api.get<EskAssetSnapshot>('/api/me/assets/esk'),
  sellbackRequests: () => api.get<EskSellbackList>('/api/me/assets/esk/sellback-requests?limit=20'),
  quantAllocationRequests: () => api.get<EskQuantAllocationList>('/api/me/assets/esk/quant-allocation-requests?limit=20'),
  exchangeAccount: () => api.get<EskExchangeAccount>('/api/me/assets/esk/exchange-account'),
  exchangeHistory: () => api.get<EskExchangeExecutionList>('/api/me/assets/esk/exchanges?limit=20'),
  createExchangeQuote: (direction: EskExchangeDirection, inputAmount: string) =>
    api.post<EskExchangeQuote>('/api/me/assets/esk/exchange-quotes', {
      direction,
      input_amount: inputAmount,
    }),
  executeExchange: (quoteId: string, idempotencyKey: string) =>
    api.post<EskExchangeExecution>('/api/me/assets/esk/exchanges', {
      quote_id: quoteId,
      idempotency_key: idempotencyKey,
      confirmation: 'CONFIRM PAPER ESK USDT EXCHANGE',
    }),
  createSellback: (amount: string, idempotencyKey: string) =>
    api.post<EskSellbackRequest>('/api/me/assets/esk/sellback-requests', {
      amount,
      idempotency_key: idempotencyKey,
    }),
  cancelSellback: (requestId: string) =>
    api.post<EskSellbackRequest>(`/api/me/assets/esk/sellback-requests/${encodeURIComponent(requestId)}/cancel`, {
      confirmation: 'CANCEL ESK SELLBACK REQUEST',
    }),
  createQuantAllocation: (amount: string, idempotencyKey: string) =>
    api.post<EskQuantAllocationRequest>('/api/me/assets/esk/quant-allocation-requests', {
      amount,
      idempotency_key: idempotencyKey,
      risk_disclosure_revision: 'esk-quant-paper-allocation-v2',
      confirmation: 'REQUEST PAPER ESK QUANT ALLOCATION',
    }),
  cancelQuantAllocation: (requestId: string) =>
    api.post<EskQuantAllocationRequest>(`/api/me/assets/esk/quant-allocation-requests/${encodeURIComponent(requestId)}/cancel`, {
      confirmation: 'CANCEL PAPER ESK QUANT ALLOCATION',
    }),
  applyQuantAllocationReceipt: (receiptToken: string) =>
    api.post<EskQuantAllocationRequest>('/api/me/assets/esk/quant-allocation-receipts', {
      receipt_token: receiptToken,
    }),
}

export const ESK_PREVIEW_SNAPSHOT: EskAssetSnapshot = {
  schema: 'yilong.esk.asset_account.v2',
  mode: 'paper',
  enabled: true,
  simulated: true,
  funds_moved: false,
  asset: {
    asset_id: 'esk',
    symbol: 'ESK',
    name: '一龙 ESK',
    decimals: 6,
    issuance_mode: 'paper_recorded',
    chain_status: 'not_deployed',
    contract_address: null,
  },
  balance: {
    total: '1280.000000',
    available: '780.000000',
    reserved_for_sellback: '200.000000',
    reserved_for_quant: '300.000000',
    reserved_total: '500.000000',
    total_base_units: '1280000000',
    available_base_units: '780000000',
    sellback_reserved_base_units: '200000000',
    quant_reserved_base_units: '300000000',
    reserved_base_units: '500000000',
    revision: 4,
    updated_at: '2026-09-02T04:30:00Z',
  },
  sellback: {
    application_only: true,
    request_enabled: true,
    settlement_enabled: false,
    pricing_status: 'not_defined',
  },
  status_message: 'Paper 测试登记，尚未上链；可用余额已扣除卖回和量化申请占用，两类申请都不代表成交、付款、入金或收益。',
}

export const ESK_PREVIEW_REQUESTS: EskSellbackRequest[] = [{
  request_id: 'eskr_preview_001',
  amount: '200.000000',
  amount_base_units: '200000000',
  status: 'submitted',
  revision: 1,
  submitted_at: '2026-09-02T04:35:00Z',
  updated_at: '2026-09-02T04:35:00Z',
  simulated: true,
  funds_moved: false,
  replayed: false,
}]

export const ESK_PREVIEW_QUANT_REQUESTS: EskQuantAllocationRequest[] = [{
  request_id: 'eskq_preview_001',
  amount: '300.000000',
  amount_base_units: '300000000',
  risk_disclosure_revision: 'esk-quant-paper-allocation-v2',
  status: 'submitted',
  revision: 1,
  submitted_at: '2026-09-02T04:40:00Z',
  updated_at: '2026-09-02T04:40:00Z',
  simulated: true,
  funds_moved: false,
  position_created: false,
  allocation_binding_created: false,
  replayed: false,
}]
