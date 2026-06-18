(function (global) {
  function normalizeBase(baseUrl) {
    return String(baseUrl || '').replace(/\/+$/, '');
  }

  function joinUrl(baseUrl, path) {
    return normalizeBase(baseUrl) + path;
  }

  async function parseError(resp) {
    const text = await resp.text().catch(() => '');
    if (!text) return resp.statusText || 'TTS request failed';
    try {
      const data = JSON.parse(text);
      return data.error || data.message || text;
    } catch (_) {
      return text;
    }
  }

  class ElonVoiceTtsClient {
    constructor(options) {
      const opts = options || {};
      this.baseUrl = normalizeBase(opts.baseUrl || '');
      this.token = opts.token || '';
      this.tokenProvider = typeof opts.tokenProvider === 'function' ? opts.tokenProvider : null;
    }

    authToken() {
      return this.tokenProvider ? (this.tokenProvider() || '') : this.token;
    }

    jsonHeaders(extra) {
      const headers = Object.assign({ 'Content-Type': 'application/json' }, extra || {});
      const token = this.authToken();
      if (token) headers.Authorization = 'Bearer ' + token;
      return headers;
    }

    async catalog() {
      const resp = await fetch(joinUrl(this.baseUrl, '/api/voice/tts/catalog'), {
        headers: this.jsonHeaders()
      });
      if (!resp.ok) throw new Error(await parseError(resp));
      return resp.json();
    }

    async synthesizeBlob(request) {
      const body = Object.assign({ rewrite: true }, request || {});
      const resp = await fetch(joinUrl(this.baseUrl, '/api/voice/tts'), {
        method: 'POST',
        headers: this.jsonHeaders(),
        body: JSON.stringify(body)
      });
      if (!resp.ok) throw new Error(await parseError(resp));
      return {
        blob: await resp.blob(),
        contentType: resp.headers.get('content-type') || 'audio/wav',
        provider: resp.headers.get('x-elon-tts-provider') || '',
        voice: resp.headers.get('x-elon-tts-voice') || '',
        emotion: resp.headers.get('x-elon-tts-emotion') || '',
        cache: resp.headers.get('x-elon-tts-cache') || ''
      };
    }

    async play(request) {
      const audio = await this.synthesizeBlob(request);
      const url = URL.createObjectURL(audio.blob);
      const player = new Audio(url);
      player.addEventListener('ended', () => URL.revokeObjectURL(url), { once: true });
      player.addEventListener('error', () => URL.revokeObjectURL(url), { once: true });
      await player.play();
      return Object.assign({ objectUrl: url, player }, audio);
    }
  }

  function createClient(options) {
    return new ElonVoiceTtsClient(options);
  }

  global.ElonVoiceTts = {
    Client: ElonVoiceTtsClient,
    createClient
  };
})(window);
