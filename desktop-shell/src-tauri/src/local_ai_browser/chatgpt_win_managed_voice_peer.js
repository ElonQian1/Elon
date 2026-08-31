(function () {
  'use strict';

  const root = window;
  if (location.origin !== 'https://chatgpt.com') return;
  const previous = root.__elonWinChatGptManagedVoicePeer;
  if (previous && Number(previous.version) >= 2) return;
  try { if (previous && typeof previous.dispose === 'function') previous.dispose(); } catch (_) {}

  const PeerConnection = root.__elonWinChatGptManagedVoicePeerConstructor;
  const MAX_SDP_CHARS = 320000;
  const RELAY_TIMEOUT_MS = 16500;
  const CONNECT_TIMEOUT_MS = 18000;
  const POLL_MS = 120;
  const ROUTE_POLL_MS = 500;
  const REQUEST_ID = /^mcp_[a-z0-9]{1,32}$/;
  const DOCUMENT_TOKEN = /^doc_[a-z0-9_]{3,80}$/;
  const AUDIO_ID = 'elon-win-chatgpt-managed-voice-audio';
  let generation = 0;
  let phase = 'idle';
  let requestId = '';
  let route = '';
  let peer = null;
  let microphone = null;
  let audio = null;
  let pollTimer = 0;
  let routeTimer = 0;
  let deadline = 0;
  let microphoneActive = false;
  let remoteAudio = false;
  let muted = false;
  let fallbackCode = '';
  let lifecycleRevision = 0;

  function publishState() {
    lifecycleRevision += 1;
    const transcript = root.__elonWinChatGptRealtimeVoiceTranscript;
    if (!transcript || typeof transcript.updateManagedState !== 'function') return;
    transcript.updateManagedState({
      phase,
      active: phase === 'active',
      microphoneActive,
      remoteAudio,
      muted,
      routeBound: Boolean(route),
      fallbackCode,
      revision: lifecycleRevision,
    });
  }

  function transition(nextPhase, code) {
    phase = nextPhase;
    fallbackCode = typeof code === 'string' ? code.slice(0, 80) : '';
    publishState();
  }

  function relay() {
    const value = root.__elonChatGptPrivateVoiceRelay;
    return value && Number(value.version) >= 4 ? value : null;
  }

  function parseObject(raw) {
    if (typeof raw !== 'string' || raw.length > MAX_SDP_CHARS + 2000) return null;
    try {
      const value = JSON.parse(raw);
      return value && typeof value === 'object' && !Array.isArray(value) ? value : null;
    } catch (_) { return null; }
  }

  function validSdp(value) {
    return typeof value === 'string' && value.length >= 16 && value.length <= MAX_SDP_CHARS &&
      /^v=0(?:\r?\n)/.test(value) && /(?:\r?\n)m=audio\s/i.test(value);
  }

  function stopTracks(stream) {
    try { stream && stream.getTracks().forEach((track) => track.stop()); } catch (_) {}
  }

  function removeAudio() {
    if (!audio) return;
    try { audio.pause(); } catch (_) {}
    try { audio.srcObject = null; } catch (_) {}
    try { audio.remove(); } catch (_) {}
    audio = null;
  }

  function clearTimers() {
    if (pollTimer) root.clearTimeout(pollTimer);
    if (routeTimer) root.clearTimeout(routeTimer);
    pollTimer = 0;
    routeTimer = 0;
  }

  function releaseLocal(nextPhase, code) {
    generation += 1;
    clearTimers();
    const activePeer = peer;
    const activeMicrophone = microphone;
    peer = null;
    microphone = null;
    requestId = '';
    route = '';
    deadline = 0;
    microphoneActive = false;
    remoteAudio = false;
    muted = false;
    removeAudio();
    stopTracks(activeMicrophone);
    try { activePeer && activePeer.close(); } catch (_) {}
    transition(nextPhase, code);
  }

  function fail(code) {
    releaseLocal('failed', String(code || 'managed_voice_failed'));
    return { ok: false, code: String(code || 'managed_voice_failed').slice(0, 80) };
  }

  function relayCancel(id) {
    const bridge = relay();
    try { if (bridge && id) bridge.cancelExchange(id); } catch (_) {}
  }

  function resetTakeover() {
    const bridge = relay();
    try { if (bridge) bridge.resetTakeover(); } catch (_) {}
  }

  function end() {
    const id = requestId;
    relayCancel(id);
    releaseLocal('closed');
    resetTakeover();
    return { ok: true, code: null };
  }

  function dataChannelOptions(value) {
    const source = value && typeof value === 'object' ? value : {};
    const options = { ordered: source.ordered !== false };
    if (Number.isInteger(source.maxRetransmits)) options.maxRetransmits = source.maxRetransmits;
    if (typeof source.protocol === 'string' && source.protocol.length <= 64) {
      options.protocol = source.protocol;
    }
    if (source.negotiated === true && Number.isInteger(source.id)) {
      options.negotiated = true;
      options.id = source.id;
    }
    return options;
  }

  function attachRemoteAudio(event) {
    const track = event && event.track;
    if (!track || track.kind !== 'audio') return;
    const stream = event.streams && event.streams[0]
      ? event.streams[0]
      : typeof MediaStream === 'function' ? new MediaStream([track]) : null;
    if (!stream) return;
    removeAudio();
    const element = document.createElement('audio');
    element.id = AUDIO_ID;
    element.autoplay = true;
    element.playsInline = true;
    element.style.display = 'none';
    element.srcObject = stream;
    (document.body || document.documentElement).appendChild(element);
    audio = element;
    if (typeof track.addEventListener === 'function') {
      track.addEventListener('ended', function () {
        remoteAudio = false;
        publishState();
      });
    }
    const play = element.play();
    if (play && typeof play.then === 'function') {
      play.then(function () {
        if (audio !== element) return;
        remoteAudio = true;
        publishState();
      }, function () {
        remoteAudio = false;
        publishState();
      });
    } else {
      remoteAudio = true;
      publishState();
    }
  }

  function isConversationRoute(value) {
    return /^\/c\/[A-Za-z0-9_-]{1,192}\/?$/.test(String(value || ''));
  }

  function acceptRouteTransition(nextRoute) {
    if (nextRoute === route) return true;
    // ChatGPT assigns /c/{id} after a voice call starts from the blank route.
    // This is the same conversation gaining an identity, not a cross-chat switch.
    if (route === '/' && isConversationRoute(nextRoute)) {
      route = nextRoute;
      publishState();
      return true;
    }
    return false;
  }

  function scheduleRouteGuard(token) {
    routeTimer = root.setTimeout(function checkRoute() {
      if (token !== generation || !peer) return;
      if (!acceptRouteTransition(location.pathname)) {
        end();
        return;
      }
      routeTimer = root.setTimeout(checkRoute, ROUTE_POLL_MS);
    }, ROUTE_POLL_MS);
  }

  function scheduleResultPoll(token) {
    pollTimer = root.setTimeout(async function poll() {
      if (token !== generation || !peer || !requestId) return;
      if (Date.now() >= deadline) {
        fail('relay_timeout');
        return;
      }
      const bridge = relay();
      let result = null;
      try { result = bridge && parseObject(bridge.takeResult(requestId)); } catch (_) {}
      if (!result) {
        scheduleResultPoll(token);
        return;
      }
      if (result.status !== 'ok' || !validSdp(result.answer)) {
        fail(result.code || 'relay_failed');
        return;
      }
      transition('applying_answer');
      try {
        await peer.setRemoteDescription({ type: 'answer', sdp: result.answer });
        if (token !== generation || !peer) return;
        transition('connecting');
        pollTimer = root.setTimeout(function connectionTimeout() {
          if (token === generation && peer && phase !== 'active') fail('connection_timeout');
        }, CONNECT_TIMEOUT_MS);
      } catch (_) {
        fail('remote_description_failed');
      }
    }, POLL_MS);
  }

  async function prepare() {
    if (!['idle', 'failed', 'closed'].includes(phase)) return { ok: false, code: 'busy' };
    const bridge = relay();
    if (!bridge || typeof PeerConnection !== 'function') return fail('relay_unavailable');
    if (!navigator.mediaDevices || typeof navigator.mediaDevices.getUserMedia !== 'function') {
      return fail('microphone_unavailable');
    }
    const bootstrap = parseObject(bridge.bootstrap());
    if (!bootstrap || bootstrap.available !== true) return fail('relay_busy');
    const token = ++generation;
    transition('requesting_microphone');
    route = location.pathname;
    publishState();
    try {
      microphone = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true },
        video: false,
      });
      if (token !== generation) return { ok: false, code: 'stale' };
      const microphoneTracks = microphone.getAudioTracks();
      if (!microphoneTracks.length) return fail('microphone_track_unavailable');
      microphoneActive = true;
      muted = false;
      publishState();
      peer = new PeerConnection({});
      const transcript = root.__elonWinChatGptRealtimeVoiceTranscript;
      if (transcript && typeof transcript.hookPeer === 'function') transcript.hookPeer(peer);
      peer.addEventListener('track', attachRemoteAudio);
      peer.addEventListener('connectionstatechange', function () {
        if (token !== generation || !peer) return;
        if (peer.connectionState === 'connected') {
          if (pollTimer) root.clearTimeout(pollTimer);
          pollTimer = 0;
          transition('active');
        }
        else if (peer.connectionState === 'failed') fail('peer_failed');
        else if (peer.connectionState === 'closed') releaseLocal('closed');
      });
      peer.addEventListener('iceconnectionstatechange', function () {
        if (token === generation && peer && peer.iceConnectionState === 'failed') fail('ice_failed');
      });
      microphoneTracks.forEach((track) => {
        peer.addTrack(track, microphone);
        if (typeof track.addEventListener === 'function') {
          track.addEventListener('ended', function () {
            if (token === generation && peer) fail('microphone_track_ended');
          });
        }
      });
      const channel = bootstrap.dataChannel && typeof bootstrap.dataChannel === 'object'
        ? bootstrap.dataChannel : {};
      peer.createDataChannel(typeof channel.label === 'string' ? channel.label : '', dataChannelOptions(channel));
      transition('creating_offer');
      const offer = await peer.createOffer({ offerToReceiveAudio: true });
      await peer.setLocalDescription(offer);
      const sdp = peer.localDescription && peer.localDescription.sdp;
      if (!validSdp(sdp)) return fail('invalid_local_offer');
      requestId = 'relay_' + Array.from(crypto.getRandomValues(new Uint32Array(2)), function (word) {
        return word.toString(16).padStart(8, '0');
      }).join('');
      const armed = parseObject(bridge.armExchange(requestId, sdp));
      if (!armed || armed.armed !== true) return fail(armed && armed.code || 'relay_rejected');
      transition('armed');
      deadline = Date.now() + RELAY_TIMEOUT_MS;
      scheduleResultPoll(token);
      scheduleRouteGuard(token);
      return { ok: true, code: null };
    } catch (error) {
      const name = String(error && error.name || '');
      return fail(name === 'NotAllowedError' ? 'microphone_permission_required' : 'prepare_failed');
    }
  }

  function control(action) {
    if (action === 'end') return end();
    if (action !== 'mute' && action !== 'unmute') return { ok: false, code: 'invalid_control' };
    if (!microphone) return { ok: true, code: 'official_fallback' };
    const enabled = action === 'unmute';
    let changed = 0;
    try {
      microphone.getAudioTracks().forEach((track) => {
        track.enabled = enabled;
        if (track.enabled === enabled) changed += 1;
      });
    } catch (_) {}
    if (changed > 0) {
      muted = !enabled;
      microphoneActive = true;
      publishState();
    }
    return changed > 0 ? { ok: true, code: null } : { ok: false, code: 'microphone_track_unavailable' };
  }

  function emitResult(command, outcome) {
    const id = String(command.requestId || '');
    const token = String(command.documentToken || '');
    if (!REQUEST_ID.test(id) || !DOCUMENT_TOKEN.test(token) || token !== root.__elonChatGptDocumentToken) return;
    const nativeBridge = root.elonChatGptNative;
    if (!nativeBridge || typeof nativeBridge.postMessage !== 'function') return;
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      adapterVersion: Number(root.__elonChatGptAdapterVersion || 0),
      documentToken: token,
      requestId: id,
      action: String(command.action || ''),
      ok: outcome.ok === true,
      detail: outcome.ok === true ? '' : 'Win 实时语音增强暂不可用，已继续使用官网语音。',
    }));
  }

  function handle(raw) {
    let command = null;
    try { command = JSON.parse(String(raw || '{}')); } catch (_) { return false; }
    if (!command || !['prepare_realtime_voice', 'control_managed_realtime_voice'].includes(command.action)) {
      return false;
    }
    if (command.action === 'prepare_realtime_voice') {
      void prepare().then((outcome) => emitResult(command, outcome));
    } else {
      emitResult(command, control(String(command.value || '')));
    }
    return true;
  }

  const api = Object.freeze({
    version: 2,
    prepare,
    control,
    handle,
    dispose: end,
    status() {
      return Object.freeze({
        version: 2,
        phase,
        active: phase === 'active',
        microphoneActive,
        remoteAudio,
        muted,
        routeBound: Boolean(route),
        fallbackCode,
        revision: lifecycleRevision,
      });
    },
  });
  root.__elonWinChatGptManagedVoicePeer = api;
  root.__elonWinChatGptManagedVoicePeerLifecycle = Object.freeze({
    version: 2,
    commit(target) {
      const base = target.__elonChatGptBridge;
      if (!base || typeof base.command !== 'function' || Number(base.__elonWinManagedVoiceVersion) >= 2) {
        return false;
      }
      target.__elonChatGptBridge = Object.freeze({
        version: base.version,
        __elonWinManagedVoice: true,
        __elonWinManagedVoiceVersion: 2,
        command(raw) { if (!api.handle(raw)) base.command(raw); },
        dispose() { api.dispose(); if (typeof base.dispose === 'function') base.dispose(); },
      });
      return true;
    },
  });
  publishState();
})();
