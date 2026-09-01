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
  total_base_units: string
  available_base_units: string
  reserved_base_units: string
  revision: number
  updated_at: string | null
}

export interface EskAssetSnapshot {
  schema: 'yilong.esk.asset_account.v1'
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

export const eskAssetApi = {
  account: () => api.get<EskAssetSnapshot>('/api/me/assets/esk'),
  sellbackRequests: () => api.get<EskSellbackList>('/api/me/assets/esk/sellback-requests?limit=20'),
  createSellback: (amount: string, idempotencyKey: string) =>
    api.post<EskSellbackRequest>('/api/me/assets/esk/sellback-requests', {
      amount,
      idempotency_key: idempotencyKey,
    }),
  cancelSellback: (requestId: string) =>
    api.post<EskSellbackRequest>(`/api/me/assets/esk/sellback-requests/${encodeURIComponent(requestId)}/cancel`, {
      confirmation: 'CANCEL ESK SELLBACK REQUEST',
    }),
}

export const ESK_PREVIEW_SNAPSHOT: EskAssetSnapshot = {
  schema: 'yilong.esk.asset_account.v1',
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
    available: '1080.000000',
    reserved_for_sellback: '200.000000',
    total_base_units: '1280000000',
    available_base_units: '1080000000',
    reserved_base_units: '200000000',
    revision: 3,
    updated_at: '2026-09-02T04:30:00Z',
  },
  sellback: {
    application_only: true,
    request_enabled: true,
    settlement_enabled: false,
    pricing_status: 'not_defined',
  },
  status_message: 'Paper 测试登记，尚未上链；卖回仅提交申请，不代表成交或付款。',
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
