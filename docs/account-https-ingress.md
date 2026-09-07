# Native account HTTPS ingress

This optional Rust listener lets first-party child backends validate main accounts
over trusted HTTPS. It does not publish the general main API, install Nginx, change
the existing 8080 service, or copy the account/password database.

## Configuration

- `ACCOUNT_HTTPS_ENABLED`: defaults off; only `true` enables the listener.
- `ACCOUNT_HTTPS_LISTEN_ADDR`: defaults to `0.0.0.0:443`; supports another nonzero
  port, including 8443. A new listener must not conflict with another service.
- `ACCOUNT_HTTPS_CERTIFICATE_PATH`: absolute PEM chain path.
- `ACCOUNT_HTTPS_PRIVATE_KEY_PATH`: absolute PEM private key path.

TLS 1.3 and HTTP/1.1 terminate in Rust. The PEM pair reloads every 60 seconds;
invalid renewal material retains the previous pair and emits a non-secret warning.
Explicitly enabled invalid configuration fails startup instead of silently serving
credentials over HTTP. Legacy HTTP account clients remain unchanged by this work;
the merchant child must use only this HTTPS origin.

## Published contract

GET: `/health`, `/api/me`, `/api/auth/security`, `/api/me/asset-access/grants`,
`/api/asset-access/me`, `/api/asset-access/grids`.

POST: `/api/auth/login`, `/api/auth/register`, `/api/auth/logout`,
`/api/auth/password/recover`, `/api/auth/recovery-codes/rotate`,
`/api/me/asset-access/authorize`, `/api/asset-access/token`, `/api/asset-access/revoke`,
`/api/node/private-read-projections`, and owner revocation at
`/api/me/asset-access/grants/aag_<32 lowercase hex>/revoke`.

PUT: `/api/auth/password`.

All other paths, query strings and methods return 404. Responses are not cached.
There is no browser CORS grant: the merchant backend makes these requests. Bodies
are capped at 16 KiB except the authenticated node projection upload (256 KiB);
connections at 128; handshake time at 10 seconds and connection
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

The two scripts `configure-account-https.sh` and `account_https_target.py` must be
deployed together from the same committed revision. `prepare` requires public inbound
TCP 80 for HTTP-01 issuance and renewal; the business TLS listener can use another
reachable port. It installs
pinned Certbot into a dedicated venv only when absent, issues a short-lived IP
certificate, and installs a six-hour renewal timer. It never restarts the backend.
Standalone ACME temporarily needs port 80 free on each renewal. Monitor timer failure
and certificate expiry; do not claim renewal is verified until a dry run succeeds.

`activate` expects the new binary already deployed, validates certificate lifetime,
system trust, target host/IP identity and matching private key,
atomically updates only the four account TLS settings and restarts the service.
On failed health or API-surface checks it restores the previous environment and
restarts the old configuration. It does not roll back a binary release.

### Custom port or externally managed certificate

