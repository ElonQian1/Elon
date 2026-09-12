export const OFFICIAL_QUANT_PROJECT_ID: 'yilong-quant'

export function isMemberProtectedProjectApk(projectId: string | null | undefined): boolean

export function buildProjectApkMemberDownloadRequest(
  projectId: string,
  url: string,
  token: string | null | undefined,
): { url: string; init: RequestInit }

export function projectApkDownloadFilename(contentDisposition: string | null | undefined): string

export function downloadMemberProtectedProjectApk(options: {
  projectId: string
  url: string
  token: string | null | undefined
  fetchImpl?: typeof fetch
  documentRef?: Document
  urlApi?: typeof URL
  scheduleRevoke?: (callback: () => void) => unknown
}): Promise<void>
