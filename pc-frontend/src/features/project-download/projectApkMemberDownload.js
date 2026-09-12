export const OFFICIAL_QUANT_PROJECT_ID = 'yilong-quant'

const MEMBER_LOGIN_REQUIRED = '成功加入项目并登录后才能下载或更新量化 APK'
const DEFAULT_APK_FILENAME = 'yilong-quant.apk'

export function isMemberProtectedProjectApk(projectId) {
  return projectId === OFFICIAL_QUANT_PROJECT_ID
}

export function buildProjectApkMemberDownloadRequest(projectId, url, token) {
  if (!isMemberProtectedProjectApk(projectId)) {
    throw new Error('这个项目不使用成员专属 APK 下载入口')
  }
  const cleanToken = String(token ?? '').trim()
  if (!cleanToken) throw new Error(MEMBER_LOGIN_REQUIRED)

  const parsed = new URL(url)
  const expectedPath = `/api/store/projects/${OFFICIAL_QUANT_PROJECT_ID}/downloads/android`
  if (!/^https?:$/.test(parsed.protocol)
    || parsed.username
    || parsed.password
    || parsed.pathname !== expectedPath
    || parsed.search
    || parsed.hash) {
    throw new Error('量化 APK 下载地址不符合安全要求')
  }

  return {
    url: parsed.toString(),
    init: {
      cache: 'no-store',
      redirect: 'error',
      headers: { Authorization: `Bearer ${cleanToken}` },
    },
  }
}

export function projectApkDownloadFilename(contentDisposition) {
  const match = String(contentDisposition ?? '').match(/filename\s*=\s*"([^\"]+)"/i)
  const candidate = match?.[1]?.trim() ?? ''
  if (!/^[A-Za-z0-9._-]+\.apk$/i.test(candidate) || candidate.includes('..')) {
    return DEFAULT_APK_FILENAME
  }
  return candidate
}

export async function downloadMemberProtectedProjectApk({
  projectId,
  url,
  token,
  fetchImpl = globalThis.fetch,
  documentRef,
  urlApi = globalThis.URL,
  scheduleRevoke = callback => globalThis.setTimeout(callback, 1000),
}) {
  const request = buildProjectApkMemberDownloadRequest(projectId, url, token)
  let response
  try {
    response = await fetchImpl(request.url, request.init)
  } catch {
    throw new Error('APK 下载连接失败，请稍后重试')
  }
  if (!response.ok) {
    if (response.status === 401) throw new Error(MEMBER_LOGIN_REQUIRED)
    if (response.status === 403) throw new Error('当前账号尚未成功加入量化项目，暂不能下载或更新')
    let message = ''
    try {
      const payload = await response.json()
      message = typeof payload?.error === 'string' ? payload.error : ''
    } catch {
      // Keep the bounded fallback below when the server did not return JSON.
    }
    throw new Error(message || `APK 下载失败（HTTP ${response.status}）`)
  }

  const payload = await response.blob()
  const activeDocument = documentRef ?? globalThis.document
  if (!activeDocument?.body) throw new Error('当前环境无法保存 APK 文件')
  const objectUrl = urlApi.createObjectURL(payload)
  const anchor = activeDocument.createElement('a')
  anchor.href = objectUrl
  anchor.download = projectApkDownloadFilename(response.headers.get('content-disposition'))
  anchor.rel = 'noopener'
  activeDocument.body.appendChild(anchor)
  try {
    anchor.click()
  } finally {
    anchor.remove()
    scheduleRevoke(() => urlApi.revokeObjectURL(objectUrl))
  }
}
