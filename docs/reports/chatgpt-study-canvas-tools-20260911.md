# Study And Canvas Tools

## Scope And Evidence

This batch extends the existing private composer tool runtime and production
native Tools menu. It does not introduce another sender, controller, polling
loop, or a DOM menu click. Google, deep research, agents and connectors are not
part of this change. Canvas selection and document generation are distinct from
full native Canvas editing and version management.

Current public assets retained in `runtime-20260911-b`:

| Asset | SHA-256 |
|---|---|
| shared `4813494d-c6b4nsqqwi13e6rd.js` | `41e6e38589707e7c6ff5d181afa2193dda17f605dfe256224af09e6c85cfa096` |
| composer `8b34dbc2-fpy4mlfnxc115y6k.js` | `c4b74136b4efd5255fd91e9c0f2f9c382212ffc28f5ecee7aaa564ed05e36cec` |

The shared enum maps Tatertot to `tatertot` and Canvas to `canvas`. The composer
menu `Y_n` delegates ordinary hints to `Mz`, implemented by `BHt`. It updates
the same `Bz` controller signal with previous-hint and locked-state checks.
The existing versioned bindings expose these as `Bg` and `Ng`; this batch does
not freeze current minified export names. Local actions and connector/agent
branches have different callbacks and are deliberately excluded.

The installed 1670 baseline failed the bounded study-tool lookup before send.
Its private catalog and native enum only admitted Search and Image Generation.
Original conversation and awake policy were restored. This was an adapter
coverage gap, not proof that the website lacked Study.

## Implementation

- Private tool runtime v3 admits the two additional ordinary hints only when
  the current official filtered menu confirms them. Model, account, route,
  controller, disabled/upsell/hidden/local-action and stale-binding guards stay.
- Existing native menu presets now include Study and Canvas. The same selected
  chip, clear action, cache and command receipt are reused. Explicit semantics
  take priority over labels; filenames and deep research cannot impersonate them.
- Adapter 354 loads this version. There is no new endpoint, token export or
  second state owner. Existing private Search/Image behavior is unchanged.
- Web mirror checked: its provider entry opens the external official page and
  does not own the Android on-device private controller. No fake PWA controls or
  independent PWA implementation were added; existing APK menu/chip styling is
  reused without changing shared layout, colors or dimensions.

## Verification

Five targeted Node suites: **78 passed**, including private hint selection,
switch/clear/idempotence, model drift and each official admission restriction.
Android Release production/test sources compiled. Native quick-action tests:
**10 passed**. In the broader 17-case selection, two old source-position checks
failed: latency contract expects removed `SUCCESS_COOLDOWN_MS`; composer
contract looks for assets in PageAdapter after extraction to AdapterAssets.
Both mismatches exist in base `efcd7f708`; they were not relaxed to pass this
batch. The other 15 selected tests passed.

`smoke-chatgpt-web-extended-tools.ps1` uses the actual production native menu,
selected chip, send and clear buttons. MCP supplies structured receipts and
an isolated synthetic draft. Acceptance requires private acknowledgements,
a matching user-turn anchor, completed native/official reply and restoration.
Canvas additionally requires artifact/interactive/code output, not a plain
text claim. It never exports conversation text or headers.

## Normal Release 1671

Source `7e6648af6`, adapter 354, normal **1.1.1671 (1671)** was published.
The Release build passed; remote APK hash and size were verified:
`8ab1081abac09674f0adf659a2a6ccccfe260b86565c39ce8e0c8e08a27a1acb`,
40,170,318 bytes. Focused Android tests passed again (10/10); the external
semantic native UI runner also compiled successfully. No Debug APK is used.

The publish command returned failure only after successful server publication,
when automatic wireless installation timed out. Xiaomi became offline during
the build; USB was not enumerated. A discovered TLS wireless endpoint also
timed out. No 1671 installation or new-tool device result is claimed. Publisher
cleanup additionally reported its existing missing `Branch` property warning;
the task still uses the required explicit finish workflow.

| Capability ID | Code | Default | Device | Completed |
|---|---|---|---|---|
| `android_chatgpt_private_study_tool_v1` | implemented | native preset, official admission | deferred | false |
| `android_chatgpt_private_canvas_tool_v1` | implemented | native preset, official admission | deferred | false |

Reuse this published artifact on reconnect, then run the native tool acceptance
script. Do not rebuild or repeat protocol research merely because the device
left. Full Canvas editing, other account/model tool eligibility and server
preference persistence are not established by the offline tests.
