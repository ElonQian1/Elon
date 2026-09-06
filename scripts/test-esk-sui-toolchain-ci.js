const assert = require('node:assert/strict')
const childProcess = require('node:child_process')
const crypto = require('node:crypto')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const fromRoot = (relative) => path.join(root, relative)
const read = (relative) => fs.readFileSync(fromRoot(relative), 'utf8')
const parse = (relative) => JSON.parse(read(relative))
const sha256 = (bytes) => `sha256:${crypto.createHash('sha256').update(bytes).digest('hex')}`

const configPath = 'scripts/esk-sui-toolchain-ci/toolchain-v1.json'
const installerPath = 'scripts/install-esk-sui-toolchain.ps1'
const validatorPath = 'scripts/validate-esk-sui-move.ps1'
const validatorHelpersPath = 'scripts/esk-sui-toolchain-ci/validation-helpers.psm1'
const artifactVerifierPath = 'scripts/esk-sui-toolchain-ci/verify-artifacts.js'
const workflowPath = '.github/workflows/ci.yml'

for (const relative of [
  configPath,
  installerPath,
  validatorPath,
  validatorHelpersPath,
  artifactVerifierPath,
  workflowPath,
]) {
  assert.ok(fs.existsSync(fromRoot(relative)), `missing implementation: ${relative}`)
}

const config = parse(configPath)
const installer = read(installerPath)
const validator = read(validatorPath)
const validatorHelpers = read(validatorHelpersPath)
const validationImplementation = `${validator}\n${validatorHelpers}`
const toolchainImplementation = `${installer}\n${validationImplementation}`
const workflow = read(workflowPath)
const workflowLf = workflow.replace(/\r\n?/g, '\n')
const fixture = parse('contracts/sui/esk-allocation-policy-v1.fixture.json')
const manifest = parse('contracts/sui/esk-genesis-manifest-v1.fixture.json')
const verifier = require(fromRoot(artifactVerifierPath))

assert.deepEqual(Object.keys(config).sort(), ['cli', 'framework', 'packages', 'schema', 'sui_source_commit'])
assert.equal(config.schema, 'yilong.esk.sui.reproducible_toolchain.v1')
assert.equal(config.sui_source_commit, '46f18562f1f5af2438d35828e8b62d5e0b972db7')
assert.deepEqual(Object.keys(config.framework).sort(), [
  'archive_root',
  'archive_sha256',
  'archive_size',
  'archive_url',
  'tracked_content_digest',
  'tracked_file_count',
  'tracked_roots',
])
assert.equal(config.framework.archive_url,
  'https://codeload.github.com/MystenLabs/sui/tar.gz/46f18562f1f5af2438d35828e8b62d5e0b972db7')
assert.equal(config.framework.archive_size, 87498700)
assert.equal(config.framework.archive_sha256,
  'sha256:9046fce263794cca6772c59aacd328706d42522dd7408c0dcb4a7f2613016afd')
assert.equal(config.framework.archive_sha256,
  fixture.runtime_verification.framework_source_archive_sha256)
assert.equal(config.framework.archive_root,
  'sui-46f18562f1f5af2438d35828e8b62d5e0b972db7')
assert.deepEqual(config.framework.tracked_roots, [
  'crates/sui-framework/packages/move-stdlib',
  'crates/sui-framework/packages/sui-framework',
])
assert.equal(config.framework.tracked_file_count, 187)
assert.equal(config.framework.tracked_file_count,
  fixture.runtime_verification.framework_blob_verification.expected)
assert.equal(config.framework.tracked_content_digest,
  'sha256:017e6a38b5d976c87b710e02b39d26988691d101bf914a42e6509c82d62e027b')
assert.equal(config.cli.release, 'testnet-v1.79.0')
assert.equal(config.cli.version, 'sui 1.79.0-46f18562f1f5')
assert.equal(config.cli.platform, 'windows-x86_64')
assert.equal(config.cli.asset_name, 'sui-testnet-v1.79.0-windows-x86_64.tgz')
assert.equal(config.cli.asset_url,
  'https://github.com/MystenLabs/sui/releases/download/testnet-v1.79.0/sui-testnet-v1.79.0-windows-x86_64.tgz')
assert.equal(config.cli.archive_size, 273621059)
assert.equal(config.cli.archive_sha256,
  'sha256:9d8442bad8fd516116a76ff52213cdfd43b98857c3f7fbbde124ed5e2041a558')
assert.equal(config.cli.binary_size, 138727424)
assert.equal(config.cli.binary_sha256, fixture.runtime_verification.sui_cli_sha256)
assert.equal(config.cli.source_commit, fixture.runtime_verification.sui_source_commit)
assert.equal(config.cli.release, fixture.runtime_verification.sui_release)
assert.equal(config.cli.version, fixture.runtime_verification.sui_cli_version)

assert.deepEqual(config.packages.map((item) => item.id), ['esk_currency', 'yilong_participation'])
assert.equal(config.packages[0].test_count, 3)
assert.equal(config.packages[0].path, 'contracts/sui/esk_currency')
assert.equal(config.packages[0].package_input_digest,
  'sha256:6266890971f1c4ee6be82e9441959ebf0d4588e0483113a99ec61e66ea208665')
