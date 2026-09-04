# ChatGPT private conversation metadata mutations

## Capability

- ID: `android_chatgpt_private_conversation_metadata_mutations_v1`
- Status: completed and production-default
- Provider: ChatGPT Web
- Adapter: `244`
- Device acceptance: APK `v1.1.1506 (1506)`
- Formal release: APK `v1.1.1510 (1510)`, source `fb3e29ca8`

## Product behavior

Pin or unpin, rename, and archive or unarchive run from the native conversation UI without
opening or polling the visible official menu. The current-conversation entry and directory
rows resolve the same stable conversation identity and use one coordinator. The official
conversation options remain available after an explicit user choice for repair and for
sharing, files, delete, or other unsupported actions.

## Transaction contract

The persistent identity WebView keeps page-local authorization and issues one same-origin
`PATCH` for one explicit mutation. Android never receives or persists cookies, authorization
headers, or runtime tokens. Only one write may be active, and a timeout or ambiguous receipt
starts bounded read-only reconciliation instead of replaying the write. Repeated transport
failures open a short circuit and route the next explicit action to the official fallback.

Confirmed pin and title values temporarily override stale directory responses. Confirmed
archive installs an explicit bounded tombstone so the row disappears immediately and cannot
be reintroduced by a stale cache while the authoritative directory refresh is pending.

## Verification

Targeted JavaScript and Kotlin transaction, directory, protocol, MCP, production-action, and
capability-catalog tests passed together with the Release build. On the Xiaomi device, a
dedicated archived test conversation was unarchived, renamed, restored to its original title,
and archived again. Every mutation received a confirmed receipt; the final archive produced
the expected native directory tombstone. The original title and archived state were restored,
the app returned to conversation home, and no message was sent. Cookies and app data were not
cleared. The formal APK was then installed as a data-preserving upgrade; MCP confirmed adapter
`244`, a ready authenticated bridge and composer, and restored the app to conversation home.
