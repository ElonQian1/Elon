# Browser research test harness

This independent Cargo workspace imports the production research queue, command contract,
HTTP handlers and MCP dispatcher with `#[path]`. It does not copy their implementation.
The runtime and MCP request containers include only the fields used by those modules.

The tests exercise real Axum routes in memory, including malformed and oversized bodies,
claims, receipt replay, credential rejection and project-bound MCP access. Existing queue
unit tests are imported unchanged.

The harness also imports the production scoped conversation-reader launch module.
Its reduced launch container tests explicit link grants, immutable assets and Claude
MCP configuration merging without launching Claude or reading an account.

Run from the repository root through the managed validation entry:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/validate-rust.ps1 -- test --manifest-path server/tests/browser-research-harness/Cargo.toml --lib
```

This separates these tests from unrelated `elon-pc-node` test-only modules. A successful
run does not validate the full node executable, descriptor authentication middleware,
the React bridge, WebView2 or a real website. Those require separate acceptance evidence.
