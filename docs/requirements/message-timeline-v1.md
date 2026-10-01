---
version_status: current
decision_status: accepted
implementation_status: pending
reviewed_at: 2026-10-01
owner: conversation-platform
---

# Unified message timeline and bounded context

## User outcome

Friend, group, personal AI and project conversations remain responsive as history grows.
History is retained at the server; opening a conversation does not download its entire history.
Windows, mobile PWA and Android consume a common versioned timeline contract, with native renderers.

## Contract

- Default page size 50, enforced maximum 100; stable timestamp and message-ID history cursors.
- Separate ordered change cursor covers insert, edit, recall and deletion, including offline gaps.
- Cursor binds to authenticated account, source and conversation; every call rechecks access.
- Initial page and change watermark share one database snapshot. Expired change cursors request reset.
- Changes are paged, deduplicated, and applied before advancing the checkpoint.
- Keep bounded message windows and byte budgets; protect pending sends and drafts separately.
- History loads preserve the visible anchor; incoming messages do not interrupt historical reading.
- Read receipts advance only to a delivered, visible message, independently of historical paging.
- Preserve attachments, quotes, revisions, AI provenance, task cards and existing message actions.
- Cache is account-scoped and bounded. Late requests cannot repopulate a signed-out account.
- Old clients retain their existing endpoints; deployment exposes the new server before new clients.

## AI context boundary

Display history and model input are independent. Existing context builders receive bounded inputs;
recent complete turns, explicit selections and authorized retrieval take priority. Original messages
remain available. Any approximate token estimate is labelled as such, never as provider usage.
Summaries require source ranges and revision invalidation; no generated summary becomes authoritative
project knowledge. Provider-hosted web conversations retain their provider-managed context semantics.

## Delivery slices

1. Durable timeline contract, access isolation, cursor paging and change recovery.
2. Friend/group integration on Windows, PWA and Android; bounded memory and incremental rendering.
3. Personal AI/project adapters and bounded context construction using existing retrieval capabilities.

## Acceptance

- Same-timestamp messages, concurrent arrivals and page boundaries have no duplication or omission.
- Reconnect catches up over multiple pages; edit/recall/delete propagate; expired cursors reset safely.
- Cross-account/cross-conversation cursors and revoked access disclose no messages.
- 1,000/10,000/100,000-row fixtures keep page size and client working set bounded.
- History navigation, streaming, quotes, attachments, pending sends and read position remain usable.
- Offline recovery, cancellation and account switches cannot apply stale results.
- Risk-matched Rust, client logic, browser/native runtime and release checks are recorded separately.

## Evidence

Implementation, runtime and release results belong in a delivery report. This requirement records
accepted scope, not a claim that implementation or performance acceptance has passed.
