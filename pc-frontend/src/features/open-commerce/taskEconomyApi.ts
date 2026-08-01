import { api } from '../../api/client'
import type {
  SettlementDisputeDetail,
  SettlementDisputeReason,
  SettlementReceiptDetail,
  SuiProjectionPackage,
  SuiSettlementEnvelope,
  SuiTargetNetwork,
  TaskEconomyOverview,
  TaskEconomyProjectSetting,
} from './taskEconomyTypes'

function base(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/economy`
}

export const taskEconomyApi = {
  overview: (projectId: string) => api.get<TaskEconomyOverview>(`${base(projectId)}/overview`),
  updateSetting: (projectId: string, enabled: boolean) =>
    api.patch<TaskEconomyProjectSetting>(`${base(projectId)}/settings`, { enabled }),
  receipt: (projectId: string, receiptId: string) =>
    api.get<SettlementReceiptDetail>(
      `${base(projectId)}/settlements/${encodeURIComponent(receiptId)}`,
    ),
  suiEnvelope: (projectId: string, receiptId: string) =>
    api.get<SuiSettlementEnvelope>(
      `${base(projectId)}/settlements/${encodeURIComponent(receiptId)}/sui-envelope`,
    ),
  suiProjections: (projectId: string) =>
    api.get<SuiProjectionPackage[]>(`${base(projectId)}/sui-projections`),
  prepareSuiProjection: (
    projectId: string,
    receiptId: string,
    targetNetwork: SuiTargetNetwork,
  ) =>
    api.post<SuiProjectionPackage>(
      `${base(projectId)}/settlements/${encodeURIComponent(receiptId)}/sui-projections`,
      { target_network: targetNetwork },
    ),
  verifySuiProjection: (projectId: string, projectionId: string) =>
    api.post<SuiProjectionPackage>(
      `${base(projectId)}/sui-projections/${encodeURIComponent(projectionId)}/verify`,
      {},
    ),
  settlementDisputes: (projectId: string, receiptId: string) =>
    api.get<SettlementDisputeDetail[]>(
      `${base(projectId)}/settlements/${encodeURIComponent(receiptId)}/disputes`,
    ),
  openSettlementDispute: (
    projectId: string,
    receiptId: string,
    input: { reason_code: SettlementDisputeReason; summary: string; evidence_ref?: string },
  ) =>
    api.post<SettlementDisputeDetail>(
      `${base(projectId)}/settlements/${encodeURIComponent(receiptId)}/disputes`,
      input,
    ),
  withdrawSettlementDispute: (projectId: string, disputeId: string, note: string) =>
    api.post<SettlementDisputeDetail>(
      `${base(projectId)}/disputes/${encodeURIComponent(disputeId)}/withdraw`,
      { note },
    ),
  resolveSettlementDispute: (
    projectId: string,
    disputeId: string,
    decision: 'accept' | 'reject',
    note: string,
  ) =>
    api.post<SettlementDisputeDetail>(
      `${base(projectId)}/disputes/${encodeURIComponent(disputeId)}/resolve`,
      { decision, note },
    ),
}
