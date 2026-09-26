const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const Module = require('node:module')
const ts = require('typescript')
function load(relative, stubs = {}) {
  const filename = path.resolve(__dirname, '../src', relative)
  const compiled = new Module(filename, module)
  compiled.filename = filename
  compiled.paths = Module._nodeModulePaths(path.dirname(filename))
  const original = compiled.require.bind(compiled)
  compiled.require = name => stubs[name] || original(name)
  compiled._compile(ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 }, fileName: filename,
  }).outputText, filename)
  return compiled.exports
}
const { uploadPrivateAttachments, privateUploadDiagnostic } = load('features/user-browser/privateAttachmentUpload.ts')
const { groupAttachmentFiles } = load('features/friends/group-ai/groupAiAttachments.ts', {
  '../../../api/runtime': { resolveApiUrl: path => 'https://platform.invalid' + path },
})
const manifest = { name: 'group_01_file.txt', mime_type: 'text/plain', size_bytes: 3,
  download_path: '/api/user/u/chat-attachments/g/file.txt', message_id: 'm', attachment_id: 'f' }

test('frontend chunks flow through the production Win byte bridge and shared native reader', async () => {
  const factory = require('../../desktop-shell/src-tauri/src/local_ai_browser/win_attachment_source.js')
  const nativeSource = require('../../android/app/src/main/assets/chatgpt_web_native_attachment_source.js')
  const receipts = [], uploaded = []
  const root = { location: { href: 'https://chatgpt.com/?temporary-chat=true' },
    __elonChatGptDocumentToken: 'doc-fixture', __elonChatGptAdapterVersion: 209,
    __elonChatGptPrivateAttachmentProtocol: require('../../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js'),
    File, Blob, atob, btoa, setTimeout, clearTimeout,
    elonChatGptNative: { postMessage: raw => receipts.push(JSON.parse(raw)) } }
  const bridge = factory(root), reader = nativeSource.create(root)
  root.__elonChatGptPrivateAttachmentSend = { cancel() {}, async start(raw, respond) {
    for (const descriptor of JSON.parse(raw).files) {
      const file = await reader.read(descriptor)
      uploaded.push(Buffer.from(await file.arrayBuffer()))
    }
    respond('request_attachment_upload', true, 'private_attachment_associated')
  } }
  const bytes = Buffer.alloc(70003, 37)
  await uploadPrivateAttachments([{ name: 'fixture.txt', type: 'text/plain', size: bytes.length,
    load: async () => new Blob([bytes]) }], {
    check() {},
    command: (value, requestId) => bridge.command(JSON.stringify({ value, requestId })),
    state: async () => ({ commandResults: receipts }),
  })
  assert.deepEqual(uploaded, [bytes])
  assert.equal(bridge.guardSend('{"action":"send_prompt"}'), false)
})

test('a changed file cancels the batch without requesting a private upload', async () => {
  const calls = [], receipts = []
  await assert.rejects(uploadPrivateAttachments([{ name: 'file.txt', type: 'text/plain', size: 3,
    load: async () => new Blob(['changed']) }], {
    check() {},
    command: async (raw, requestId) => {
      calls.push(JSON.parse(raw).step)
      receipts.push({ requestId, action: 'stage_attachments', ok: true })
    },
    state: async () => ({ commandResults: receipts }),
  }), /附件已变化/)
  assert.deepEqual(calls, ['begin', 'cancel'])
})

test('upload diagnostics identify the failed step without exporting provider details', async () => {
  for (const step of ['begin', 'read', 'chunk', 'upload']) {
    const receipts = [], calls = []
    await assert.rejects(uploadPrivateAttachments([{ name: 'fixture.txt', type: 'text/plain', size: 3,
      load: async () => { if (step === 'read') throw Error('network unavailable'); return new Blob(['abc']) } }], {
      check() {},
      async command(raw, requestId) {
        const value = JSON.parse(raw); calls.push(value.step)
        if (value.step === 'chunk' && step === 'chunk') throw Error('bridge unavailable')
        receipts.push({ requestId, action: value.step === 'upload' ? 'request_attachment_upload' : 'stage_attachments',
          ok: value.step !== step, detail: value.step === step ? 'provider private detail' : 'private_attachment_associated' })
      },
      state: async () => ({ commandResults: receipts }),
    }), error => {
      assert.equal(error.code, `private_upload_${step === 'upload' ? 'associate' : step}_${['read', 'chunk'].includes(step) ? 'failed' : 'rejected'}`)
      assert.doesNotMatch(error.message, /provider private detail/)
      return true
    })
    assert.equal(calls.at(-1), 'cancel')
  }
})

test('only reviewed Rspack diagnostic details survive the MCP boundary', async () => {
  for (const detail of ['private_upload_rspack_owner_pending', 'private_upload_rspack_private_content',
    'private_upload_rspack_upload_cancelled', 'private_upload_rspack_upload_context_changed',
    'private_upload_rspack_upload_association_unconfirmed', 'private_upload_rspack_upload_transaction_failed']) {
    const receipts = []
    await assert.rejects(uploadPrivateAttachments([{ name: 'fixture.txt', type: 'text/plain', size: 3,
      load: async () => new Blob(['abc']) }], {
      check() {},
      async command(raw, requestId) {
        const step = JSON.parse(raw).step
        receipts.push({ requestId, action: step === 'upload' ? 'request_attachment_upload' : 'stage_attachments',
          ok: step !== 'upload', detail })
      },
      state: async () => ({ commandResults: receipts }),
    }), error => {
      assert.equal(error.code, privateUploadDiagnostic.test(detail) ? detail : 'private_upload_associate_rejected')
      return true
    })
  }
})

test('group files use only platform URLs, omit credentials and reject incomplete bodies', async () => {
  const previous = global.fetch
  const requests = []
  try {
    global.fetch = async (url, options) => {
      requests.push([url, options]); return new Response('abc')
    }
    assert.equal(await (await groupAttachmentFiles([manifest])[0].load()).text(), 'abc')
    assert.equal(requests[0][0], 'https://platform.invalid' + manifest.download_path)
    assert.equal(requests[0][1].credentials, 'omit')
    assert.equal(requests[0][1].redirect, 'error')
    assert.equal(requests[0][1].headers, undefined)
    await assert.rejects(groupAttachmentFiles([{ ...manifest, size_bytes: 4 }])[0].load(), /未完整下载/)
    await assert.rejects(groupAttachmentFiles([{ ...manifest, size_bytes: 2 }])[0].load(), /大小已变化/)
    for (const download_path of ['https://other.invalid/file', '/api/user/u/chat-attachments/g/../secret',
      '/api/user/u/chat-attachments/g/%252fsecret', '/api/user/u/chat-attachments/g/file?token=secret'])
      assert.throws(() => groupAttachmentFiles([{ ...manifest, download_path }]), /地址无效/)
  } finally { global.fetch = previous }
})
