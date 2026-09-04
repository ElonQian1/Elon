const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

// Source wiring checks complement Kotlin behavior tests; they do not run Android.
const root = path.resolve(__dirname, '..')
const android = 'android/app/src/main/kotlin/com/elon/app/'
const read = file => fs.readFileSync(path.join(root, file), 'utf8').replace(/\r\n/g, '\n')
const files = {
  policy: 'OfficialQuantApkPolicy.kt', launcher: 'OfficialQuantApkLauncher.kt',
  guard: 'ProjectApkSignatureGuard.kt', installer: 'ApkChatInstaller.kt',
  actions: 'ProjectApkInstallActions.kt', download: 'ProjectSpaceDownloadButton.kt',
  privateFile: 'OfficialQuantApkFile.kt', store: 'MainStoreApi.kt', controller: 'ProjectSpaceController.kt',
}
const sources = Object.fromEntries(Object.entries(files).map(([key, file]) => [
  key, read(android + file),
]))
sources.paths = read('android/app/src/main/res/xml/file_paths.xml')
sources.manifest = read('android/app/src/main/AndroidManifest.xml')
sources.catalog = read('server/src/official_project_catalog/mod.rs')
const pin = '019a3d95366fb4c6fe578c1f7f26fb96e462dc54f41b9a7c7b5a715052e418bb'
const stripComments = source => source.replace(/<!--[\s\S]*?-->/g, '').replace(
  /("(?:\\.|[^"\\])*"|\/\/[^\n]*|\/\*[\s\S]*?\*\/)/g,
  token => token.startsWith('"') ? token : '',
)
function functionSource(source, name) {
  const start = source.search(new RegExp(`^(?:internal|private) fun ${name}\\(`, 'm'))
  assert.ok(start >= 0, `missing function ${name}`)
  const remainder = source.slice(start)
  const next = remainder.slice(1).search(/^(?:internal|private) fun /m)
  return next < 0 ? remainder : remainder.slice(0, next + 1)
}
function before(source, first, second, message) {
  const a = source.indexOf(first)
  const b = source.indexOf(second)
  assert.ok(a >= 0 && b > a, message)
}