Certificate validation and the business port are separate choices. HTTPS does not
require port 443. This deployment entry supports IPv4 or an ASCII DNS name and an
explicit port; the DNS form requires a supplied certificate chain/key pair. It does
not create a domain, modify DNS, implement a DNS provider, or issue an external
certificate. A DNS-01 provider can obtain a domain certificate without inbound
80/443; the supplied IP certificate must instead use its issuer's supported
validation. See [Let's Encrypt challenge types](https://letsencrypt.org/docs/challenge-types/).

```sh
# Pure plan: no file reads, sockets, installation, issuance or runtime changes.
bash scripts/configure-account-https.sh plan https://43.139.149.158:8443
bash scripts/configure-account-https.sh plan https://account.example.com:9443 \
  --certificate-path /etc/elon-account/fullchain.pem \
  --private-key-path /etc/elon-account/privkey.pem

# Check a provider-managed pair locally before activation; no service restart.
bash scripts/configure-account-https.sh check https://account.example.com:9443 \
  --certificate-path /etc/elon-account/fullchain.pem \
  --private-key-path /etc/elon-account/privkey.pem
```

`account.example.com` and the paths above are examples, not configured resources.
External paths must be absolute ASCII POSIX paths using letters, digits, `/`, `.`,
`_` or `-`, so dotenv does not expand or escape their contents. The private key
must be an unencrypted PEM supported by the native TLS loader, stored with access
limited to the service owner; encrypted PEM, even with an empty password, is rejected.
Use the same target and pair with `activate`, followed by `verify` from a client
network. `plan` validates syntax only; `check` does not establish public reachability
or APK acceptance. External renewal remains with the provider and must preserve
the configured paths; this script does not install an IP renewal timer for it.

The script protects the legacy 8080 port and rejects conflicts with configured
HTTP/direct TLS listeners or other processes. Reconfiguring an existing account
TLS listener requires the running `elon-server` process and verified account health.
There is no HTTP/TLS multiplexing: replacing HTTP 8080 with TLS would break old
HTTP/WS clients and is outside this deployment change.

Update both APK builds' `ELON_ASSET_ACCESS_ORIGIN` and the Win node's
`ELON_PRIVATE_READ_HTTPS_ORIGIN` to the same exact HTTPS origin including its port.
No Origin is added to native requests. A browser Origin still requires the separate
exact `PUBLIC_URL` HTTPS match; do not change the legacy global public URL merely
to configure this native channel.

### IP certificate with port 80 closed

`install-account-acme.sh` installs the checksum-pinned official lego v5.4.1 client
and the matching operator scripts. It accepts `--archive /absolute/archive.tar.gz`
for an already downloaded official archive, still checking its fixed SHA-256 before
extraction. Installation does not request a certificate or restart the main service.

The independent `configure-account-acme.sh` provider uses TLS-ALPN-01 on TCP 443
and leaves TCP 8443 to the existing native HTTPS listener. It never enables port 80,
uses no contact email, and keeps staging and production accounts/certificates in
separate root-only directories. See the [lego v5.4.1 release](https://github.com/go-acme/lego/releases/tag/v5.4.1).

```sh
bash scripts/configure-account-acme.sh plan 43.139.149.158
sudo bash scripts/install-account-acme.sh --archive /absolute/lego-v5.4.1.tar.gz
sudo bash /opt/elon-account-acme/scripts/configure-account-acme.sh staging 43.139.149.158
sudo bash /opt/elon-account-acme/scripts/configure-account-acme.sh issue 43.139.149.158
sudo bash /opt/elon-account-acme/scripts/configure-account-acme.sh install-timer 43.139.149.158
sudo bash /opt/elon-account-acme/scripts/configure-account-https.sh activate https://43.139.149.158:8443 \
  --certificate-path /var/lib/elon-account-acme/production/certificates/43.139.149.158.crt \
  --private-key-path /var/lib/elon-account-acme/production/certificates/43.139.149.158.key
```

Staging proves IP/ALPN issuance and lifetime only; its certificate must never be
activated for users. Production checks reuse the existing trust/identity/key gates.
The systemd timer checks every six hours with up to ten minutes of random delay;
lego v5 repeats `run` and uses its default ARI/dynamic renewal policy. Internal
random sleeping is disabled because the timer supplies the delay. Each operation
uses a shared file lock and a five-minute timeout, with no restart of the main
8443 service during renewal. Existing Certbot state and timers remain separate.

The operator scripts must be committed and deployed together. Keep the 443 ingress
available for future challenges and monitor the timer's actual result; installing
the timer is not evidence that a future scheduled renewal has succeeded.

To disable, set `ACCOUNT_HTTPS_ENABLED=false` and restart `elon-server`. This
breaks child sign-in until restored; existing main HTTP clients are unaffected.

Before enabling merchant authentication, verify the HTTPS contract from both a PC
and the merchant server, use an isolated account for registration/login/password
tests, and review old APK compatibility with same-origin cookie/CSRF enforcement.
Do not reset an actual owner's password to test this integration.

## Focused validation

The target parser and guarded deployment entry have offline tests:

```sh
python3 scripts/test_account_https_target.py -v
bash scripts/test-account-https-config.sh
bash scripts/test-account-acme.sh
```

They cover custom ports, external pairs, strict origins and certificate checks for
trust, host/IP, expiry and key mismatch. Certificates are synthetic and live only
in the test's owned temporary directory; no production service is changed.

Run `scripts/validate-rust.ps1 -- test --manifest-path server/tests/account-https-harness/Cargo.toml --locked`
through the logged-command runner. The harness imports production configuration,
policy and abuse-store modules directly. Nine tests passed on 2026-09-05, including
body limits, route isolation, required socket identity and spoofed forwarding headers.
The full server test target exceeded the no-output timeout during compilation; that
is not a failed assertion or a completed full-suite run. The production binary still
requires normal cargo check and the guarded release build.
