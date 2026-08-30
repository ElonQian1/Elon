const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const source = fs.readFileSync(path.resolve(
  __dirname,
  '../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_managed_voice_peer.js',
), 'utf8')
const voiceHook = fs.readFileSync(path.resolve(
  __dirname,
  '../pc-frontend/src/features/user-browser/useLocalAiRealtimeVoiceControl.ts',
), 'utf8')
const rustCommands = fs.readFileSync(path.resolve(
  __dirname,
  '../desktop-shell/src-tauri/src/local_ai_browser/adapter_command.rs',
), 'utf8')

assert.doesNotMatch(source, /localStorage|sessionStorage|indexedDB|document\.cookie|Authorization/i)
assert.ok(
  voiceHook.indexOf("run('prepare_realtime_voice')") <
    voiceHook.indexOf("run('invoke_ui_control', controlId)"),
  'managed offer must arm before the official start control is invoked',
)
assert.match(voiceHook, /run\('control_managed_realtime_voice', action\)/)
assert.match(rustCommands, /"prepare_realtime_voice"/)
assert.match(rustCommands, /"control_managed_realtime_voice"/)

function harness({ microphoneDenied = false } = {}) {
  const posted = []
  const delegated = []
  const timers = new Map()
  const tracks = [{
    kind: 'audio', enabled: true, stopped: false,
    stop() { this.stopped = true },
  }]
  let nextTimer = 1
  let relayResult = null
  let armedOffer = ''
  let transcriptHookCount = 0
  let remoteAnswer = ''
  let activePeer = null

  class FakePeer {
    constructor() {
      this.listeners = new Map()
      this.connectionState = 'new'
      this.iceConnectionState = 'new'
      this.localDescription = null
      this.closed = false
      this.channel = null
      activePeer = this
    }
    addEventListener(type, listener) {
      const list = this.listeners.get(type) || []
      list.push(listener)
      this.listeners.set(type, list)
    }
    emit(type, event = {}) {
      for (const listener of this.listeners.get(type) || []) listener(event)
    }
    addTrack() {}
    createDataChannel(label, options) {
      this.channel = { label, options }
      return this.channel
    }
    async createOffer() {
      return { type: 'offer', sdp: 'v=0\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\n' }
    }
    async setLocalDescription(value) { this.localDescription = value }
    async setRemoteDescription(value) { remoteAnswer = value.sdp }
    close() { this.closed = true }
  }

  class FakeMediaStream {
    constructor(value = tracks) { this.value = value }
    getTracks() { return this.value }
    getAudioTracks() { return this.value.filter((track) => track.kind === 'audio') }
  }

  const relay = {
    version: 4,
    bootstrap() {
      return JSON.stringify({
        version: 4,
        available: true,
        dataChannel: {
          label: '', ordered: true, maxRetransmits: null,
          protocol: '', negotiated: false, id: null,
        },
      })
    },
    armExchange(id, offer) {
      assert.match(id, /^relay_[a-f0-9]{16}$/)
      armedOffer = offer
      return JSON.stringify({ version: 4, armed: true, code: null })
    },
    takeResult() { return relayResult },
    cancelExchange() { return true },
    resetTakeover() { return JSON.stringify({ version: 4, applied: true }) },
  }
  const document = {
    body: { appendChild() {} },
    documentElement: { appendChild() {} },
    createElement() {
      return {
        style: {}, srcObject: null, autoplay: false, playsInline: false,
        play: async () => {}, pause() {}, remove() {},
      }
    },
  }
  const context = {
    console,
    location: { origin: 'https://chatgpt.com', pathname: '/c/voice-one' },
    document,
    navigator: {
      mediaDevices: {
        async getUserMedia() {
          if (microphoneDenied) {
            const error = new Error('denied')
            error.name = 'NotAllowedError'
            throw error
          }
          return new FakeMediaStream()
        },
      },
    },
    MediaStream: FakeMediaStream,
    __elonWinChatGptManagedVoicePeerConstructor: FakePeer,
    __elonChatGptPrivateVoiceRelay: relay,
    __elonWinChatGptRealtimeVoiceTranscript: {
      hookPeer(peer) { transcriptHookCount += 1; return peer },
    },
    __elonChatGptAdapterVersion: 206,
    __elonChatGptDocumentToken: 'doc_win_voice_contract',
    elonChatGptNative: { postMessage(raw) { posted.push(JSON.parse(raw)) } },
    __elonChatGptBridge: {
      version: 206,
      command(raw) { delegated.push(JSON.parse(raw)) },
      dispose() {},
    },
    crypto: {
      getRandomValues(words) {
        words[0] = 0x12345678
        words[1] = 0x90abcdef
        return words
      },
    },
    setTimeout(callback, delay) {
      const id = nextTimer++
      timers.set(id, { callback, delay })
      return id
    },
    clearTimeout(id) { timers.delete(id) },
  }
  context.window = context
  vm.runInNewContext(source, context, { filename: 'chatgpt_win_managed_voice_peer.js' })
  assert.equal(context.__elonWinChatGptManagedVoicePeerLifecycle.commit(context), true)

  return {
    context,
    posted,
    delegated,
    tracks,
    setRelayResult(value) { relayResult = value },
    get armedOffer() { return armedOffer },
    get remoteAnswer() { return remoteAnswer },
    get transcriptHookCount() { return transcriptHookCount },
    get activePeer() { return activePeer },
    runTimer(delay) {
      const entry = [...timers.entries()].find(([, value]) => value.delay === delay)
      assert.ok(entry, `missing ${delay}ms timer`)
      timers.delete(entry[0])
      entry[1].callback()
    },
  }
}

