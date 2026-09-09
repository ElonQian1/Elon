# Owned directory continuation

Follow-up to [the 1610 pagination acceptance](chatgpt-directory-continuation-20260909.md).

## Scope

- `android_chatgpt_directory_owned_continuation_v1`: implemented; targeted
  offline tests passed; Release/device evidence recorded below after delivery.
- Private directory refresh version 5; directory request dispatcher version 12;
  Android adapter 311. Existing private directory cache remains version 14.
- No voice, dictation, login, cookie, transport identity, or independent proxy changes.

## Changes

Successful time-budget partial reads now publish their validated rows with an
explicit continuation flag and the originating command ID. The native refresh
owner accepts completion only for the matching ID and project scope. Passive
cache snapshots cannot settle active refreshes; late native responses after
cancellation/replacement are ignored. The retained DOM mode also tags its
snapshots and fences cancelled callbacks.

The existing coordinator schedules the next batch after 250 ms, retaining the
last dispatched scope for continuation and failures. New user navigation/actions
take priority. Continuation requires increasing page progress, is limited to ten
continuations and a 60-second cycle, and does not activate for capped or
unknown-cursor results. Those cases remain partial, not a complete empty account.

The existing identity/document/transport-bound checkpoint now covers project
contents as well as global history. At most eight scopes are retained for a
fixed 60 seconds, with cancellation/context-change invalidation. Completed
cycles are discarded, not used as an indefinite freshness cache.

Native rows stay visible during continuation. Same-section index updates preserve
the existing sidebar scroll offset; changing date/project/query resets the view.

## Verification

- 69 focused Node cases passed, including request ownership, cancellation, DOM
  opt-in compatibility, bounded continuation and project-cursor retry.
- Targeted Release compilation and 42 Android tests passed in 263.7 seconds:
  refresh session/coordinator, directory cache, structural diagnostic and private
  evidence sanitizer. A metadata-only diagnostic-source correction followed;
  the final Release build also compiles that change.
- Source-size guard passed before staging; rechecked including new files before commit.
- Real-device results are not inferred from these offline tests.

## Remaining

Full-account older-history pagination remains incomplete. JavaScript ordinary
history is capped at 200 rows, projects at 40, and the native persisted history
store at 200 rows. A real load-more design must preserve overflow/page boundaries,
handle cache eviction, and render larger directories efficiently; simply
incrementing the next offset after dropping part of a page would skip records.

This batch does not establish heat/battery improvement, all-project completeness,
or completion of the broader private-API Goal. Google remains last in priority.
