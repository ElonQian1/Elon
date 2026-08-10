const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const modelPath = path.join(
  __dirname,
  '..',
  'src',
  'features',
  'project-docs',
  'projectDocumentNativeContextModel.ts',
)
const output = ts.transpileModule(fs.readFileSync(modelPath, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const responses = []
const calls = []
const loaded = { exports: {} }
new Function('module', 'exports', 'require', output)(
  loaded,
  loaded.exports,
  (request) => {
    if (request === '../node/localNodeApi') {
      return {
        nodeApi: async (adminUrl, apiPath, options) => {
          calls.push({ adminUrl, apiPath, options, body: JSON.parse(options.body) })
          assert(responses.length, `missing response for ${apiPath}`)
          return responses.shift()
        },
      }
    }
    if (request === './projectDocumentNativeContextHealthModel') return {}
    return require(request)
  },
)

const model = loaded.exports
const hash = 'a'.repeat(64)

function candidate(overrides = {}) {
  return {
    candidate_id: 'native-candidate-1',
    summary: 'The project document route is owned by the verified native context module.',
    topics: ['project memory', 'project memory'],
    evidence: [{
      path: 'server\\src\\project_document_native_context.rs',
      content_hash: hash,
      locator: 'record_candidate',
      evidence_kind: 'source',
    }],
    reviewed_at: '',
    owner: '',
    scope: { kind: 'repository', paths: [], scope_ids: [], branches: [], releases: [], worktree_state: 'any' },
    review: { reviewed_on: '', reviewed_by: '', review_interval_days: 0, expires_at: '' },
    status: 'pending',
    producer: 'codex_native_tools',
    created_at_ms: 1,
    updated_at_ms: 2,
    evidence_current: true,
    ingest_action: 'created',
    provenance: {
      source: 'receipt_profile', assurance: 'local_mcp_session_attested',
      session_fingerprint: 'abcdef', evidence_path_count: 1, recorded_at_ms: 1,
      last_editor: '', last_edited_at_ms: 0,
    },
    conflicts: [],
    review_feedback: { decision: '', reason: '', decided_at: '', decided_by: '' },
    ...overrides,
  }
}

async function main() {
  const portable = model.sanitizeProjectContextMemories([
    candidate(),
    candidate({ summary: 'The later duplicate replaces the earlier candidate in the bounded view.' }),
    candidate({
      candidate_id: 'native-invalid',
      evidence: [{ path: '../secret.txt', content_hash: hash, evidence_kind: 'source' }],
    }),
  ])
  assert.equal(portable.length, 1)
  assert.equal(portable[0].topics.length, 1)
  assert.match(portable[0].summary, /later duplicate/)
  assert.equal(portable[0].evidence[0].path, 'server/src/project_document_native_context.rs')

  responses.push({
    ok: true,
    result: {
      status: 'unknown',
      counts: { pending: 2.9, reviewed: -1, rejected: 1, applied: 0 },
      pagination: { offset: 0, limit: 5, total: 2, next_offset: 5 },
      candidates: [candidate(), candidate({
        candidate_id: 'native-unsafe',
        evidence: [{ path: '../outside.rs', content_hash: hash, evidence_kind: 'source' }],
      })],
      producer_quality: {
        schema: 'elon.native_context_producer_quality.v1',
        producers: { codex_native_tools: { pending: 2 } },
        interpretation: 'Descriptive only.',
      },
    },
  })
  const page = await model.listNativeContextCandidates({
    adminUrl: 'http://127.0.0.1:7799',
    projectRoot: 'D:\\project',
    status: 'pending',
    offset: 0,
    limit: 5,
  })
  assert.equal(page.status, 'pending')
  assert.equal(page.candidates.length, 1)
  assert.equal(page.pagination.returned, 1)
  assert.equal(page.pagination.total, 2)
  assert.equal(page.counts.pending, 2)
  assert.equal(page.counts.reviewed, 0)
  assert.equal(calls[0].apiPath, '/api/project-docs/native-context/candidates')
  assert.deepEqual(calls[0].body, {
    project_root: 'D:\\project', status: 'pending', offset: 0, limit: 5,
  })

  responses.push({ ok: true, result: { action: 'reject', repository_changed: false } })
  await model.reviewNativeContextCandidates({
    adminUrl: 'http://127.0.0.1:7799',
    projectRoot: 'D:\\project',
    candidateIds: ['native-candidate-1'],
    action: 'reject',
    authorizationMode: 'suggestions_only',
    catalogRevision: 'catalog-1',
    suggestionsRevision: 'suggestions-1',
    reviewReason: 'task_local',
  })
  assert.equal(calls[1].apiPath, '/api/project-docs/native-context/review')
  assert.deepEqual(calls[1].body, {
    project_root: 'D:\\project',
    candidate_ids: ['native-candidate-1'],
    action: 'reject',
    authorization_mode: 'suggestions_only',
    expected_catalog_revision: 'catalog-1',
    expected_suggestions_revision: 'suggestions-1',
    review_reason: 'task_local',
  })

  responses.push({ ok: true, result: { candidate: candidate({ status: 'rejected' }) } })
  const revised = await model.reviseNativeContextCandidate({
    adminUrl: 'http://127.0.0.1:7799',
    projectRoot: 'D:\\project',
    candidateId: 'native-candidate-1',
    expectedUpdatedAtMs: 2,
    summary: 'A human reviewed summary remains bound to the original evidence.',
    topics: ['project memory'],
  })
  assert.equal(revised.status, 'rejected')
  assert.equal(calls[2].apiPath, '/api/project-docs/native-context/revise')
  assert.equal(calls[2].body.expected_updated_at_ms, 2)

  responses.push({ ok: true, result: { candidate: candidate({ candidate_id: 'native-repair' }) } })
  const repair = await model.createNativeContextRelocationRepair({
    adminUrl: 'http://127.0.0.1:7799',
    projectRoot: 'D:\\project',
    candidateId: 'native-candidate-1',
    sourcePath: 'server/src/old.rs',
    replacementPath: 'server/src/new.rs',
  })
  assert.equal(repair.candidate_id, 'native-repair')
  assert.equal(calls[3].apiPath, '/api/project-docs/native-context/repair-relocation')
  assert.equal(calls[3].body.producer, 'pc_memory_repair')

  responses.push({ ok: false, error: 'candidate page unavailable' })
  await assert.rejects(
    model.listNativeContextCandidates({
      adminUrl: 'http://127.0.0.1:7799', projectRoot: 'D:\\project',
      status: 'pending', offset: 0,
    }),
    /candidate page unavailable/,
  )

  const inboxPath = path.join(
    __dirname,
    '..',
    'src',
    'features',
    'project-docs',
    'ProjectDocumentNativeContextInbox.tsx',
  )
  const inbox = fs.readFileSync(inboxPath, 'utf8')
  assert(!inbox.includes('/api/project-docs/native-context'), 'UI component must use the bounded model client')
  assert(inbox.includes("selected.has(candidate.candidate_id) && !candidate.evidence_current"))
  assert(inbox.includes("disabled={!selected.size || !selectedEvidenceCurrent || !canEdit || !catalogRevision || !!action}"))
  assert(inbox.includes('漂移项只能拒绝'))
  assert(inbox.includes('ProjectDocumentNativeContextEditor'))
  assert(inbox.includes("void runAction('restore')"))

  assert.equal(responses.length, 0)
  console.log('project document native context model tests passed')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
