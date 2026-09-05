/** Canonical nonnegative decimal integer strings, runtime-checked against signed i64. */
export type BaseUnits = string;
export type AssetScope = 'esk.summary.read' | 'esk.progress.read' | 'profile.read';
export type ClientId = 'quant.android' | 'quant.web' | 'quant.ai';
export interface AssetAccessOptions {
  baseUrl: string;
  clientId: ClientId;
  fetch?: typeof globalThis.fetch;
  clock?: () => number;
  allowLoopbackHttp?: boolean;
  timeoutMs?: number;
  maxResponseBytes?: number;
}
export interface AssetAccessState {
  readonly status: 'unauthenticated' | 'authorizing' | 'authorized' | 'expired';
  readonly client_id: ClientId;
  readonly subject: string | null;
  readonly expires_at: string | null;
  readonly scopes: readonly AssetScope[];
  readonly has_snapshot: boolean;
}
export interface AuthorizationRequest {
  readonly schema: 'yilong.asset_access.authorize.v1';
  readonly client_id: ClientId;
  readonly redirect_uri: string;
  readonly state: string;
  readonly code_challenge: string;
  readonly code_challenge_method: 'S256';
  readonly scopes: readonly AssetScope[];
  readonly expires_in: number;
  readonly explicit_consent: true;
  readonly confirmation: '授权量化只读我的资产';
}
export interface DelegatedIdentity {
  readonly schema: 'yilong.asset_access.identity.v1';
  readonly audience: 'yilong-quant';
  readonly subject: string;
  readonly client_id: ClientId;
  readonly grant_id: string;
  readonly expires_at: string;
  readonly scopes: readonly AssetScope[];
  readonly nickname?: string;
}
export interface AssetRequestProgress {
  readonly request_id: string;
  readonly amount_base_units: BaseUnits;
  readonly status: 'submitted' | 'canceled';
  readonly created_at: string;
  readonly canceled_at: string | null;
}
export interface AssetPage {
  readonly schema: 'yilong.esk.delegated_asset_page.v1';
  readonly subject: string;
  readonly client_id: ClientId;
  readonly expires_at: string;
  readonly asset: {
    readonly asset_id: 'esk'; readonly symbol: 'ESK'; readonly decimals: 6;
    readonly source: 'platform_recorded'; readonly simulated: false;
    readonly chain_status: 'not_deployed'; readonly funds_moved: false;
  };
  readonly balance: {
    readonly total_base_units: BaseUnits;
    readonly reserved_base_units: BaseUnits;
    readonly available_base_units: BaseUnits;
  };
  readonly snapshot_digest: string;
  readonly progress?: {
    readonly request_count: string; readonly open_count: string;
    readonly range_start: string; readonly range_end: string;
    readonly requests: readonly AssetRequestProgress[];
    readonly has_more: boolean; readonly next_cursor: string | null;
  };
}
export interface AssetAccessClient {
  readonly state: AssetAccessState;
  toJSON(): AssetAccessState;
  clear(): void;
  authorizationRequest(options: {
    redirectUri: string; scopes?: readonly AssetScope[]; expiresIn?: number; explicitConsent: true;
  }): Promise<AuthorizationRequest>;
  exchangeCode(response: unknown): Promise<AssetAccessState>;
  identity(): Promise<DelegatedIdentity>;
  readAssets(options?: { limit?: number; cursor?: string | null; includeProgress?: boolean }):
    Promise<Readonly<{ page: AssetPage; restarted: boolean }>>;
  revoke(): Promise<Readonly<{ schema: 'yilong.asset_access.revoked.v1'; revoked: true; funds_moved: false }>>;
}
export class AssetAccessError extends Error {
  readonly code: string;
  readonly status: number;
  constructor(code: string, status?: number);
  toJSON(): { code: string; status: number };
}
export function createAssetAccessClient(options: AssetAccessOptions): AssetAccessClient;