function command(action, value) {
  return JSON.stringify({
    action,
    value,
    requestId: `mcp_${action === 'prepare_realtime_voice' ? 'prepare1' : 'control1'}`,
    documentToken: 'doc_win_voice_contract',
  })
}

async function flush() {
  await Promise.resolve()
  await new Promise((resolve) => setImmediate(resolve))
}

async function main() {
  const active = harness()
  active.context.__elonChatGptBridge.command(command('prepare_realtime_voice'))
  await flush()
  assert.match(active.armedOffer, /^v=0\r\nm=audio/)
  assert.equal(active.transcriptHookCount, 1)
  assert.equal(active.posted.at(-1).action, 'prepare_realtime_voice')
  assert.equal(active.posted.at(-1).ok, true)
  assert.doesNotMatch(JSON.stringify(active.posted), /m=audio|UDP\/TLS/)

  const answer = 'v=0\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\n'
  active.setRelayResult(JSON.stringify({ status: 'ok', answer }))
  active.runTimer(120)
  await flush()
  assert.equal(active.remoteAnswer, answer)
  active.activePeer.connectionState = 'connected'
  active.activePeer.emit('connectionstatechange')
  assert.deepEqual(
    JSON.parse(JSON.stringify(active.context.__elonWinChatGptManagedVoicePeer.status())),
    { version: 1, phase: 'active', active: true, routeBound: true },
  )

  active.context.__elonChatGptBridge.command(command('control_managed_realtime_voice', 'mute'))
  assert.equal(active.tracks[0].enabled, false)
  active.context.__elonChatGptBridge.command(command('control_managed_realtime_voice', 'unmute'))
  assert.equal(active.tracks[0].enabled, true)
  active.context.__elonChatGptBridge.command(command('control_managed_realtime_voice', 'end'))
  assert.equal(active.tracks[0].stopped, true)
  assert.equal(active.activePeer.closed, true)

  active.context.__elonChatGptBridge.command(command('snapshot'))
  assert.equal(active.delegated.at(-1).action, 'snapshot')

  const routeBound = harness()
  routeBound.context.__elonChatGptBridge.command(command('prepare_realtime_voice'))
  await flush()
  routeBound.context.location.pathname = '/c/voice-two'
  routeBound.runTimer(500)
  assert.equal(routeBound.activePeer.closed, true)
  assert.equal(routeBound.context.__elonWinChatGptManagedVoicePeer.status().phase, 'closed')

  const denied = harness({ microphoneDenied: true })
  denied.context.__elonChatGptBridge.command(command('prepare_realtime_voice'))
  await flush()
  assert.equal(denied.posted.at(-1).ok, false)
  assert.match(denied.posted.at(-1).detail, /继续使用官网语音/)
  assert.equal(denied.context.__elonWinChatGptManagedVoicePeer.status().phase, 'failed')

  console.log('PASS ChatGPT Win-managed WebView2 realtime voice relay')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
