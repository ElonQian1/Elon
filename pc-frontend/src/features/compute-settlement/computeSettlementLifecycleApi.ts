import { api } from '../../api/client'
import {
  type ComputeSettlementLifecycleHistoryResponse,
} from '../compute-attempt/settlementLifecycleHistoryContracts'

export type {
  ComputeSettlementLifecycleHistoryItem,
} from '../compute-attempt/settlementLifecycleHistoryContracts'

export const computeSettlementLifecycleApi = {
  listConsumer: (limit = 100) => api.get<ComputeSettlementLifecycleHistoryResponse>(
    `/api/me/compute/settlements/history?limit=${limit}`,
  ).then((response) => response.settlement_history),
  listProvider: (providerId: string, limit = 100) =>
    api.get<ComputeSettlementLifecycleHistoryResponse>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/settlements/history?limit=${limit}`,
    ).then((response) => response.settlement_history),
  listAdmin: (limit = 100) => api.get<ComputeSettlementLifecycleHistoryResponse>(
    `/api/admin/compute/settlements/history?limit=${limit}`,
  ).then((response) => response.settlement_history),
}