assert.equal(config.packages[0].build_evidence.kind, 'single_module_sha256_v1')
assert.deepEqual(config.packages[0].build_evidence.modules, ['esk'])
assert.equal(config.packages[0].test_evidence.kind, 'canonical_test_receipt_sha256_v1')
assert.equal(config.packages[0].test_evidence.path,
  'contracts/sui/yilong_participation/evidence/esk-currency-regression-output-v1.txt')
assert.equal(config.packages[0].build_evidence.digest, manifest.toolchain.build_evidence_digest)
assert.equal(config.packages[0].test_evidence.digest, manifest.toolchain.test_evidence_digest)
assert.equal(config.packages[1].test_count, 13)
assert.equal(config.packages[1].path, 'contracts/sui/yilong_participation')
assert.equal(config.packages[1].package_input_digest,
  'sha256:7ca8100f358f179b9d937a091373d4dbcdefd2cd6b78c0de223409e07f5f0038')
assert.equal(config.packages[1].build_evidence.kind, 'production_bytecode_bundle_v1')
assert.deepEqual(config.packages[1].build_evidence.modules, ['genesis_allocation', 'team_vesting'])
assert.equal(config.packages[1].test_evidence.kind, 'canonical_test_receipt_sha256_v1')
assert.equal(config.packages[1].test_evidence.path,
  'contracts/sui/yilong_participation/evidence/move-test-output-v1.txt')
assert.equal(config.packages[1].test_evidence.kind,
  fixture.runtime_verification.move_test.evidence_kind)
assert.equal(config.packages[1].build_evidence.digest,
  fixture.runtime_verification.move_build.evidence_digest)
assert.equal(config.packages[1].test_evidence.digest,
  fixture.runtime_verification.move_test.evidence_digest)

for (const value of [installer, validationImplementation]) {
  assert.doesNotMatch(value, /Invoke-Expression|cmd\.exe|sui\s+(?:client|keytool|genesis|start)|move\s+publish/i)
  assert.doesNotMatch(value, /mainnet|seed[_-]?phrase|--yes|\.sui[\\/]sui_config/i)
}
assert.doesNotMatch(validationImplementation,
  /Invoke-(?:WebRequest|RestMethod|Command)|Start-(?:BitsTransfer|Process|Job)|System\.Net|HttpClient|WebClient|WebRequest|Test-NetConnection|New-PSSession|\b(?:curl|wget)(?:\.exe)?\b/i,
  'validator must not contain a network or indirect process execution surface')
assert.deepEqual(validationImplementation.match(/&\s+\$[A-Za-z][A-Za-z0-9_]*/g), [
  '& $FilePath',
  '& $binaryPath',
  '& $TarPath',
  '& $TarPath',
], 'validator call-operator sites must remain the reviewed capture helper, fixed CLI version probe, and two trusted-tar calls')
const validatorLines = validator.replace(/\r\n?/g, '\n').split('\n').map((line) => line.trim())
assert.equal(validatorLines.filter((line) => line ===
  '$rawLines = @(& $FilePath @Arguments 2>&1 | ForEach-Object { $_.ToString() })').length, 1,
  'captured child execution must keep its exact reviewed argument forwarding')
assert.equal(validatorLines.filter((line) => line ===
  '$versionOutput = @(& $binaryPath --version 2>&1 | ForEach-Object { $_.ToString() })').length, 1,
  'the only direct fixed-CLI probe must remain the exact non-network --version command')
