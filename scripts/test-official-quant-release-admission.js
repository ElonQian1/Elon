const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8')

const upload = read('server/src/project_releases.rs')
const declaration = read('server/src/project_releases/admission.rs')
const releases = read('server/src/store/project_releases.rs')
const storeAdmission = read('server/src/store/project_releases/admission.rs')
const landing = read('server/src/store/project_landing_snapshots.rs')
const storeApk = read('server/src/project_store/apk.rs')
const projectSpace = read('server/src/project_space.rs')
const installableFilter = read('server/src/store/projects/installable_apk.rs')
const pcApkSync = read('server/src/ai_cli/ai_cli_apk_sync.rs')

const validateDeclaration = upload.indexOf('admission::validate_release_declaration(declaration)')
const validatePayload = upload.indexOf('admission::validate_apk_payload(&body)')
const computeDigest = upload.indexOf('Sha256::digest(&body)')
const createDirectory = upload.indexOf('tokio::fs::create_dir_all(&release_dir)')
assert.ok(validateDeclaration >= 0 && validatePayload > validateDeclaration)
assert.ok(computeDigest > validatePayload && createDirectory > computeDigest,
  'official declaration and APK bytes must be rejected before any release directory is created')

assert.match(upload, /StatusCode::UNPROCESSABLE_ENTITY/)
assert.match(upload, /StatusCode::CONFLICT/)
assert.match(upload, /version_code: Option<String>/)
assert.match(upload, /parse_release_version_code\(query\.version_code\.as_deref\(\), official_quant\)/)
assert.match(upload, /if official_quant \{\s*StatusCode::UNPROCESSABLE_ENTITY\s*\} else \{\s*StatusCode::BAD_REQUEST/)
assert.match(upload, /official_apk\.as_ref\(\)/)
assert.match(upload, /idempotent_replay/)
const restoreReplay = upload.indexOf('restore_idempotent_official_quant_artifact(')
const cleanupReplay = upload.indexOf('cleanup_release_directory(&release_dir).await', restoreReplay)
assert.ok(restoreReplay >= 0 && cleanupReplay > restoreReplay,
  'an idempotent healthy upload must repair a missing/corrupt admitted artifact before cleanup')
assert.match(upload, /"artifact_repaired": artifact_repaired/)
assert.match(upload, /"installable"\.to_string\(\)/)
assert.match(upload, /verify_release_payload/)
assert.match(upload, /official_quant_release_is_installable/)
assert.match(upload, /\.filter\(\|release\| \{\s*!official_quant\s*\|\|/)

assert.match(declaration, /OFFICIAL_QUANT_PACKAGE_NAME[^\n]+"com\.elon\.quant"/)
assert.match(declaration, /OFFICIAL_QUANT_MIN_VERSION_CODE[^\n]+5/)
assert.match(declaration, /OFFICIAL_QUANT_MIN_VERSION_NAME[^\n]+"0\.5\.0"/)
assert.match(declaration, /OFFICIAL_QUANT_CHANNEL[^\n]+"paper"/)
assert.match(declaration, /APK Sig Block 42/)
assert.match(declaration, /central_directory_offset/)
assert.match(declaration, /ValidatedOfficialQuantApk/)
assert.match(declaration, /matches_artifact/)
assert.match(declaration, /Sha256::digest\(payload\)/)

assert.match(releases, /create_project_release_with_admission\(write, None\)/)
assert.match(releases, /official_apk\.ok_or/)
assert.match(releases, /latest_installable_official_quant_release/)
assert.match(releases, /is_official_quant_project\(&release\.project_id\)[\s\S]*release\.version_code/)
assert.match(releases, /match \(version_name, release\.release_number\)/)
assert.match(releases, /cannot clone or\s*\/\/ synthesize an installable official quant APK/)
assert.match(releases, /!official_quant_release_is_installable\(release\)/)
assert.match(storeAdmission, /TransactionBehavior::Immediate/)
assert.match(storeAdmission, /VersionRollback/)
assert.match(storeAdmission, /VersionConflict/)
assert.match(storeAdmission, /ArtifactRelabeled/)
assert.match(storeAdmission, /OFFICIAL_QUANT_ADMISSION_SCHEMA/)
assert.match(storeAdmission, /apk_signing_block_structure_present/)
assert.match(storeAdmission, /cryptographic_signature_verified/)
assert.match(storeAdmission, /ArtifactProofMismatch/)
assert.match(storeAdmission, /artifact_sha256/)
assert.match(storeAdmission, /artifact_size_bytes/)

assert.match(landing, /is_official_quant_project\(project_id\)/)
assert.match(landing, /latest_project_release\(project_id\)/)
assert.match(installableFilter, /CASE WHEN p\.id = 'yilong-quant' THEN EXISTS/)
assert.match(installableFilter, /yilong\.official_quant_release_admission\.v1/)
assert.match(installableFilter, /artifact_sha256/)
assert.match(installableFilter, /artifact_size_bytes/)
const officialSyncGate = pcApkSync.indexOf(
  'project_id.is_some_and(crate::project_releases::admission::is_official_quant_project)',
)
const syncCall = pcApkSync.indexOf('sync_pc_agent_apk_artifact(', officialSyncGate)
assert.ok(officialSyncGate >= 0 && syncCall > officialSyncGate,
  'official quant task sync must stop before copying or advertising an unadmitted APK')
assert.match(storeApk, /if !official_quant && project\.latest_apk_url\.is_some\(\)/)
assert.match(storeApk, /if official_quant \{\s*project\.latest_apk_url = None;/)
const officialSpaceGate = projectSpace.indexOf(
  'if !crate::project_releases::admission::is_official_quant_project(&project.id)',
)
const historicalTaskLookup = projectSpace.indexOf(
  'state.store.latest_project_apk_delivery(&project.id)',
  officialSpaceGate,
)
assert.ok(officialSpaceGate >= 0 && historicalTaskLookup > officialSpaceGate,
  'official quant project space must not present historical task APK deliveries')

console.log('Official quant release admission source contract passed')
