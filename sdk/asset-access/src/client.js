import { AssetAccessError, makeTransport, registeredRedirect } from './transport.js';
import { CLIENTS, authorizationCode, tokenResponse, identityResponse, assetResponse,
  revokedResponse, scopes, freezeCopy } from './contract.js';
import { PaginationChain } from './pagination.js';

export { AssetAccessError } from './transport.js';

function randomBase64url() {
  const bytes = globalThis.crypto.getRandomValues(new Uint8Array(32));
  return btoa(String.fromCharCode(...bytes)).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/u, '');
}
async function challenge(verifier) {
  const digest = new Uint8Array(await globalThis.crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier)));
  return btoa(String.fromCharCode(...digest)).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/u, '');
}

/** Credentials and PKCE material live only in private fields of this runtime instance. */
class AssetAccessClient {
  #clientId; #clock; #transport; #origin; #pending = null; #token = null;
  #snapshot = null; #epoch = 0; #controller = null; #status = 'unauthenticated';
  #expires = 0; #effectiveExpiry = null;
  #revocationEpoch = null;
  #pagination = new PaginationChain();

  constructor(options) {
    if (!options || !CLIENTS.includes(options.clientId) ||
        (options.clock !== undefined && typeof options.clock !== 'function') ||
        !globalThis.crypto?.subtle || !globalThis.crypto?.getRandomValues) {
      throw new AssetAccessError('invalid_options');
    }
    this.#clientId = options.clientId;
    this.#clock = options.clock ?? Date.now;
    this.#transport = makeTransport(options);
    this.#origin = new URL(options.baseUrl).origin;
    Object.freeze(this);
  }