function verify(raw) {
  const s = Object.fromEntries(Object.entries(raw).map(([key, value]) => [key, stripComments(value)]))
  for (const [name, value] of Object.entries({
    PROJECT_ID: 'yilong-quant', PACKAGE_NAME: 'com.elon.quant',
    ACTIVITY_NAME: 'com.elon.quant.MainActivity', SIGNER_SHA256: pin,
  })) assert.ok(s.policy.includes(`const val ${name} = "${value}"`), `fixed identity ${name}`)
  assert.match(s.policy, /MIN_VERSION_CODE\s*=\s*2L\b/, 'minimum reviewed version')
  assert.match(s.policy, /projectId\?\.trim\(\)\s*==\s*PROJECT_ID/, 'stable project ID only')
  assert.match(s.policy, /packageName\s*==\s*PACKAGE_NAME\s*&&/, 'exact official package')
  assert.match(s.policy, /currentSigners\s*==\s*setOf\(SIGNER_SHA256\)\s*&&/, 'exact single current signer')
  assert.match(s.policy, /versionCode\s*!=\s*null\s*&&\s*versionCode\s*>=\s*MIN_VERSION_CODE/, 'version gate')

  const current = functionSource(s.guard, 'currentPackageSignerSha256')
  assert.match(current, /info\.signingInfo\?\.apkContentsSigners/, 'current Android 28+ signer')
  assert.match(current, /info\.signatures/, 'legacy Android current signatures')
  assert.doesNotMatch(current, /signingCertificateHistory/, 'identity cannot use certificate history')
  assert.match(current, /sha256Hex\(it\.toByteArray\(\)\)/, 'certificate bytes must be hashed')
  assert.match(current, /getOrDefault\(emptySet\(\)\)/, 'unreadable signer must fail closed')
  const inspect = functionSource(s.guard, 'inspectProjectApkSignature')
  assert.match(inspect, /projectId:\s*String\?\s*=\s*null/, 'existing callers remain compatible')
  assert.match(inspect, /OfficialQuantApkPolicy\.appliesTo\(projectId\)\s*&&\s*!OfficialQuantApkPolicy\.accepts\(/)
  assert.match(inspect, /archive\.packageName,\s*currentPackageSignerSha256\(archive\),\s*archive\.projectApkVersionCode\(\)/)
  before(inspect, 'OFFICIAL_IDENTITY_MISMATCH', 'evaluateProjectApkSignatureCompatibility(', 'official gate must precede general install compatibility')
  assert.match(inspect, /return ProjectApkSignatureInspection\(\s*ProjectApkSignatureCompatibility\.OFFICIAL_IDENTITY_MISMATCH/)
  const compatibility = functionSource(s.guard, 'evaluateProjectApkSignatureCompatibility')
  assert.match(compatibility, /archiveSignerSha256\.any\(installedSignerSha256::contains\)/, 'nonofficial lineage compatibility remains')
  assert.match(compatibility, /installedPackageName\.isNullOrBlank\(\)/, 'nonofficial fresh install remains')
  assert.match(s.installer, /inspectProjectApkSignature\(activity,\s*apkFile,\s*projectId\)/, 'download path passes project identity')
  before(s.installer, 'if (!signatureDecision.allowed)', 'installApk(activity, apkFile)', 'installation must follow rejection gate')
  assert.match(s.privateFile, /File\(privateCacheDirectory, "official-quant-apk"\)/, 'private narrow directory')
  assert.match(s.privateFile, /return File\.createTempFile\("quant-", "\.apk", directory\)/, 'unique file for every attempt')
  assert.match(s.installer, /val officialQuant = OfficialQuantApkPolicy\.appliesTo\(projectId\)/)
  assert.match(s.installer, /if \(officialQuant\) createOfficialQuantApkFile\(activity\.cacheDir\) else File\(/, 'official bytes cannot use shared/external storage')
  assert.match(s.installer, /if \(officialQuant\) check\(apkFile\.setReadOnly\(\)\)/, 'protection failure stops installation')
  before(s.installer, 'apkFile.outputStream()', 'check(apkFile.setReadOnly())', 'finish download before sealing')
  before(s.installer, 'check(apkFile.setReadOnly())', 'inspectProjectApkSignature(', 'seal bytes before signature inspection')
  assert.match(s.installer, /FileProvider\.getUriForFile\(\s*activity,\s*"\$\{activity.packageName\}\.update_provider",\s*apkFile/)
  assert.match(s.installer, /Intent\.FLAG_GRANT_READ_URI_PERMISSION/)
  assert.doesNotMatch(s.installer, /FLAG_GRANT_WRITE_URI_PERMISSION/, 'installer receives read-only access')
  const cachePaths = [...s.paths.matchAll(/<cache-path\b[^>]*\/>/g)].map(match => match[0])
  assert.equal(cachePaths.filter(tag => /name="official_quant_apk"/.test(tag)).length, 1)
  assert.ok(cachePaths.some(tag => /name="official_quant_apk"/.test(tag) && /path="official-quant-apk\/"/.test(tag)), 'provider exposes only official APK directory')
  assert.ok(cachePaths.every(tag => !/path="(?:\.|\/|\.\/|)"/.test(tag)), 'private cache root must not be exposed')
  const provider = s.manifest.match(/<provider\b[^>]*android:name="androidx\.core\.content\.FileProvider"[^>]*>[\s\S]*?<\/provider>/)?.[0] || ''
  for (const attribute of ['${applicationId}.update_provider', 'android:exported="false"',
    'android:grantUriPermissions="true"', 'android:resource="@xml/file_paths"']) assert.ok(provider.includes(attribute), `provider wiring: ${attribute}`)

  const open = functionSource(s.actions, 'openInstalledProjectApp')
  assert.match(open, /if \(OfficialQuantApkPolicy\.appliesTo\(projectId\)\) return openOfficialQuantApp\(activity\)/)
  before(open, 'return openOfficialQuantApp(activity)', 'resolveInstalledProjectApp(', 'official launch bypasses generic resolution')
  const resolve = functionSource(s.actions, 'resolveInstalledProjectApp')
  assert.match(resolve, /if \(OfficialQuantApkPolicy\.appliesTo\(projectId\)\)\s*\{\s*return resolveInstalledPackage\(activity, OfficialQuantApkPolicy\.PACKAGE_NAME\)\s*\}/)
  before(resolve, 'OfficialQuantApkPolicy.PACKAGE_NAME', 'resolveStoredPackage(', 'official discovery bypasses stored package')
  before(resolve, 'OfficialQuantApkPolicy.PACKAGE_NAME', 'resolveInstalledAppByLabel(', 'official discovery bypasses display name')
  assert.match(s.download, /openInstalledProjectApp\(activity, projectId, projectName\)/, 'project page launch is wired')
  assert.match(s.actions, /projectId\s*=\s*projectId/, 'project identity reaches downloader')
  assert.match(s.catalog, /INSERT OR IGNORE INTO projects\s*\(\s*id, name,[\s\S]*?VALUES \(\?1,[\s\S]*?params!\[\s*project\.id,\s*project\.name,/, 'catalog ID persists as project ID')
  assert.match(s.store, /fun parseStoreProject\(obj: JSONObject\) = StoreProject\(\s*id = obj\.getString\("id"\)/, 'Android retains server project ID')
  assert.match(s.controller, /downloadProjectApk = \{\s*val space = activeSpace\s*openProjectApkDownload\(\s*activity,\s*space\?\.latestApkUrl,\s*space\?\.project\?\.id,/, 'project space passes stable ID into installation')

  const launch = functionSource(s.launcher, 'openOfficialQuantApp')
  assert.match(launch, /readInstalledPackageInfo\(manager, OfficialQuantApkPolicy\.PACKAGE_NAME\)/)
  assert.match(launch, /OfficialQuantApkPolicy\.accepts\(\s*installed\.packageName,\s*currentPackageSignerSha256\(installed\),\s*installed\.projectApkVersionCode\(\)/)
  assert.match(launch, /if \(!trusted\)\s*\{[\s\S]*?return false\s*\}/, 'untrusted app cannot launch')
  before(launch, 'currentPackageSignerSha256(installed)', 'activity.startActivity(intent)', 'fresh signature check precedes every launch')
  assert.match(launch, /ComponentName\(OfficialQuantApkPolicy\.PACKAGE_NAME, OfficialQuantApkPolicy\.ACTIVITY_NAME\)/)
  assert.match(launch, /manager\.getActivityInfo\(component, 0\)/)
  for (const gate of ['!target.enabled', '!target.exported', '!target.applicationInfo.enabled',
    'target.packageName != component.packageName', 'target.name != component.className',
    'target.targetActivity != null']) assert.ok(launch.includes(gate), `activity gate: ${gate}`)
  assert.match(launch, /val intent = Intent\(Intent\.ACTION_MAIN\)\.apply\s*\{[^}]*setComponent\(component\)/)
  for (const forbidden of [
    /putExtra|putExtras|replaceExtras|\.extras\s*=|setData|setDataAndType|\.data\s*=|\.clipData\s*=/,
    /getLaunchIntentForPackage|resolveInstalledAppByLabel|resolveStoredPackage|SharedPreferences/,
    /AuthManager|paper-access-grants|paper-launch|Authorization|Bearer|ypg1|yep2|yeqa1/,
    /WebView|loadUrl|evaluateJavascript|addJavascriptInterface|OkHttp|newCall|Request\.Builder/,
  ]) assert.doesNotMatch(launch, forbidden, 'identity-only explicit launch must not gain handoff or fallback')
}

verify(sources)
// Prove high-risk wiring regressions are caught instead of merely checking file existence.
let mutations = 0
for (const [key, from, to] of [
  ['policy', pin, '0'.repeat(64)],
  ['policy', 'MIN_VERSION_CODE = 2L', 'MIN_VERSION_CODE = 1L'],
  ['policy', 'currentSigners == setOf(SIGNER_SHA256)', 'currentSigners.contains(SIGNER_SHA256)'],
  ['guard', 'info.signingInfo?.apkContentsSigners', 'info.signingInfo?.signingCertificateHistory'],
  ['installer', 'inspectProjectApkSignature(activity, apkFile, projectId)', 'inspectProjectApkSignature(activity, apkFile)'],
  ['actions', 'return openOfficialQuantApp(activity)', 'openOfficialQuantApp(activity)'],
  ['actions', 'return resolveInstalledPackage(activity, OfficialQuantApkPolicy.PACKAGE_NAME)', 'resolveInstalledPackage(activity, OfficialQuantApkPolicy.PACKAGE_NAME)'],
  ['launcher', 'currentPackageSignerSha256(installed)', 'setOf(OfficialQuantApkPolicy.SIGNER_SHA256)'],
  ['launcher', 'setComponent(component)', 'setComponent(component)\nputExtra("credential", "value")'],
  ['launcher', '!target.exported', 'false'],
  ['installer', 'createOfficialQuantApkFile(activity.cacheDir)', 'createOfficialQuantApkFile(activity.getExternalFilesDir(null))'],
  ['installer', 'check(apkFile.setReadOnly())', 'apkFile.setReadOnly()'],
  ['paths', 'path="official-quant-apk/"', 'path="."'],
]) {
  assert.ok(sources[key].includes(from), `mutation anchor missing: ${key}`)
  assert.throws(() => verify({ ...sources, [key]: sources[key].replace(from, to) }), `regression undetected: ${key}`)
  mutations += 1
}
console.log('OFFICIAL_QUANT_APK_IDENTITY_SOURCE_CONTRACT=passed')
console.log(`OFFICIAL_QUANT_APK_IDENTITY_MUTATION_CHECKS=${mutations}_passed`)
console.log('OFFICIAL_QUANT_APK_IDENTITY_ANDROID_RUNTIME=not_performed_by_this_script')
