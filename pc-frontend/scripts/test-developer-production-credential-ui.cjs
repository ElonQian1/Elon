const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const projectRoot = path.resolve(__dirname, '..')
const modelPath = path.join(
  projectRoot,
  'src/features/open-commerce/developerProductionCredentialUiModel.ts',
)
const compiled = ts.transpileModule(fs.readFileSync(modelPath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const loaded = { exports: {} }
new Function('module', 'exports', 'require', compiled)(loaded, loaded.exports, require)

const {
  normalizeProductionCredentialRevocationReason,
  updateOneTimeProductionCredentialSecrets,
} = loaded.exports

let secrets = updateOneTimeProductionCredentialSecrets({}, {
  type: 'issue_succeeded',
  appRecordId: 'app-a',
  liveToken: '  oc_live_first  ',
})
assert.deepEqual(secrets, { 'app-a': 'oc_live_first' })

secrets = updateOneTimeProductionCredentialSecrets(secrets, {
  type: 'issue_started',
  appRecordId: 'app-a',
})
assert.deepEqual(secrets, {}, 'a rotation attempt must clear the previous one-time secret first')

secrets = updateOneTimeProductionCredentialSecrets({
  'app-a': 'oc_live_a',
  'app-b': 'oc_live_b',
}, {
  type: 'cleared',
  appRecordId: 'app-a',
})
assert.deepEqual(secrets, { 'app-b': 'oc_live_b' }, 'clearing one App must retain another App secret')

assert.equal(normalizeProductionCredentialRevocationReason('  主动轮换密钥  '), '主动轮换密钥')
assert.equal(normalizeProductionCredentialRevocationReason('   '), '项目方主动撤销生产凭据')
assert.throws(
  () => normalizeProductionCredentialRevocationReason('短'),
  /4 至 500 个可见字符/,
)
assert.throws(
  () => normalizeProductionCredentialRevocationReason(`合法原因\n注入`),
  /4 至 500 个可见字符/,
)

const reviewPanel = fs.readFileSync(
  path.join(projectRoot, 'src/features/open-commerce/DeveloperAppAdmissionReviewPanel.tsx'),
  'utf8',
)
assert.doesNotMatch(reviewPanel, /localStorage|sessionStorage/, 'one-time secrets must remain memory-only')
assert.match(reviewPanel, /type: 'issue_started'/, 'issuing must remove a stale one-time secret')
assert.match(reviewPanel, /await globalThis\.navigator\.clipboard\.writeText/, 'copy must await clipboard completion')
assert.match(reviewPanel, /复制失败/, 'clipboard failures must be visible')
assert.match(reviewPanel, /credentialSecret/, 'the one-time secret must use the responsive credential layout')

const credentialPanel = fs.readFileSync(
  path.join(projectRoot, 'src/features/open-commerce/DeveloperProductionCredentialPanel.tsx'),
  'utf8',
)
assert.match(credentialPanel, /normalizeProductionCredentialRevocationReason/, 'revocation must validate its reason')
assert.match(credentialPanel, /maxLength=\{500\}/, 'the revocation input must expose the server length limit')
assert.match(credentialPanel, /credentialRevokeActions/, 'revocation controls must use the responsive layout')

const styles = fs.readFileSync(
  path.join(projectRoot, 'src/features/open-commerce/OpenCommercePanel.module.css'),
  'utf8',
)
assert.match(styles, /\.credentialSecret code[\s\S]*overflow-wrap: anywhere/, 'long secrets must wrap safely')
assert.match(styles, /@media \(max-width: 820px\)[\s\S]*\.credentialRevokeActions/, 'small screens must stack revocation controls')

console.log('developer production credential UI model tests passed')
