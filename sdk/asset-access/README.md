# 只读资产客户端 V1

零第三方依赖 ESM，支持 Node.js 20+ 和安全上下文中的现代浏览器。共同线协议在
`contracts/assets/asset-access-v1.schema.json`，合成向量在相邻 fixture 文件。

主项目保存身份和正式账本。SDK 只保存当前运行实例的受限授权，不创建登录账号、
不复制主会话、不执行交易或资金操作。ESK 的 `platform_recorded`、`not_deployed`
和 `funds_moved:false` 必须保留；可申请量不能改名为可提现余额。

```js
import { createAssetAccessClient } from './src/client.js';

const client = createAssetAccessClient({
  baseUrl: 'https://your-approved-asset-origin.example',
  clientId: 'quant.web', // quant.ai uses a separately registered client
});

// Run only after your trusted UI displays the account, scopes, lifetime and
// revocation entry and the user explicitly agrees. This does not send HTTP.
const request = await client.authorizationRequest({
  redirectUri: 'https://your-approved-asset-origin.example/quant/asset-access/callback',
  scopes: ['esk.summary.read', 'esk.progress.read'],
  expiresIn: 900,
  explicitConsent: true,
});

// Deliver request to the trusted main-account consent surface. That surface
// owns POST /api/me/asset-access/authorize and retains the main credential.
// Route its authorization response directly to this SAME runtime instance.
// Do not serialize this response or exchange secrets into model messages.
await client.exchangeCode(authorizationResponse);
const identity = await client.identity();
let result = await client.readAssets({ limit: 20 });
if (result.page.progress?.has_more) {
  result = await client.readAssets({ cursor: result.page.progress.next_cursor });
  // restarted=true means discard previously collected pages and replace them.
}
await client.revoke();
```

`authorizationResponse` above is an input from the host integration, not a global supplied by
the SDK. Authorization cannot survive a full-page unload because its PKCE verifier exists only
in memory. Browser integrations must keep the requesting runtime alive, for example using a
trusted popup callback with exact origin, source-window and state checks, or place this SDK in
a BFF and use its protected session. The SDK does not install a popup or cross-origin receiver.

## Contract and lifecycle

- `authorizationRequest()` returns frozen request parameters; it never sends the main-account token.
  The caller must first obtain explicit consent. New authorization clears the prior local session.
- `exchangeCode(response)` consumes one local PKCE transaction and returns safe state only.
  It checks code/client/state/redirect/scopes/expiry before sending the one-time exchange.
- `identity()` returns the delegated subject, client and grant; `nickname` appears only with
  `profile.read`. Main user IDs, account names, email addresses and roles are rejected.
- `readAssets({limit,cursor,includeProgress})` returns `{page,restarted}`. `limit` is 1–20 and
  cursors are at most 160 characters. Progress defaults to
  whether the grant has `esk.progress.read`. Only the previous returned cursor is accepted.
  HTTP 409 `asset_access_snapshot_changed` or a changed response digest resets pagination
  and reads page one once. A second failure stops; consumers must replace old pages on restart.
- A resource response may shorten the original grant deadline when the parent session was
  shortened. The SDK only reduces its deadline, clears earlier cached pages, and restarts an
  in-progress pagination from page one. A later response cannot restore the longer deadline.
- `revoke()` confirms server revocation and always clears locally. If the server request fails,
  the method rejects; local clearing alone does not prove the remote grant was revoked. Use
  the main account's owner-only grant/revoke API when the SDK has lost its credential; the
  main APK's visual authorization manager is not connected in this version.
- `clear()` is synchronous local cleanup, aborts in-flight reads and blocks late results.
  Call it on logout, account change, window close and host session shutdown.
- `state` and `toJSON()` expose status, delegated subject/client, grant expiry, scopes and
  whether a snapshot exists. They contain no token, authorization code, verifier or state nonce.
  Errors expose stable codes/status only, never server-provided error text.

Access tokens have no refresh mechanism in V1. Any failed read clears the local token and
snapshot, except the one bounded snapshot restart. Expiration and 401/403 require new explicit
authorization; network errors are errors, never successful zero balances. Reads are serialized:
overlapping calls reject with `request_in_progress`; they do not cancel an already valid read.

Amounts and counts are canonical nonnegative integer strings, bounded by signed i64. The SDK
checks them using `BigInt`, verifies `total = reserved + available`, and retains strings in
returned JSON. Never convert balances through JavaScript `Number`.

## Transport and AI runtime

Production API URLs must be HTTPS origins without paths, query, credentials or fragments.
The V1 `quant.web` callback is exactly `<API origin>/quant/asset-access/callback`; deploy that
origin consistently with the server's HTTPS `public_url`. The `quant.ai` callback is exactly
`http://127.0.0.1:<port>/asset-access/callback`, with port 1024–65535. This production loopback
callback exception never allows plaintext remote API traffic. `quant.android` uses the fixed
native callback `com.elon.quant:/asset-access/callback`; the main APK's native approval adapter
accepts only state/challenge and independently fixes scopes to summary/progress for 15 minutes.
This package does not install that Android adapter. Redirect following
is disabled, browser cookies are omitted, and all resource requests bind
`Authorization: Bearer …` with `X-Elon-Asset-Client`. There is no arbitrary authenticated path API.

For a local test API server only, `allowLoopbackHttp:true` permits HTTP on `127.0.0.1` or `[::1]`.
This does not permit public HTTP or `localhost`. Tests can inject `fetch` and `clock` (epoch
milliseconds). Fetch plus streamed-body processing share a 10-second timeout and 128 KiB byte
limit, configurable by `timeoutMs` (1–30000) and `maxResponseBytes` (1–1048576).

An AI tool host must retain one SDK instance in protected runtime state and expose only
`identity` and sanitized `{page,restarted}` outputs as model-visible results. Authorization
requests/responses, token exchange bodies, injected fetch request headers and runtime objects
must not be logged or returned to the model. `client_id` identifies the registered client;
it does not itself prove possession of that application's code or signature.

## Verification

Run `node --test sdk/asset-access/test/client.test.js` from the repository root. Tests execute
PKCE exchange, fixed HTTP routing, real streamed responses, consent reuse across pagination,
revocation, expiry, subject changes, large integers, malformed input, timeout and clearing races
against synthetic data. They do not authenticate a real user or prove production HTTPS is ready.

For actual Rust-to-SDK interoperability, set `ELON_ASSET_ACCESS_WIRE_OUTPUT` to a new temporary
JSON path, then run the harness test
`synthetic_delegated_wire_export_matches_formal_truth_without_credentials` through
`scripts/validate-rust.ps1 -- test --manifest-path server/tests/esk-platform-harness/Cargo.toml`.
Keep the same environment variable when running
`node --test sdk/asset-access/test/client.test.js sdk/asset-access/test/rust-wire.test.js`.
The export contains synthetic identity/asset JSON only. The exporter refuses to overwrite an
existing file. Without the variable the SDK wire test is skipped, so that run cannot prove
cross-language interoperability.
