# Public runtime asset diagnostics

The existing production MCP `chatgpt_private_protocol_probe` accepts
`mode: runtime_assets`. It synchronously reads Resource Timing and modulepreload/
script URL metadata, returning `elon.private_runtime_assets.v1` with at most 96
deduplicated public `/cdn/assets/*.js` filenames and a `truncated` flag.

This is an explicit diagnostic request, not a feature detector or background
poller. It does not fetch/import modules, open a debugging socket, read inline
scripts, copy credentials, or collect conversation text. Non-official origins,
userinfo, alternate ports, query strings, fragments and other paths are rejected.
Native receipt validation independently enforces the same bounded schema.

Missing names do not establish that the provider lacks a capability: modules may
be lazy, resource entries may have been evicted, and the result may be partial.
Use positive filename evidence to investigate a failed version-pinned runtime
binding, alongside its actual command receipt and current public source.

Protocol-evidence module 2 and research-probe module 13 reuse existing assembly.
HTTP shape capture remains dormant unless explicitly started; inventory does not
activate it. Global adapter 293 is unchanged because older worktrees own pending
version-only edits; module versions and the published APK identify this change.

Verification: 51 focused Node cases plus 73 compatibility runner cases passed.
Includes no added requests/telemetry,
strict filtering, bounded results, production receipt routing and existing
shared-link guards. Android Release Kotlin/Java compilation and six focused
JUnit tests passed (two new validator tests and four existing receipt tests).
APK 1545 was published/installed and its production MCP inventory succeeded in
3,361 ms. It returned 96 filenames with `truncated=true`; neither pinned shared
nor conversation module was in that partial result. This is not proof of absence.
The phone returned to the conversation home without changing content or identity.
