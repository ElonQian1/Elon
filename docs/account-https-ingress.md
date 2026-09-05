# Native account HTTPS ingress

This optional Rust listener lets first-party child backends validate main accounts
over trusted HTTPS. It does not publish the general main API, install Nginx, change
the existing 8080 service, or copy the account/password database.

## Configuration

- `ACCOUNT_HTTPS_ENABLED`: defaults off; only `true` enables the listener.
- `ACCOUNT_HTTPS_LISTEN_ADDR`: defaults to `0.0.0.0:443`; must be unused.
- `ACCOUNT_HTTPS_CERTIFICATE_PATH`: absolute PEM chain path.
- `ACCOUNT_HTTPS_PRIVATE_KEY_PATH`: absolute PEM private key path.

TLS 1.3 and HTTP/1.1 terminate in Rust. The PEM pair reloads every 60 seconds;
invalid renewal material retains the previous pair and emits a non-secret warning.
Explicitly enabled invalid configuration fails startup instead of silently serving
credentials over HTTP. Legacy HTTP account clients remain unchanged by this work;
the merchant child must use only this HTTPS origin.

## Published contract

GET: `/health`, `/api/me`, `/api/auth/security`.

POST: `/api/auth/login`, `/api/auth/register`, `/api/auth/logout`,
`/api/auth/password/recover`, `/api/auth/recovery-codes/rotate`.

PUT: `/api/auth/password`.

All other paths, query strings and methods return 404. Responses are not cached.
There is no browser CORS grant: the merchant backend makes these requests. Bodies
are capped at 16 KiB; connections at 128; handshake time at 10 seconds and connection
lifetime at 60 seconds. Writes use existing bounded process-local abuse storage,
60 per socket peer/minute and 240 globally/minute, independent of forwarded headers.
This is a single-process guard, not a distributed rate-limit claim.

## Production rollout

Use the normal `publish-server.ps1 -SkipPcFrontend` workflow for committed code.
The configuration script is a separate, bounded operator action:

```sh
bash scripts/configure-account-https.sh prepare 43.139.149.158
bash scripts/configure-account-https.sh activate 43.139.149.158
bash scripts/configure-account-https.sh verify 43.139.149.158
```

`prepare` requires public inbound TCP 80 and 443 in host/cloud firewalls. It installs
pinned Certbot into a dedicated venv only when absent, issues a short-lived IP
certificate, and installs a six-hour renewal timer. It never restarts the backend.
Standalone ACME temporarily needs port 80 free on each renewal. Monitor timer failure
and certificate expiry; do not claim renewal is verified until a dry run succeeds.

`activate` expects the new binary already deployed, validates certificate lifetime,
atomically updates only the four account TLS settings and restarts the service.
On failed health or API-surface checks it restores the previous environment and
restarts the old configuration. It does not roll back a binary release.

To disable, set `ACCOUNT_HTTPS_ENABLED=false` and restart `elon-server`. This
breaks child sign-in until restored; existing main HTTP clients are unaffected.

Before enabling merchant authentication, verify the HTTPS contract from both a PC
and the merchant server, use an isolated account for registration/login/password
tests, and review old APK compatibility with same-origin cookie/CSRF enforcement.
Do not reset an actual owner's password to test this integration.

## Focused validation

Run `scripts/validate-rust.ps1 -- test --manifest-path server/tests/account-https-harness/Cargo.toml --locked`
through the logged-command runner. The harness imports production configuration,
policy and abuse-store modules directly. Nine tests passed on 2026-09-05, including
body limits, route isolation, required socket identity and spoofed forwarding headers.
The full server test target exceeded the no-output timeout during compilation; that
is not a failed assertion or a completed full-suite run. The production binary still
requires normal cargo check and the guarded release build.
