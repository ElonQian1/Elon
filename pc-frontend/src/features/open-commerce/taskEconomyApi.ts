import { api } from '../../api/client'
import type {
  SettlementDisputeDetail,
  SettlementDisputeReason,
  SettlementCorrectionDetail,
  SettlementCorrectionLineage,
  SettlementReceiptDetail,
  SuiCorrectionProjectionPackage,
  SuiProjectionPackage,
  SuiSettlementEnvelope,
  SuiTargetNetwork,
  TaskEconomyOverview,
  TaskEconomyProjectSetting,
} from './taskEconomyTypes'
import type { SuiAdapterHandoffBundle } from './suiAdapterHandoffTypes'

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
  suiProjectionAdapterHandoff: (projectId: string, projectionId: string) =>
    api.post<SuiAdapterHandoffBundle>(
      `${base(projectId)}/sui-projections/${encodeURIComponent(projectionId)}/adapter-handoff`,
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
  settlementCorrections: (projectId: string, receiptId: string) =>
    api.get<SettlementCorrectionDetail[]>(
      `${base(projectId)}/settlements/${encodeURIComponent(receiptId)}/corrections`,
    ),
  settlementLineage: (projectId: string, receiptId: string) =>
    api.get<SettlementCorrectionLineage>(
      `${base(projectId)}/settlements/${encodeURIComponent(receiptId)}/lineage`,
    ),
  createSettlementCorrection: (
    projectId: string,
    disputeId: string,
    input: {
      corrected_compute_amount_micros: number
      corrected_provider_amount_micros: number
      summary: string
      evidence_ref?: string
    },
  ) =>
    api.post<SettlementCorrectionDetail>(
      `${base(projectId)}/disputes/${encodeURIComponent(disputeId)}/corrections`,
      input,
    ),
  finalizeSettlementCorrection: (projectId: string, correctionId: string) =>
    api.post<SettlementCorrectionDetail>(
      `${base(projectId)}/corrections/${encodeURIComponent(correctionId)}/finalize`,
      {},
    ),
  suiCorrectionProjections: (projectId: string) =>
    api.get<SuiCorrectionProjectionPackage[]>(
      `${base(projectId)}/sui-correction-projections`,
    ),
  prepareSuiCorrectionProjection: (
    projectId: string,
    correctionId: string,
    targetNetwork: SuiTargetNetwork,
  ) =>
    api.post<SuiCorrectionProjectionPackage>(
      `${base(projectId)}/corrections/${encodeURIComponent(correctionId)}/sui-projections`,
      { target_network: targetNetwork },
    ),
  verifySuiCorrectionProjection: (projectId: string, projectionId: string) =>
    api.post<SuiCorrectionProjectionPackage>(
      `${base(projectId)}/sui-correction-projections/${encodeURIComponent(projectionId)}/verify`,
      {},
    ),
  suiCorrectionProjectionAdapterHandoff: (projectId: string, projectionId: string) =>
    api.post<SuiAdapterHandoffBundle>(
      `${base(projectId)}/sui-correction-projections/${encodeURIComponent(projectionId)}/adapter-handoff`,
      {},
    ),
}
