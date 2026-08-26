const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const privateStreamPolicy = require(path.resolve(
  __dirname,
  '../../android/app/src/main/assets/chatgpt_web_private_stream_policy.js',
))

const source = fs.readFileSync(path.resolve(
  __dirname,
  '../../desktop-shell/src-tauri/src/local_ai_browser/win_web_response_research_capture.js',
), 'utf8').replace('__PROVIDER_ID__', 'chatgpt')

class FakeXhr {}
FakeXhr.prototype.open = function () {}
FakeXhr.prototype.send = function () {}

async function main() {
  const captures = []
  const recovered = []
  const recoveryGeneration = 7
  let fetchCalls = 0
  const richStream = [
    'data: ' + JSON.stringify({
      conversation_id: 'conversation-one',
      message: {
        id: 'assistant-one',
        author: { role: 'assistant' },
        status: 'finished_successfully',
        content: { content_type: 'text', parts: ['visible answer'] },
        metadata: {
          content_references: [{
            type: 'client_defined_widget',
            category: 'visualization',
            data: {
              language: 'recharts-json',
              widget_type: 'charts_widget_v2',
              content: {
                chartType: 'line',
                meta: { title: 'Bitcoin trend' },
                xKey: 'date',
                series: [{ dataKey: 'price', label: 'BTC/USD' }],
                data: [{ date: '8/24', price: 77000 }, { date: '8/25', price: 78805 }],
              },
            },
          }],
        },
      },
    }),
    'data: [DONE]',
    '',
  ].join('\n\n')
  const unsupportedRichStream = [
    'data: ' + JSON.stringify({
      conversation_id: 'conversation-one',
      message: {
        id: 'assistant-future-widget',
        author: { role: 'assistant' },
        status: 'finished_successfully',
        content: { content_type: 'text', parts: ['visible future answer'] },
        metadata: {
          content_references: [{
            type: 'client_defined_widget',
            category: 'visualization',
            data: {
              language: 'future-renderer',
              widget_type: 'future_widget_v9',
              content: { chartType: 'radar' },
            },
          }],
        },
      },
    }),
    'data: [DONE]',
    '',
  ].join('\n\n')
  const window = {
    __elonChatGptPrivateStreamPolicy: privateStreamPolicy,
    __elonWinChatGptPrivateStreamRecovery: {
      generation: () => recoveryGeneration,
      accept: (snapshot) => recovered.push(snapshot),
    },
    __TAURI_INTERNALS__: {
      invoke: async (command, args) => captures.push({ command, args }),
    },
    fetch: async (input) => {
      fetchCalls += 1
      const body = String(input).includes('backend-anon')
        ? 'data: {"type":"unknown_future_frame","payload":{"next":true}}\n\n'
        : String(input).includes('unsupported=1') ? unsupportedRichStream : richStream
      return new Response(body, {
        status: 200,
        headers: {
          'content-type': String(input).includes('mislabeled=1')
            ? 'application/json'
            : 'text/event-stream',
        },
      })
    },
    XMLHttpRequest: FakeXhr,
  }
  const context = {
    window,
    XMLHttpRequest: FakeXhr,
    location: {
      origin: 'https://chatgpt.com',
      href: 'https://chatgpt.com/c/conversation-one',
      pathname: '/c/conversation-one',
    },
    URL,
    Request,
    Response,
    TextEncoder,
    TextDecoder,
    Uint8Array,
    Set,
    WeakMap,
    Promise,
  }
  vm.runInNewContext(source, context, { filename: 'win_web_response_research_capture.js' })

  await window.fetch('https://chatgpt.com/backend-api/f/conversation', {
    method: 'POST',
    body: 'request-secret-must-not-be-captured',
  })
  await new Promise((resolve) => setTimeout(resolve, 20))
  assert.equal(fetchCalls, 1, 'capture must observe the original request without replaying it')
  assert.equal(captures.length, 1)
  assert.equal(captures[0].command, 'publish_local_ai_web_research_capture')
  assert.equal(captures[0].args.capture.endpointFamily, 'conversation_stream')
  assert.equal(captures[0].args.capture.format, 'sse')
  assert.match(captures[0].args.capture.body, /visible answer/)
  assert.equal(captures[0].args.capture.analysis.schema, 'yilong.web-ai.capture-analysis.v1')
  assert.equal(captures[0].args.capture.analysis.analyzerVersion, 2)
  assert.equal(captures[0].args.capture.analysis.policyAvailable, true)
  assert.equal(captures[0].args.capture.analysis.acceptedFrameCount, 1)
  assert.equal(captures[0].args.capture.analysis.assistantFrameCount, 1)
  assert.equal(captures[0].args.capture.analysis.textLength, 'visible answer'.length)
  assert.deepEqual(Array.from(captures[0].args.capture.analysis.richKinds), ['chart'])
  assert.equal(captures[0].args.capture.analysis.unsupportedRichCount, 0)
  assert.equal(recovered.length, 1)
  assert.equal(recovered[0].messageId, 'assistant-one')
  assert.equal(recovered[0].conversationId, 'conversation-one')
  assert.equal(recovered[0].text, 'visible answer')
  assert.equal(recovered[0].generation, recoveryGeneration)
  assert.equal(recovered[0].richParts[0].richContent.source, 'private_response')
  assert.doesNotMatch(JSON.stringify(captures[0]), /request-secret/)

  await window.fetch('https://chatgpt.com/backend-api/f/conversation?mislabeled=1', {
    method: 'POST',
  })
  await new Promise((resolve) => setTimeout(resolve, 20))
  assert.equal(captures[1].args.capture.format, 'sse')
  assert.deepEqual(Array.from(captures[1].args.capture.analysis.richKinds), ['chart'])
  assert.equal(recovered[1].generation, recoveryGeneration)

  await window.fetch('https://chatgpt.com/backend-api/f/conversation/stream', {
    method: 'POST',
  })
  await new Promise((resolve) => setTimeout(resolve, 20))
  assert.equal(captures.length, 3, 'versioned conversation stream paths must remain observable')
  assert.equal(captures[2].args.capture.endpointFamily, 'conversation_stream')

  await window.fetch('https://chatgpt.com/backend-anon/conversation', {
    method: 'POST',
  })
  await new Promise((resolve) => setTimeout(resolve, 20))
  assert.equal(captures.length, 4, 'guest conversation streams must enter the local research capture')
  assert.equal(captures[3].args.capture.endpointFamily, 'conversation_stream')
  assert.equal(captures[3].args.capture.analysis.decodedFrameCount, 1)
  assert.equal(captures[3].args.capture.analysis.acceptedFrameCount, 0)

  await window.fetch('https://chatgpt.com/backend-api/f/conversation?unsupported=1', {
    method: 'POST',
  })
  await new Promise((resolve) => setTimeout(resolve, 20))
  assert.equal(captures.length, 5)
  assert.equal(captures[4].args.capture.analysis.unsupportedRichCount, 1)
  assert.deepEqual(
    Array.from(captures[4].args.capture.analysis.richKinds),
    ['renderer_upgrade_required'],
  )
  assert.equal(recovered.at(-1).richParts[0].type, 'interactive')
  assert.equal(recovered.at(-1).richParts[0].kind, 'renderer_upgrade_required')

  await window.fetch('https://chatgpt.com/backend-api/accounts/check', { method: 'GET' })
  await new Promise((resolve) => setTimeout(resolve, 5))
  assert.equal(fetchCalls, 6)
  assert.equal(captures.length, 5, 'unregistered endpoint families must not enter local capture')
  console.log('Win Web response research capture tests passed')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
