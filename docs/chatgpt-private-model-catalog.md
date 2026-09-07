# ChatGPT private additional model catalog

## Status

- Capability: `android_chatgpt_private_model_catalog_state_v1`.
- Implementation: implemented for the account-provided additional model groups
  in normal ChatGPT chat. Verification: offline verified; not `completed`.
- Delivery commits: runtime exports `985ad1d87`, native option semantics
  `ccef92a78`, private catalog/controller `a47c51473`.
- Catalog module version 1; model contract/controller version 3. The previously
  pending adapter 294 increment is now packaged in release 1.1.1548. See the
  [grouped build and device evidence](reports/chatgpt-grouped-native-20260907.md).
- Android compilation and 46 selected JUnit cases passed; APK publication and
  installation are verified. Account-specific model selection is not device
  accepted: this device session currently reports guest/no runtime identity.
- This is a native-menu/page-runtime private-state integration, not an
  independent Android HTTP model selector. The background WebView still owns
  authentication and the official conversation and preference stores.

## Evidence

The September 7 public scripts already captured for the current runtime were
read and their SHA-256 hashes verified again. No logged-in page data, credentials
or private content was used to infer the contract.

| Role and public CDN filename | SHA-256 |
|---|---|
| `8b34dbc2-nhot65scqrg20d6p.js` | `36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733` |
| `conversation-small-owrec55n6vm0ekcc.js` | `7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35` |
| `4813494d-o593jrji51wy4azk.js` | `48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375` |

Base URL: `https://chatgpt.com/cdn/assets/`. The existing
[versioned runtime resolver](chatgpt-private-runtime-bindings.md) retains the
previous observed build as well; arbitrary imports or guessed exports are not
accepted.

The old composer's `bqn` and current `SJn` render the additional-model search.
Their normal-chat path is distinct from the work-mode promoted/alpha UI. Old
`nqn` / current `iJn` flatten the known model-group category options. The
default category options come from a React hook, but its underlying builder
`EKt` / `vKt` is pure and exported. The adapter uses that builder and intersects
any explicitly supplied picker options with its current catalog; it never
calls the hook or renders the component outside React.

| Role | Compatibility export | Current export | Responsibility |
|---|---|---|---|
| Conversation | `u1t` | `W1t` | Pure model category builder, including hidden filtering |
| Conversation | `l1t` | `U1t` | Account catalog group identifiers |
| Conversation | `iin` | `Jin` | Official model/privacy eligibility check |
| Shared | `$3` | `y6` | Existing official feature gate getter |
| Composer | `Ih` | `Qh` | Existing official model/preference selection action |

The official group identifiers are `alpha`, `data_campaigns`, `experiments` and
`mainline`. Membership is taken from the current account's catalog, not from
hardcoded model names. Current denial checks still decide whether an entry is
selectable. Model IDs containing `:` or `/` are supported only as exact catalog
members, never interpolated into an adapter-created request URL.

The official selector tests gate `3016847258`. When enabled, its `Jin` checks
the cached/fetched `/models/config` data and evaluates the current privacy
policy, including `history_off_approved`. This is not a second login or a
confirmation dialog. The adapter invokes this exact existing checker with a
four-second bounded wait; it does not copy Cookie into Android or reconstruct
the request. A false, unknown, failed or late result cannot authorize a write.

## Native behavior and ownership

- Advanced -> additional models stays in the existing production native popup.
  Twenty entries per page plus navigation fit the protocol's 30-option limit.
- A validated empty catalog omits the redundant additional-model entry. Missing
  modules or unknown schemas are not interpreted as provider unavailability;
  the existing official recovery reader remains available before any write.
- `model_catalog` survives the protocol parser and consumer mapper. These are
  discrete models, not an effort slider even when labels are short or contain
  words such as Fast/High. Selection uses the explicit selected flag.
- Every selection is checked against the current account, document, route,
  conversation object, committed picker, model, effort, tier, version and
  catalog/denials again after the asynchronous eligibility check.
- Concurrent clicks share one in-flight choice or replace the previous choice.
  Closing the menu, opening another menu or selecting Back cancels a pending
  choice. Late results cannot write or update the replacement menu.
- The official composer action owns model preference persistence and selection.
  The adapter does not set effort, service tier or version for this action, and
  readback checks they were preserved. No uncertain mutation is replayed through
  DOM or another transport. Local readback does not prove server persistence.
- Directory reads and paging use existing in-page data. There is no new periodic
  polling, account-wide prefetch or extra import on a warm menu visit.

## Verification and next acceptance

125 Node runner cases passed, zero failures/cancellations/skips, in the final
logged run `model-catalog-native-final-20260907-20260907-222744-140`.
The five suites cover model catalog, existing model state, restricted efforts,
runtime bindings and model-label policy. All production asset files also pass
JavaScript syntax parsing in their actual assembly catalog.

New coverage includes production composer request/select/dismiss wiring,
47-entry pagination, nested and restricted picker options, hidden/denied/missing
models, malformed/cyclic catalogs, empty-vs-unknown behavior, both runtime alias
families, duplicate clicks, privacy failure/timeout, late-result cancellation,
context races, preserved effort/tier/version and no post-write DOM replay.

The grouped Android run executed `ChatGptWebModelCatalogProtocolTest` and the
additional `WebChatModelControlPolicyTest` case among 46 passing tests. The first
run exposed a missing protocol envelope in the new test fixture; commit
`cdfe8e6b6` corrected that fixture, and the identical test selection passed.
Production parsing was not relaxed. Source-size and whitespace checks passed.

Next: after a signed-in session is confirmed, inspect the actual production
menu using installed 1.1.1548 or its successor. If the account has an eligible extra
model, select once and restore the original model/effort/tier. If its official
catalog is empty, verify the absent entry only; do not claim a model-switch
device pass or invent one. Restore the original conversation afterward.

Work-mode promoted models, independent HTTP generation, account-specific
privacy variants and asynchronous preference persistence are not proved by
this batch. Keep their separate gaps in the remaining-work matrix. Existing
audio, subtitles, dictation, read-aloud and the independent proxy are unchanged.
