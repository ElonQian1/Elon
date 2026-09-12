import assert from 'node:assert/strict'

import {
  buildProjectApkMemberDownloadRequest,
  downloadMemberProtectedProjectApk,
  isMemberProtectedProjectApk,
  projectApkDownloadFilename,
} from '../src/features/project-download/projectApkMemberDownload.js'

const fixedUrl = 'https://main.example/api/store/projects/yilong-quant/downloads/android'

assert.equal(isMemberProtectedProjectApk('yilong-quant'), true)
for (const projectId of ['', 'YILONG-QUANT', 'yilong-quant-copy']) {
  assert.equal(isMemberProtectedProjectApk(projectId), false)
}

assert.throws(
  () => buildProjectApkMemberDownloadRequest('yilong-quant', fixedUrl, ''),
  /成功加入项目并登录后才能下载或更新量化 APK/,
)

const request = buildProjectApkMemberDownloadRequest(
  'yilong-quant',
  fixedUrl,
  'member-secret',
)
assert.deepEqual(request, {
  url: fixedUrl,
  init: {
    cache: 'no-store',
    redirect: 'error',
    headers: { Authorization: 'Bearer member-secret' },
  },
})
assert.equal(request.url.includes('member-secret'), false)
assert.equal(projectApkDownloadFilename('attachment; filename="YilongQuant-0.7.38.apk"'), 'YilongQuant-0.7.38.apk')
assert.equal(projectApkDownloadFilename('attachment; filename="../secret.apk"'), 'yilong-quant.apk')

let capturedRequest
let clicked = false
let removed = false
let appended = false
let revoked = ''
const anchor = {
  href: '',
  download: '',
  rel: '',
  click() { clicked = true },
  remove() { removed = true },
}
await downloadMemberProtectedProjectApk({
  projectId: 'yilong-quant',
  url: fixedUrl,
  token: 'member-secret',
  fetchImpl: async (url, init) => {
    capturedRequest = { url, init }
    return new Response(new Blob(['signed-apk']), {
      status: 200,
      headers: { 'content-disposition': 'attachment; filename="YilongQuant-0.7.38.apk"' },
    })
  },
  documentRef: {
    createElement: () => anchor,
    body: { appendChild() { appended = true } },
  },
  urlApi: {
    createObjectURL: () => 'blob:member-apk',
    revokeObjectURL: value => { revoked = value },
  },
  scheduleRevoke: callback => callback(),
})
assert.deepEqual(capturedRequest, request)
assert.equal(anchor.download, 'YilongQuant-0.7.38.apk')
assert.equal(anchor.href, 'blob:member-apk')
assert.equal(appended, true)
assert.equal(clicked, true)
assert.equal(removed, true)
assert.equal(revoked, 'blob:member-apk')

await assert.rejects(
  downloadMemberProtectedProjectApk({
    projectId: 'yilong-quant',
    url: fixedUrl,
    token: 'outsider-token',
    fetchImpl: async () => new Response('', { status: 403 }),
  }),
  /尚未成功加入量化项目/,
)

console.log('PROJECT_APK_MEMBER_DOWNLOAD=passed')