assert.match(installer, /archive_sha256/)
assert.match(installer, /binary_sha256/)
assert.match(installer, /AllowAutoRedirect\s*=\s*\$false/)
assert.match(installer, /archive_size/)
assert.match(installer, /binary_size/)
assert.match(installer, /\$entry -cne "\.\/sui\.exe"/)
assert.match(installer, /SUI_TOOLCHAIN_STATUS=verified/)
assert.match(installer, /FrameworkArchivePath/)
assert.match(installer, /framework-source\.tar\.gz/)
assert.match(installer, /framework\.archive_url/)
assert.match(installer, /framework\.archive_size/)
assert.match(installer, /framework\.archive_sha256/)
assert.match(installer, /ContentLength/)
assert.match(installer, /\$null -ne \$declaredLength -and \[long\]\$declaredLength -ne \$ExpectedLength/)
assert.match(installer, /\$ExpectedLength \+ 1L/)
assert.match(installer, /CancellationTokenSource/)
assert.match(installer, /CancelAfter\(\[System\.TimeSpan\]::FromMinutes\(10\)\)/)
assert.match(installer, /ResponseHeadersRead/)
assert.match(installer, /ReadAsync\(/)
assert.match(installer, /WriteAsync\(/)
assert.match(installer, /FlushAsync\(/)
assert.match(installer, /AllowAutoRedirect\s*=\s*\$false/)
assert.match(installer, /\$redirects -ge 3/)
assert.match(installer, /IsDefaultPort/)
assert.match(installer, /UserInfo/)
assert.match(installer, /ReadToEndAsync\(\)/)
assert.match(installer, /WaitForExitAsync\(/)
assert.match(installer, /CancelAfter\(30000\)/)
assert.match(installer, /Kill\(\$true\)/)
assert.match(installer, /GetFolderPath/)
assert.match(installer, /Get-AuthenticodeSignature/)
assert.match(installer, /ReparsePoint/)
assert.match(installer, /Assert-NoReparsePathChain/)
assert.match(installer, /O=Microsoft Corporation/)
assert.match(installer, /release-assets\.githubusercontent\.com/)
assert.match(installer, /codeload\.github\.com/)
assert.doesNotMatch(installer, /Get-Command\s+tar/i)
assert.match(validator, /Import-Module/)
assert.match(validator, /validation-helpers\.psm1/)
assert.match(validator, /\[Parameter\(Mandatory = \$true\)\]\s*\[string\]\$FrameworkArchivePath/)
assert.match(validationImplementation, /\$allowedOperations\s*=\s*@\("build",\s*"test"\)/)
assert.match(validationImplementation, /SUI_CONFIG_DIR/)
assert.match(validationImplementation, /MOVE_HOME/)
assert.match(validationImplementation, /sui\.keystore/)
assert.match(validationImplementation, /envs: \[\]/)
assert.match(validationImplementation, /active_env: null/)
assert.match(validationImplementation, /--client\.config/)
assert.match(validationImplementation, /--build-env", "testnet"/)
assert.match(validationImplementation, /isolated no-account configuration changed/)
assert.match(validationImplementation, /--warnings-are-errors/)
assert.match(validationImplementation, /--threads", "1"/)
assert.match(validationImplementation, /Assert-FixedFile -Path \$frameworkArchive/)
assert.match(validationImplementation, /\$contract\.framework\.archive_size/)
assert.match(validationImplementation, /\$contract\.framework\.archive_sha256/)
assert.match(validationImplementation, /tracked_content_digest/)
assert.match(validationImplementation, /Expand-FixedFrameworkArchive/)
assert.match(validationImplementation, /Get-TrackedContentDigest/)
assert.match(validationImplementation, /Assert-IsolatedMoveHome/)
assert.match(validationImplementation, /Resolve-ContainedRelativePath/)
assert.match(validationImplementation, /IsPathRooted/)
assert.match(validationImplementation, /contains an invalid path segment/)
assert.match(validationImplementation, /MoveHome must be absent or empty/)
assert.match(validationImplementation, /data other than inert lock files/)
assert.match(toolchainImplementation, /Resolve-TrustedSystemTar|Get-TrustedSystemTar/)
assert.match(toolchainImplementation, /GetFolderPath/)
assert.match(toolchainImplementation, /Get-AuthenticodeSignature/)
assert.match(toolchainImplementation, /O=Microsoft Corporation/)
assert.match(validationImplementation, /Test-ApprovedOutputLine/)
assert.match(validationImplementation, /default \{ return \$false \}/)
assert.match(validationImplementation, /ConvertTo-ApprovedOutput/)
assert.match(validationImplementation, /Write-SafeCommandFailure/)
assert.match(validationImplementation, /COMMAND_OUTPUT=not_persisted/)
assert.match(validationImplementation, /Write-Utf8Lf -Path \$OutputPath -Lines \$approved/)
assert.match(validationImplementation, /raw child output was not persisted/)
assert.doesNotMatch(validationImplementation,
  /\$tail\b|Select-Object\s+-Last|stdout=|stderr=|Write-Utf8Lf\s+-Path \$OutputPath\s+-Lines \$rawLines/)
for (const forbiddenGitOperation of [
  /\bAssert-MoveDependencyCache\b/,
  /\bInvoke-ReadOnlyGit\b/,
  /Get-Command\s+git\b/i,
  /&\s+git(?:\.exe)?\b/i,
  /"remote"\s*,\s*"get-url"/,
  /"status"\s*,\s*"--porcelain/,
  /"ls-files"/,
  /"rev-parse"/,
]) {
  assert.doesNotMatch(toolchainImplementation, forbiddenGitOperation)
}
assert.match(validationImplementation, /ESK_SUI_PUBLICATION_STATE=not_performed/)

const fixedEvidenceNames = [...validationImplementation.matchAll(
  /Join-Path\s+\$evidenceRoot\s+"([^"$]+\.log)"/g,
)].map((match) => match[1]).sort()
assert.deepEqual(fixedEvidenceNames, [
  'artifact-verification.log',
  'test-esk-sui-allocation-vesting.log',
  'test-esk-sui-genesis-foundation.log',
  'validation-status.log',
].sort())
assert.match(validationImplementation,
  /Join-Path\s+\$evidenceRoot\s+"\$\(\$package\.id\)-build\.log"/)
assert.match(validationImplementation,
  /Join-Path\s+\$evidenceRoot\s+"\$\(\$package\.id\)-test\.log"/)

function extractWorkflowJob(source, jobName) {
  const lines = source.split('\n')
  const marker = `  ${jobName}:`
  const starts = lines.flatMap((line, index) => line === marker ? [index] : [])
  assert.equal(starts.length, 1, `workflow must contain exactly one ${jobName} job`)
  const start = starts[0]
  const relativeEnd = lines.slice(start + 1).findIndex((line) => /^  [A-Za-z0-9_-]+:\s*$/.test(line))
  const end = relativeEnd < 0 ? lines.length : start + 1 + relativeEnd
  return lines.slice(start, end).join('\n')
}

function extractWorkflowStep(job, marker) {
  const lines = job.split('\n')
  const starts = lines.flatMap((line, index) => /^      - /.test(line) ? [index] : [])
  const matching = starts.filter((start, position) => {
    const end = starts[position + 1] ?? lines.length
    return lines.slice(start, end).join('\n').includes(marker)
  })
  assert.equal(matching.length, 1, `sui-move must contain exactly one step matching ${marker}`)
  const start = matching[0]
  const next = starts.find((candidate) => candidate > start) ?? lines.length
  return lines.slice(start, next).join('\n')
}

function assertExactWorkflowStep(step, expectedLines) {
  assert.equal(step.trimEnd(), expectedLines.join('\n'))
}

function workflowStepIdentities(job) {
  const lines = job.split('\n')
  const starts = lines.flatMap((line, index) => /^      - /.test(line) ? [index] : [])
  return starts.map((start, position) => {
    const end = starts[position + 1] ?? lines.length
    const block = lines.slice(start, end).join('\n')
    const name = block.match(/^      - name: ([^\r\n]+)$/m)
    if (name) return `name:${name[1]}`
    const uses = block.match(/^      - uses: ([^\s#]+)(?:\s+#.*)?$/m)
    assert.ok(uses, `workflow step has neither a fixed name nor uses identity:\n${block}`)
    return `uses:${uses[1]}`
  })
}

assert.match(workflowLf, /^permissions:\n  contents: read$/m)
const suiJob = extractWorkflowJob(workflowLf, 'sui-move')
assert.match(suiJob, /^    name: ESK Sui Move$/m)
assert.match(suiJob, /^    runs-on: windows-2025$/m)
assert.match(suiJob, /^    timeout-minutes: 40$/m)
assert.doesNotMatch(suiJob, /secrets\.|Cache Fixed Move Dependencies|sui-move-deps-v1/i)
assert.doesNotMatch(suiJob, /^    permissions:/m,
  'sui-move must inherit the workflow-level contents: read permission')
assert.doesNotMatch(suiJob,
  /\bsui(?:\.exe)?\s+(?:client|keytool|genesis|start)\b|\bmove\s+publish\b/i)
assert.deepEqual(workflowStepIdentities(suiJob), [
  'name:Enable Git Long Paths',
  'uses:actions/checkout@11d5960a326750d5838078e36cf38b85af677262',
  'name:Set up Node',
  'name:Sui Publication Preflight Contract Guard',
  'name:Sui Toolchain Contract Guard',
  'name:Cache Fixed Sui Toolchain',
  'name:Install Fixed Sui Toolchain',
  'name:Verify ESK Move Packages',
  'name:Upload Sui Validation Evidence',
])
assert.deepEqual([...suiJob.matchAll(/^        uses: ([^\s#]+)(?:\s+#.*)?$/gm)]
  .map((match) => match[1]), [
  'actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020',
  'actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830',
  'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02',
])

const gitLongPathsStep = extractWorkflowStep(suiJob, 'Enable Git Long Paths')
assertExactWorkflowStep(gitLongPathsStep, [
  '      - name: Enable Git Long Paths',
  '        shell: pwsh',
  '        run: git config --global core.longpaths true',
])
const checkoutStep = extractWorkflowStep(suiJob,
  'actions/checkout@11d5960a326750d5838078e36cf38b85af677262')
assertExactWorkflowStep(checkoutStep, [
  '      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4',
  '        with:',
  '          fetch-depth: 0',
  '          persist-credentials: false',
])
const nodeStep = extractWorkflowStep(suiJob,
  'actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020')
assertExactWorkflowStep(nodeStep, [
  '      - name: Set up Node',
  '        uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4',
  '        with:',
  '          node-version: 22.12.0',
])
const guardStep = extractWorkflowStep(suiJob, 'Sui Toolchain Contract Guard')
assertExactWorkflowStep(guardStep, [
  '      - name: Sui Toolchain Contract Guard',
  '        run: node scripts/test-esk-sui-toolchain-ci.js',
])
const publicationPreflightGuardStep = extractWorkflowStep(
  suiJob,
  'Sui Publication Preflight Contract Guard',
)
assertExactWorkflowStep(publicationPreflightGuardStep, [
  '      - name: Sui Publication Preflight Contract Guard',
  '        run: node scripts/test-esk-sui-testnet-publication-preflight.js',
])
const cacheStep = extractWorkflowStep(suiJob, 'Cache Fixed Sui Toolchain')
assert.equal((suiJob.match(
  /^        uses: actions\/cache@0057852bfaa89a56745cba8c7296529d2fc39830 # v4$/gm,
) || []).length, 1)
assertExactWorkflowStep(cacheStep, [
  '      - name: Cache Fixed Sui Toolchain',
  '        uses: actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830 # v4',
  '        with:',
  '          path: ${{ runner.tool_cache }}\\yilong-sui\\testnet-v1.79.0\\windows-x86_64',
  '          key: sui-toolchain-v2-testnet-v1.79.0-windows-x86_64-9d8442bad8fd516116a76ff52213cdfd43b98857c3f7fbbde124ed5e2041a558-9046fce263794cca6772c59aacd328706d42522dd7408c0dcb4a7f2613016afd',
])
assert.doesNotMatch(cacheStep,
  /runner\.temp|restore-keys:|MOVE_HOME|move-home|\\git(?:\\|$)|\.move|\\sui\.exe(?:\n|$)/i,
  'CI must not cache a live Git dependency checkout')
const installStep = extractWorkflowStep(suiJob, 'Install Fixed Sui Toolchain')
assertExactWorkflowStep(installStep, [
  '      - name: Install Fixed Sui Toolchain',
  '        shell: pwsh',
  '        run: |',
  '          $installRoot = "${{ runner.tool_cache }}\\yilong-sui"',
  '          $frameworkArchivePath = Join-Path $installRoot "testnet-v1.79.0\\windows-x86_64\\framework-source.tar.gz"',
  '          scripts\\install-esk-sui-toolchain.ps1 -InstallRoot $installRoot -FrameworkArchivePath $frameworkArchivePath',
])
const verifyStep = extractWorkflowStep(suiJob, 'Verify ESK Move Packages')
assertExactWorkflowStep(verifyStep, [
  '      - name: Verify ESK Move Packages',
  '        shell: pwsh',
  '        run: |',
  '          $suiPath = "${{ runner.tool_cache }}\\yilong-sui\\testnet-v1.79.0\\windows-x86_64\\sui.exe"',
  '          $frameworkArchivePath = "${{ runner.tool_cache }}\\yilong-sui\\testnet-v1.79.0\\windows-x86_64\\framework-source.tar.gz"',
  '          $evidence = "${{ runner.temp }}\\esk-sui-move-evidence"',
  '          $moveHome = Join-Path "${{ runner.temp }}" "esk-sui-move-home-$([guid]::NewGuid().ToString(\'N\'))"',
  '          scripts\\validate-esk-sui-move.ps1 -SuiPath $suiPath -FrameworkArchivePath $frameworkArchivePath -EvidenceDirectory $evidence -MoveHome $moveHome',
])
const uploadStep = extractWorkflowStep(suiJob, 'Upload Sui Validation Evidence')
assertExactWorkflowStep(uploadStep, [
  '      - name: Upload Sui Validation Evidence',
  '        if: failure()',
  '        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4',
  '        with:',
  '          name: esk-sui-move-validation-${{ github.run_id }}',
  '          path: |',
  '            ${{ runner.temp }}\\esk-sui-move-evidence\\artifact-verification.log',
  '            ${{ runner.temp }}\\esk-sui-move-evidence\\esk_currency-build.log',
  '            ${{ runner.temp }}\\esk-sui-move-evidence\\esk_currency-test.log',
  '            ${{ runner.temp }}\\esk-sui-move-evidence\\test-esk-sui-allocation-vesting.log',
  '            ${{ runner.temp }}\\esk-sui-move-evidence\\test-esk-sui-genesis-foundation.log',
  '            ${{ runner.temp }}\\esk-sui-move-evidence\\validation-status.log',
  '            ${{ runner.temp }}\\esk-sui-move-evidence\\yilong_participation-build.log',
  '            ${{ runner.temp }}\\esk-sui-move-evidence\\yilong_participation-test.log',
  '          if-no-files-found: warn',
  '          retention-days: 7',
])
const uploadedEvidence = [...uploadStep.matchAll(
  /^            \$\{\{ runner\.temp \}\}\\esk-sui-move-evidence\\([^\r\n]+\.log)$/gm,
)].map((match) => match[1]).sort()
assert.deepEqual(uploadedEvidence, [
  'artifact-verification.log',
  'esk_currency-build.log',
  'esk_currency-test.log',
  'test-esk-sui-allocation-vesting.log',
  'test-esk-sui-genesis-foundation.log',
  'validation-status.log',
  'yilong_participation-build.log',
  'yilong_participation-test.log',
].sort())
assert.doesNotMatch(uploadStep,
  /^          path: \$\{\{ runner\.temp \}\}\\esk-sui-move-evidence\s*$/m)
assert.doesNotMatch(suiJob, /uses: actions\/(?:checkout|setup-node|cache|upload-artifact)@v4(?:\s|$)/)

assert.equal(
  verifier.canonicalTestReceipt('\u001b[32mINCLUDING DEPENDENCY Sui\u001b[0m\r\nRunning Move unit tests\r\nTest result: OK. Total tests: 1; passed: 1; failed: 0\r\n'),
  'INCLUDING DEPENDENCY Sui\nRunning Move unit tests\nTest result: OK. Total tests: 1; passed: 1; failed: 0\n',
)
assert.equal(
  verifier.canonicalTestReceipt('[NOTE] Dependencies on Sui, MoveStdlib, Bridge, DeepBook, and SuiSystem are automatically added, but this feature is disabled for your package because you have explicitly included dependencies on Sui. Consider removing these dependencies from `Move.toml`.\nRunning Move unit tests\nTest result: OK. Total tests: 1; passed: 1; failed: 0\n'),
  'Running Move unit tests\nTest result: OK. Total tests: 1; passed: 1; failed: 0\n',
)
assert.throws(() => verifier.canonicalTestReceipt('[NOTE] unknown note\n'), /unexpected Move test output/)
assert.throws(() => verifier.canonicalTestReceipt('unexpected output\n'), /unexpected Move test output/)

const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'esk-sui-ci-test-'))
try {
  const first = path.join(temporary, 'a.mv')
  const second = path.join(temporary, 'b.mv')
  fs.writeFileSync(first, Buffer.from([1, 2, 3]))
  fs.writeFileSync(second, Buffer.from([4, 5]))
  const expected = crypto.createHash('sha256')
    .update('a').update('\0').update(Buffer.from([1, 2, 3])).update('\0')
    .update('b').update('\0').update(Buffer.from([4, 5])).update('\0')
    .digest('hex')
  assert.equal(verifier.productionBundleDigest([second, first]), `sha256:${expected}`)
  assert.equal(verifier.buildEvidenceDigest('single_module_sha256_v1', [first]),
    sha256(Buffer.from([1, 2, 3])))
  assert.throws(
    () => verifier.buildEvidenceDigest('single_module_sha256_v1', [first, second]),
    /exactly one module/,
  )
  assert.throws(
    () => verifier.buildEvidenceDigest('production_bytecode_bundle_v1', []),
    /at least one module/,
  )
  assert.throws(
    () => verifier.buildEvidenceDigest('unknown_evidence_v1', [first]),
    /unsupported build evidence kind/,
  )
  assert.equal(verifier.resolveContainedRelativePath(temporary, 'a.mv', 'fixture'), first)
  for (const unsafePath of ['../a.mv', './a.mv', 'a//b.mv', 'a\\b.mv', '/a.mv', 'C:/a.mv']) {
    assert.throws(
      () => verifier.resolveContainedRelativePath(temporary, unsafePath, 'fixture'),
      /relative path|invalid path segment|approved parent/,
    )
  }
} finally {
  fs.rmSync(temporary, { recursive: true, force: true })
}

function runProcess(command, args) {
  return childProcess.spawnSync(command, args, {
    encoding: 'utf8',
    windowsHide: true,
    timeout: 60_000,
  })
}

function expectProcess(result, success, label, expectedOutput) {
  assert.equal(result.error, undefined, `${label} could not start: ${result.error?.message}`)
  assert.equal(result.status === 0, success,
    `${label} exit=${result.status}\nstdout=${result.stdout}\nstderr=${result.stderr}`)
  assert.match(`${result.stdout}\n${result.stderr}`, expectedOutput, `${label} returned unexpected output`)
}

const syntheticCommit = '0000000000000000000000000000000000000001'
const syntheticFrameworkRoot = `sui-${syntheticCommit}`
const syntheticTrackedRoots = [
  'crates/sui-framework/packages/move-stdlib',
  'crates/sui-framework/packages/sui-framework',
]

function trackedContentDigest(entries) {
  const ordered = [...entries].sort((left, right) => {
    if (left.relative < right.relative) return -1
    if (left.relative > right.relative) return 1
    return 0
  })
  const digest = crypto.createHash('sha256')
  for (const entry of ordered) {
    digest.update(Buffer.from(entry.relative, 'utf8'))
    digest.update(Buffer.from([0]))
    digest.update(entry.bytes)
    digest.update(Buffer.from([0]))
  }
  return `sha256:${digest.digest('hex')}`
}

function writeSyntheticContract(
  directory,
  archive,
  binary,
  version,
  frameworkArchive,
  frameworkEntries,
  overrides = {},
) {
  const synthetic = {
    schema: 'yilong.esk.sui.reproducible_toolchain.v1',
    sui_source_commit: syntheticCommit,
    framework: {
      archive_url: `https://codeload.github.com/MystenLabs/sui/tar.gz/${syntheticCommit}`,
      archive_size: fs.statSync(frameworkArchive).size,
      archive_sha256: sha256(fs.readFileSync(frameworkArchive)),
      archive_root: syntheticFrameworkRoot,
      tracked_roots: syntheticTrackedRoots,
      tracked_file_count: frameworkEntries.length,
      tracked_content_digest: trackedContentDigest(frameworkEntries),
    },
    cli: {
      release: 'synthetic-v1',
      platform: 'windows-x86_64',
      asset_name: path.basename(archive),
      asset_url: 'https://github.com/MystenLabs/sui/releases/download/synthetic-v1/synthetic.tgz',
      archive_size: fs.statSync(archive).size,
      archive_sha256: sha256(fs.readFileSync(archive)),
      binary_size: fs.statSync(binary).size,
      binary_sha256: sha256(fs.readFileSync(binary)),
      version,
      source_commit: syntheticCommit,
      ...overrides,
    },
    packages: [],
  }
  fs.writeFileSync(path.join(directory, 'esk-sui-toolchain-ci', 'toolchain-v1.json'),
    `${JSON.stringify(synthetic, null, 2)}\n`)
}

function flipFirstByte(file) {
  const handle = fs.openSync(file, 'r+')
  try {
    const byte = Buffer.alloc(1)
    assert.equal(fs.readSync(handle, byte, 0, 1, 0), 1, `cannot read byte from ${file}`)
    byte[0] ^= 0xff
    assert.equal(fs.writeSync(handle, byte, 0, 1, 0), 1, `cannot write byte to ${file}`)
  } finally {
    fs.closeSync(handle)
  }
}

function runSyntheticInstaller(installerCopy, installRoot, archivePath, frameworkArchivePath) {
  const args = ['-NoProfile', '-File', installerCopy, '-InstallRoot', installRoot]
  if (archivePath) args.push('-ArchivePath', archivePath)
  args.push('-FrameworkArchivePath', frameworkArchivePath)
  return runProcess('pwsh.exe', args)
}

function runSyntheticInstallerTests() {
  if (process.platform !== 'win32') return 'SKIP synthetic installer execution (Windows x64 only)'
  const systemRoot = process.env.SystemRoot || process.env.WINDIR
  assert.ok(systemRoot, 'Windows system root is required')
  const tar = path.join(systemRoot, 'System32', 'tar.exe')
  assert.ok(fs.existsSync(tar), 'Windows tar.exe is required')
  const testRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'esk-sui-installer-test-'))
  try {
    const harness = path.join(testRoot, 'harness')
    const contractDirectory = path.join(harness, 'esk-sui-toolchain-ci')
    const payload = path.join(testRoot, 'cli-payload')
    const frameworkPayload = path.join(testRoot, 'framework-payload')
    fs.mkdirSync(contractDirectory, { recursive: true })
    fs.mkdirSync(payload, { recursive: true })
    fs.mkdirSync(frameworkPayload, { recursive: true })
    const installerCopy = path.join(harness, 'install-esk-sui-toolchain.ps1')
    fs.copyFileSync(fromRoot(installerPath), installerCopy)
    const syntheticBinary = path.join(payload, 'sui.exe')
    fs.copyFileSync(tar, syntheticBinary)
    const versionProbe = runProcess(syntheticBinary, ['--version'])
    expectProcess(versionProbe, true, 'synthetic binary version probe', /bsdtar/i)
    const syntheticVersion = versionProbe.stdout.trim()

    const archive = path.join(testRoot, 'synthetic.tgz')
    const archiveResult = runProcess(tar, ['-czf', archive, '-C', payload, './sui.exe'])
    expectProcess(archiveResult, true, 'synthetic archive creation', /^\s*$/)

    const frameworkEntries = [
      {
        relative: `${syntheticTrackedRoots[0]}/Move.toml`,
        bytes: Buffer.from('[package]\nname = "MoveStdlib"\n', 'utf8'),
      },
      {
        relative: `${syntheticTrackedRoots[1]}/Move.toml`,
        bytes: Buffer.from('[package]\nname = "Sui"\n', 'utf8'),
      },
    ]
    for (const entry of frameworkEntries) {
      const destination = path.join(frameworkPayload, syntheticFrameworkRoot,
        ...entry.relative.split('/'))
      fs.mkdirSync(path.dirname(destination), { recursive: true })
      fs.writeFileSync(destination, entry.bytes)
    }
    const frameworkArchive = path.join(testRoot, 'synthetic-framework.tar.gz')
    expectProcess(runProcess(tar, [
      '-czf', frameworkArchive, '-C', frameworkPayload, `./${syntheticFrameworkRoot}`,
    ]), true, 'synthetic Framework archive creation', /^\s*$/)
    writeSyntheticContract(harness, archive, syntheticBinary, syntheticVersion,
      frameworkArchive, frameworkEntries)

    const installRoot = path.join(testRoot, 'install-valid')
    let result = runSyntheticInstaller(installerCopy, installRoot, archive, frameworkArchive)
    expectProcess(result, true, 'fresh synthetic installation', /source=official_archive/)
    const installedDirectory = path.join(installRoot, 'synthetic-v1', 'windows-x86_64')
    const cachedBinary = path.join(installedDirectory, 'sui.exe')
    const cachedFramework = path.join(installedDirectory, 'framework-source.tar.gz')
    assert.deepEqual(fs.readdirSync(installedDirectory).sort(), [
      'framework-source.tar.gz',
      'sui.exe',
    ])
    assert.equal(sha256(fs.readFileSync(cachedFramework)), sha256(fs.readFileSync(frameworkArchive)))

    result = runSyntheticInstaller(installerCopy, installRoot, undefined, cachedFramework)
    expectProcess(result, true, 'verified synthetic cache hit', /source=cache/)

    flipFirstByte(cachedBinary)
    result = runSyntheticInstaller(installerCopy, installRoot, undefined, cachedFramework)
    expectProcess(result, false, 'same-length cached CLI corruption rejection',
      /cached CLI digest mismatch/)
    fs.copyFileSync(syntheticBinary, cachedBinary)

    flipFirstByte(cachedFramework)
    result = runSyntheticInstaller(installerCopy, installRoot, undefined, cachedFramework)
    expectProcess(result, false, 'same-length cached Framework corruption rejection',
      /cached Framework source archive digest mismatch/)

    const corruptCliDigestArchive = path.join(testRoot, 'corrupt-cli-digest.tgz')
    fs.copyFileSync(archive, corruptCliDigestArchive)
    flipFirstByte(corruptCliDigestArchive)
    result = runSyntheticInstaller(installerCopy, path.join(testRoot, 'install-cli-digest'),
      corruptCliDigestArchive, frameworkArchive)
    expectProcess(result, false, 'same-length CLI archive corruption rejection',
      /official release archive digest mismatch/)

    const corruptFrameworkDigestArchive = path.join(testRoot, 'corrupt-framework-digest.tar.gz')
    fs.copyFileSync(frameworkArchive, corruptFrameworkDigestArchive)
    flipFirstByte(corruptFrameworkDigestArchive)
    result = runSyntheticInstaller(installerCopy, path.join(testRoot, 'install-framework-digest'),
      archive, corruptFrameworkDigestArchive)
    expectProcess(result, false, 'same-length Framework archive corruption rejection',
      /official Framework source archive digest mismatch/)

    const corruptCliLengthArchive = path.join(testRoot, 'corrupt-cli-length.tgz')
    fs.copyFileSync(archive, corruptCliLengthArchive)
    fs.appendFileSync(corruptCliLengthArchive, Buffer.from([0]))
    result = runSyntheticInstaller(installerCopy, path.join(testRoot, 'install-cli-length'),
      corruptCliLengthArchive, frameworkArchive)
    expectProcess(result, false, 'CLI archive length rejection',
      /official release archive length mismatch/)

    const corruptFrameworkLengthArchive = path.join(testRoot, 'corrupt-framework-length.tar.gz')
    fs.copyFileSync(frameworkArchive, corruptFrameworkLengthArchive)
    fs.appendFileSync(corruptFrameworkLengthArchive, Buffer.from([0]))
    result = runSyntheticInstaller(installerCopy, path.join(testRoot, 'install-framework-length'),
      archive, corruptFrameworkLengthArchive)
    expectProcess(result, false, 'Framework archive length rejection',
      /official Framework source archive length mismatch/)

    writeSyntheticContract(harness, archive, syntheticBinary, `${syntheticVersion}-wrong`,
      frameworkArchive, frameworkEntries)
    result = runSyntheticInstaller(installerCopy, path.join(testRoot, 'install-version'),
      archive, frameworkArchive)
    expectProcess(result, false, 'wrong version rejection', /extracted CLI version mismatch/)

    const nestedPayload = path.join(testRoot, 'nested-payload')
    fs.mkdirSync(path.join(nestedPayload, 'nested'), { recursive: true })
    fs.copyFileSync(syntheticBinary, path.join(nestedPayload, 'nested', 'sui.exe'))
    const nestedArchive = path.join(testRoot, 'nested.tgz')
    expectProcess(runProcess(tar, ['-czf', nestedArchive, '-C', nestedPayload, './nested/sui.exe']),
      true, 'nested archive creation', /^\s*$/)
    writeSyntheticContract(harness, nestedArchive, syntheticBinary, syntheticVersion,
      frameworkArchive, frameworkEntries)
    result = runSyntheticInstaller(installerCopy, path.join(testRoot, 'install-nested'),
      nestedArchive, frameworkArchive)
    expectProcess(result, false, 'nested binary rejection', /fixed root entry/)

    const multiplePayload = path.join(testRoot, 'multiple-payload')
    fs.mkdirSync(path.join(multiplePayload, 'a'), { recursive: true })
    fs.mkdirSync(path.join(multiplePayload, 'b'), { recursive: true })
    fs.copyFileSync(syntheticBinary, path.join(multiplePayload, 'a', 'sui.exe'))
    fs.copyFileSync(syntheticBinary, path.join(multiplePayload, 'b', 'sui.exe'))
    const multipleArchive = path.join(testRoot, 'multiple.tgz')
    expectProcess(runProcess(tar, ['-czf', multipleArchive, '-C', multiplePayload,
      './a/sui.exe', './b/sui.exe']), true, 'multiple archive creation', /^\s*$/)
    writeSyntheticContract(harness, multipleArchive, syntheticBinary, syntheticVersion,
      frameworkArchive, frameworkEntries)
    result = runSyntheticInstaller(installerCopy, path.join(testRoot, 'install-multiple'),
      multipleArchive, frameworkArchive)
    expectProcess(result, false, 'multiple binary rejection', /exactly one sui\.exe/)

    const junctionInstallRoot = path.join(testRoot, 'install-junction')
    const junctionTarget = path.join(testRoot, 'junction-target')
    fs.mkdirSync(junctionInstallRoot, { recursive: true })
    fs.mkdirSync(junctionTarget, { recursive: true })
    fs.symlinkSync(junctionTarget, path.join(junctionInstallRoot, 'synthetic-v1'), 'junction')
    writeSyntheticContract(harness, archive, syntheticBinary, syntheticVersion,
      frameworkArchive, frameworkEntries)
    result = runSyntheticInstaller(installerCopy, junctionInstallRoot, archive, frameworkArchive)
    expectProcess(result, false, 'junction parent rejection', /cannot be a reparse point/)
    assert.equal(fs.existsSync(path.join(junctionTarget, 'windows-x86_64')), false,
      'installer must reject the junction before writing through it')
  } finally {
    fs.rmSync(testRoot, { recursive: true, force: true })
  }
  return 'PASS synthetic dual-artifact installer fresh/cache/digest/length/version/layout cases'
}

console.log(`PASS fixed Sui toolchain contract ${config.cli.release} ${config.cli.platform}`)
console.log('PASS installer and validator preserve no-wallet/no-RPC/no-publication boundary')
console.log('PASS CI job and deterministic artifact helpers are contract-bound')
console.log(runSyntheticInstallerTests())