  #now() {
    const now = this.#clock();
    if (!Number.isFinite(now)) throw new AssetAccessError('invalid_clock');
    return now;
  }

  #expire() {
    if (this.#token && this.#now() >= this.#expires) { this.clear(); this.#status = 'expired'; }
  }

  get state() {
    this.#expire();
    return freezeCopy({ status: this.#status, client_id: this.#clientId,
      subject: this.#token?.subject ?? null, expires_at: this.#effectiveExpiry,
      scopes: this.#token?.scopes ?? [], has_snapshot: this.#snapshot !== null });
  }

  toJSON() { return this.state; }

  clear() {
    this.#epoch += 1;
    this.#controller?.abort();
    this.#controller = null;
    this.#revocationEpoch = null;
    this.#pending = null;
    this.#token = null;
    this.#expires = 0;
    this.#effectiveExpiry = null;
    this.#snapshot = null;
    this.#pagination.clear();
    this.#status = 'unauthenticated';
  }

  async authorizationRequest({ redirectUri, scopes: requested = ['esk.summary.read'],
    expiresIn = 900, explicitConsent = false } = {}) {
    if (explicitConsent !== true || !Number.isInteger(expiresIn) || expiresIn < 1 || expiresIn > 3600) {
      throw new AssetAccessError('consent_required');
    }
    registeredRedirect(redirectUri, this.#clientId, this.#origin);
    const selected = scopes(requested);
    this.clear();
    const epoch = this.#epoch;
    const verifier = randomBase64url();
    const codeChallenge = await challenge(verifier);
    if (epoch !== this.#epoch) throw new AssetAccessError('cleared');
    const request = freezeCopy({ schema: 'yilong.asset_access.authorize.v1', client_id: this.#clientId,
      redirect_uri: redirectUri, state: randomBase64url(), code_challenge: codeChallenge,
      code_challenge_method: 'S256', scopes: selected, expires_in: expiresIn,
      explicit_consent: true, confirmation: '授权量化只读我的资产' });
    this.#pending = { request, verifier };
    this.#status = 'authorizing';
    return request;
  }

  #begin() {
    if (this.#controller) throw new AssetAccessError('request_in_progress');
    const controller = new AbortController();
    this.#controller = controller;
    return { epoch: this.#epoch, signal: controller.signal };
  }
  #current(epoch) { if (epoch !== this.#epoch) throw new AssetAccessError('cleared'); }
  #finish(epoch) { if (epoch === this.#epoch) this.#controller = null; }
  #credential() {
    this.#expire();
    if (!this.#token) throw new AssetAccessError(this.#status === 'expired' ? 'expired' : 'authorization_required');
    return this.#token;
  }

  async exchangeCode(response) {
    if (!this.#pending) throw new AssetAccessError('authorization_required');
    const operation = this.#begin();
    const pending = this.#pending;
    this.#pending = null;
    try {
      const code = authorizationCode(response, pending, this.#now());
      const data = await this.#transport('token', { signal: operation.signal, body: {
        schema: 'yilong.asset_access.token_request.v1', grant_type: 'authorization_code',
        client_id: this.#clientId, redirect_uri: pending.request.redirect_uri,
        state: pending.request.state, code: code.code, code_verifier: pending.verifier } });
      this.#current(operation.epoch);
      const now = this.#now();
      this.#token = tokenResponse(data, code, now);
      this.#expires = Math.min(Date.parse(this.#token.expires_at), now + this.#token.expires_in * 1000);
      this.#effectiveExpiry = new Date(this.#expires).toISOString();
      this.#status = 'authorized';
      return this.state;
    } catch (error) {
      if (operation.epoch === this.#epoch) this.clear();
      throw error;
    } finally { this.#finish(operation.epoch); }
  }

  async identity() {
    const token = this.#credential();
    const operation = this.#begin();
    try {
      const data = await this.#transport('me', { token: token.access_token,
        clientId: this.#clientId, signal: operation.signal });
      this.#current(operation.epoch);
      this.#credential();
      const identity = identityResponse(data, token, this.#now());
      this.#narrowExpiry(identity.expires_at);
      return identity;
    } catch (error) {
      if (operation.epoch === this.#epoch) this.clear();
      throw error;
    } finally { this.#finish(operation.epoch); }
  }

  async readAssets({ limit = 20, cursor = null,
    includeProgress = this.#token?.scopes.includes('esk.progress.read') ?? false } = {}) {
    const token = this.#credential();
    if (!Number.isInteger(limit) || limit < 1 || limit > 20 || typeof includeProgress !== 'boolean' ||
        (includeProgress && !token.scopes.includes('esk.progress.read')) ||
        (cursor !== null && (typeof cursor !== 'string' || cursor.length === 0 || cursor.length > 160 ||
          !includeProgress || cursor !== this.#snapshot?.progress?.next_cursor))) {
      throw new AssetAccessError('invalid_query');
    }
    const operation = this.#begin();
    const previous = this.#snapshot;
    this.#snapshot = null;
    let restarted = false;
    const load = async currentCursor => {
      const query = { limit: String(limit), include_progress: String(includeProgress) };
      if (currentCursor) query.cursor = currentCursor;
      const data = await this.#transport('esk', { token: token.access_token,
        clientId: this.#clientId, query, signal: operation.signal });
      this.#current(operation.epoch);
      const active = this.#credential();
      const page = assetResponse(data, active, { now: this.#now(), limit, includeProgress,
        previous: currentCursor ? previous : null, cursor: currentCursor });
      if (this.#narrowExpiry(page.expires_at) && currentCursor) throw new AssetAccessError('snapshot_changed', 409);
      return page;
    };
    try {
      let page;
      try { page = await load(cursor); }
      catch (error) {
        if (error.code !== 'snapshot_changed' || !cursor) throw error;
        this.#current(operation.epoch);
        restarted = true;
        page = await load(null);
      }
      this.#pagination.accept(page, restarted ? null : cursor);
      this.#snapshot = page;
      return Object.freeze({ page, restarted });
    } catch (error) {
      if (operation.epoch === this.#epoch) this.clear();
      throw error;
    } finally { this.#finish(operation.epoch); }
  }

  async revoke() {
    if (this.#revocationEpoch === this.#epoch) throw new AssetAccessError('request_in_progress');
    let token;
    // Withdrawal also cancels pending consent or exchange before a token exists.
    // Capture an existing token when possible, but always invalidate the old epoch.
    try { token = this.#credential(); } finally { this.clear(); }
    const operation = this.#begin();
    this.#revocationEpoch = operation.epoch;
    try {
      const data = await this.#transport('revoke', { token: token.access_token,
        clientId: this.#clientId, signal: operation.signal,
        body: { schema: 'yilong.asset_access.revoke.v1', confirmation: '撤销只读资产授权' } });
      this.#current(operation.epoch);
      return revokedResponse(data);
    } finally {
      if (operation.epoch === this.#epoch) this.clear();
    }
  }

  #narrowExpiry(expiresAt) {
    const expires = Date.parse(expiresAt);
    if (expires >= this.#expires) return false;
    this.#expires = expires;
    this.#effectiveExpiry = new Date(expires).toISOString();
    this.#token = freezeCopy({ ...this.#token, expires_at: this.#effectiveExpiry });
    this.#snapshot = null;
    this.#pagination.clear();
    return true;
  }
}

export function createAssetAccessClient(options) { return new AssetAccessClient(options); }
