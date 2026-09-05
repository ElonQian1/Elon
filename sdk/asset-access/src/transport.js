/** Fixed-endpoint JSON transport. No arbitrary authenticated request surface is exported. */
export class AssetAccessError extends Error {
  constructor(code, status = 0) {
    super(`Asset access: ${code}`);
    this.name = 'AssetAccessError';
    this.code = code;
    this.status = status;
  }
  toJSON() { return { code: this.code, status: this.status }; }
}

export function secureUrl(value, allowLoopback = false) {
  if (typeof value !== 'string' || value.length > 2048 || /[\s\\]/u.test(value)) {
    throw new AssetAccessError('invalid_url');
  }
  let url;
  try { url = new URL(value); } catch { throw new AssetAccessError('invalid_url'); }
  const loopback = ['127.0.0.1', '[::1]'].includes(url.hostname);
  if (url.username || url.password || url.hash || value.includes('#') ||
      (url.protocol !== 'https:' && !(allowLoopback && loopback && url.protocol === 'http:'))) {
    throw new AssetAccessError('insecure_transport');
  }
  return url;
}

/** V1's registered callbacks. Loopback authorization callbacks are not HTTP API permission. */
export function registeredRedirect(value, clientId, baseOrigin) {
  if (clientId === 'quant.android') {
    if (value !== 'com.elon.quant:/asset-access/callback') throw new AssetAccessError('invalid_redirect');
    return value;
  }
  const url = secureUrl(value, clientId === 'quant.ai');
  if (clientId === 'quant.web') {
    if (url.protocol !== 'https:' || value !== new URL('/quant/asset-access/callback', baseOrigin).href) {
      throw new AssetAccessError('invalid_redirect');
    }
  } else if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' ||
      !url.port || Number(url.port) < 1024 || url.pathname !== '/asset-access/callback' ||
      url.search || value !== url.href) {
    throw new AssetAccessError('invalid_redirect');
  }
  return value;
}

async function boundedJson(response, maxBytes, signal) {
  const length = response.headers.get('content-length');
  if (length !== null && (!/^\d+$/u.test(length) || Number(length) > maxBytes)) {
    void response.body?.cancel().catch(() => {});
    throw new AssetAccessError('response_too_large');
  }
  if (!/^application\/json(?:\s*;|$)/iu.test(response.headers.get('content-type') ?? '')) {
    void response.body?.cancel().catch(() => {});
    throw new AssetAccessError('invalid_response');
  }
  if (!response.body) throw new AssetAccessError('invalid_response');
  const reader = response.body.getReader();
  const cancel = () => { void reader.cancel().catch(() => {}); };
  signal.addEventListener('abort', cancel, { once: true });
  if (signal.aborted) cancel();
  const chunks = [];
  let total = 0;
  try {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      total += value.byteLength;
      if (total > maxBytes) throw new AssetAccessError('response_too_large');
      chunks.push(value);
    }
  } catch (error) {
    void reader.cancel().catch(() => {});
    throw error;
  } finally { signal.removeEventListener('abort', cancel); reader.releaseLock(); }
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
  try { return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)); }
  catch { throw new AssetAccessError('invalid_response'); }
}

export function makeTransport({ baseUrl, fetch: fetchImpl = globalThis.fetch,
  allowLoopbackHttp = false, timeoutMs = 10000, maxResponseBytes = 131072 }) {
  const base = secureUrl(baseUrl, allowLoopbackHttp);
  if (base.pathname !== '/' || base.search || typeof fetchImpl !== 'function' || typeof allowLoopbackHttp !== 'boolean' ||
      !Number.isInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 30000 ||
      !Number.isInteger(maxResponseBytes) || maxResponseBytes < 1 || maxResponseBytes > 1048576) {
    throw new AssetAccessError('invalid_options');
  }
  return async function request(endpoint, { token, clientId, body, query, signal } = {}) {
    const paths = { token: '/api/asset-access/token', me: '/api/asset-access/me',
      esk: '/api/asset-access/esk', revoke: '/api/asset-access/revoke' };
    if (!Object.hasOwn(paths, endpoint)) throw new AssetAccessError('invalid_endpoint');
    const url = new URL(paths[endpoint], base);
    if (query) url.search = new URLSearchParams(query).toString();
    const controller = new AbortController();
    const abort = () => controller.abort();
    signal?.addEventListener('abort', abort, { once: true });
    if (signal?.aborted) abort();
    const headers = { Accept: 'application/json' };
    if (body) headers['Content-Type'] = 'application/json';
    if (token) { headers.Authorization = `Bearer ${token}`; headers['X-Elon-Asset-Client'] = clientId; }
    let timer, aborted, timedOut = false;
    const deadline = new Promise((_, reject) => {
      aborted = () => reject(new AssetAccessError(timedOut ? 'timeout' : 'cleared'));
      controller.signal.addEventListener('abort', aborted, { once: true });
      if (controller.signal.aborted) aborted();
      timer = setTimeout(() => { timedOut = true; controller.abort(); }, timeoutMs);
    });
    try {
      const operation = (async () => {
        const response = await fetchImpl(url.href, { method: body ? 'POST' : 'GET', headers,
          ...(body ? { body: JSON.stringify(body) } : {}), redirect: 'error', credentials: 'omit',
          cache: 'no-store', referrerPolicy: 'no-referrer', signal: controller.signal });
        if (response.redirected || (response.url && response.url !== url.href) ||
            (response.status >= 300 && response.status < 400)) {
          void response.body?.cancel().catch(() => {});
          throw new AssetAccessError('redirect_rejected');
        }
        const data = await boundedJson(response, maxResponseBytes, controller.signal);
        if (!response.ok) {
          const code = response.status === 401 ? 'unauthorized' : response.status === 403 ? 'forbidden' :
            response.status === 409 && data?.code === 'asset_access_snapshot_changed' ? 'snapshot_changed' : 'request_failed';
          throw new AssetAccessError(code, response.status);
        }
        return data;
      })();
      return await Promise.race([operation, deadline]);
    } catch (error) {
      if (error instanceof AssetAccessError) throw error;
      throw new AssetAccessError(signal?.aborted ? 'cleared' : 'network_error');
    } finally {
      clearTimeout(timer);
      controller.signal.removeEventListener('abort', aborted);
      signal?.removeEventListener('abort', abort);
    }
  };
}
