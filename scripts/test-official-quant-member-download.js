const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const read = file => fs.readFileSync(path.join(root, file), 'utf8').replace(/\r\n/g, '\n')
const server = read('server/src/project_store/apk.rs')
const legacyDownload = read('server/src/project_downloads.rs')
const androidActions = read('android/app/src/main/kotlin/com/elon/app/ProjectApkInstallActions.kt')
const androidEntry = read('android/app/src/main/kotlin/com/elon/app/ProjectSpaceDownloadButton.kt')
const installer = read('android/app/src/main/kotlin/com/elon/app/ApkChatInstaller.kt')
const plaza = read('pc-frontend/src/features/plaza/ProjectPlazaView.tsx')
const landing = read('pc-frontend/src/features/conversation/ProjectLandingDownloads.tsx')

assert.match(server, /official_quant[\s\S]*auth_from_headers\(&state, &headers\)[\s\S]*project_access/)
assert.match(server, /official_quant[\s\S]*viewer_role\.is_none\(\)[\s\S]*latest_apk_url\s*=\s*None/)
assert.match(legacyDownload, /download_user_project_apk[\s\S]*is_official_quant_project[\s\S]*auth_from_headers_or_query/)

assert.match(androidEntry, /AuthManager\.token\(activity\)\?\.trim\(\)/)
assert.doesNotMatch(androidEntry, /if \(officialQuant\) null else AuthManager\.token/)
assert.match(androidActions, /ProjectApkDownloadTarget\([\s\S]*expectedUrl,[\s\S]*bearerToken\s*=\s*cleanToken/)
assert.match(installer, /\.header\("Authorization",\s*"Bearer \$bearerToken"\)/)

assert.match(plaza, /downloadMemberProtectedProjectApk/)
assert.match(landing, /downloadMemberProtectedProjectApk/)
assert.match(landing, /projectRole === 'visitor'/)
assert.match(landing, /resolveApiUrl\('\/api\/store\/projects\/yilong-quant\/downloads\/android'\)/)

console.log('OFFICIAL_QUANT_MEMBER_DOWNLOAD_SOURCE_CONTRACT=passed')
