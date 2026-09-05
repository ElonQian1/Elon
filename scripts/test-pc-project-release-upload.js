const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const releaseTab = fs.readFileSync(
  path.join(root, 'pc-frontend/src/features/projects/ProjectReleasesTab.tsx'),
  'utf8',
)
const releaseTypes = fs.readFileSync(
  path.join(root, 'pc-frontend/src/features/projects/projectManagementTypes.ts'),
  'utf8',
)

assert.match(releaseTab, /OFFICIAL_QUANT_PROJECT_ID\s*=\s*'yilong-quant'/)
assert.match(releaseTab, /OFFICIAL_QUANT_PACKAGE_NAME\s*=\s*'com\.elon\.quant'/)
assert.match(releaseTab, /OFFICIAL_QUANT_CHANNEL\s*=\s*'paper'/)
assert.match(releaseTab, /projectId\s*===\s*OFFICIAL_QUANT_PROJECT_ID/)

for (const field of ['version_code', 'source_git_sha']) {
  assert.ok(releaseTab.includes(`params.set('${field}'`), `release upload must send ${field}`)
}
assert.equal(
  /params\.set\(['"]sha256['"]/.test(releaseTab),
  false,
  'the browser must not declare the APK SHA-256; the server computes it from the payload',
)

assert.match(releaseTab, /OFFICIAL_QUANT_MIN_VERSION_CODE\s*=\s*5/)
assert.match(releaseTab, /OFFICIAL_QUANT_MIN_VERSION_NAME\s*=\s*'0\.5\.0'/)
assert.match(releaseTab, /SOURCE_GIT_SHA_PATTERN\s*=\s*\/\^\[0-9a-f\]\{40\}\$\//)
assert.match(releaseTab, /readOnly=\{isOfficialQuant\}/)
assert.match(releaseTab, /versionCode/)
assert.match(releaseTab, /sourceGitSha/)

for (const receiptLabel of ['versionCode', '源码 Git SHA', '服务器 SHA-256']) {
  assert.ok(releaseTab.includes(receiptLabel), `release history must display ${receiptLabel}`)
}
for (const field of ['version_code', 'source_git_sha', 'sha256', 'installable']) {
  assert.ok(releaseTypes.includes(field), `ProjectRelease response type must preserve ${field}`)
}
assert.match(releaseTab, /release\.installable !== true/)
assert.match(releaseTab, /审计记录，不可安装/)
assert.match(releaseTab, /releases\.find\(\(release\) => release\.installable === true\)/)
assert.match(releaseTab, /暂无可安装新版/)
assert.match(releaseTab, /fetch\(releaseDownloadUrl\(projectId, release\.id\)/)
assert.match(releaseTab, /Authorization: `Bearer \$\{token\}`/)
assert.match(releaseTab, /response\.blob\(\)/)
assert.match(releaseTab, /URL\.createObjectURL\(blob\)/)
assert.doesNotMatch(
  releaseTab,
  /<a[^>]+href=\{releaseDownloadUrl\(/,
  'authenticated release downloads must not use a credential-less browser navigation',
)

console.log('PC project release upload contract passed')
