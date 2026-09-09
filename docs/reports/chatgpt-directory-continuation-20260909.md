# Directory continuation acceptance

Follow-up to `android_chatgpt_private_directory_pagination_v1` and the
[pagination checkpoint](chatgpt-directory-pagination-20260909.md).
This is the same directory capability, not another parallel implementation.

## Changes and limits

- Private directory refresh owner v4 retains only the current global paging cycle
  in memory, for at most 60 seconds. Account/token, transport or document changes,
  cancellation and expiry discard it. Successful cycles are not a new TTL cache.
- Retryable failures resume the failed page. Completed sibling collections are
  reused within that cycle; cursor/schema failures cannot become checkpoints.
- The 12-second batch admission budget no longer shortens an admitted request to
  its remaining milliseconds. Normal page deadline is 4 seconds; a timed-out
  global page retries with an 8-second ceiling, reset after page success.
- Existing bounds remain: 200 ordinary conversations, 40 project catalog entries,
  ten pages and 1 MiB per response. Global refresh never replaces all cached
  project histories. The combined native cache may therefore exceed 200 rows.
- Existing cached UI remains visible on failure. Failed partial results do not
  settle native scope ownership via an early snapshot. Explicit successful reads
  still settle even when their content has not changed.
- MCP `chatgpt_private_protocol_probe` mode `directory_refresh` reads structural
  timings from the actual private owner, including the captured original Fetch.
  It does not enable network capture. Native validation rejects identifiers,
  arbitrary fields, free text and coercion; no headers, cursors or content leave
  the page through this diagnostic. Replaced identities cannot reuse its data.

## Offline verification

64 Node runner cases passed in `directory-adaptive-deadline-node-20260909`.
They cover continuation, the wider retry deadline/reset, healthy sibling reuse,
fixed expiry, identity/document/cancel isolation, pagination, partial retention,
dispatcher completion, bounded JSON and protocol-probe wiring.

Production/test Kotlin compilation and 29 JVM cases passed in
`directory-continuation-android-20260909`: directory state, refresh coordinator,
typed diagnostic validation and the existing protocol evidence boundary.
The subsequent adaptive-deadline change only modifies JavaScript and its tests;
the final Release build also passed. Source-size and document guards passed.

## Published phone evidence

Final APK **1.1.1610 (1610)**, adapter 310, source
`2f306a5d10b5468d09f8ea3ca26b80c255130e5b`.
Published/local SHA-256:
`6d0a226b93b2126f40c9e43214f8ed7b7b305e976e781c72ece91298bbc05dea`
(40,038,781 bytes). `install -r` succeeded on the Xiaomi. Publication took
527.5 seconds including the shared release-lease wait.

The intermediate 1608 package established that identity acquisition took 0-3 ms,
while individual HTTP reads hit their approximately 4-second deadline. It also
proved a retry resumed four completed pages and skipped a completed project
catalog. This does not establish a VPN or remote-server root cause.

Final acceptance used production native sidebar/refresh controls, not raw
directory commands or the official page UI. Only diagnostic reads used the probe.

| Case | Observed result |
|---|---|
| Cached sidebar | Open in 279 ms including MCP; 200 cached rows and 19 projects |
| First observed global batch | `directory_partial`; five ordinary pages and one complete project-catalog page; owner 12,547 ms |
| Following global refresh | Timeout on the next ordinary page; resumedPages=5; zero project HTTP requests |
| Next global refresh | Three further ordinary pages, total eight; `directory_partial` with truncated=true at the configured bound; owner 10,547 ms; zero project HTTP requests |
| Project refresh | `directory_ready`; one project-content page; owner 596 ms, native receipt observed in 802 ms; drawer remained open |
| Restore | Date section, drawer closed, same conversation and unchanged empty input; no dictation/streaming; authenticated; native cache 260 rows, 19 projects, ready |

Native receipt elapsed times include polling/coalescing overhead, not isolated
latency benchmarks. Composer-ready stayed false during the global cases. No
message, microphone, file write, login reset or independent proxy operation was
performed. This is semantic UI acceptance, not a screenshot visual review.

## Remaining work

The resumable retry and structural diagnostic are implemented and device-verified.
Do not repeat these implementations. Full-account pagination is **not complete**:
older history beyond the configured ordinary-history bound is not fetched here.
Successful time-budget partial batches currently settle native refresh, so a
further caller request is needed within the continuation lifetime. Add explicit
load-more or correctly owned queued continuation instead of repeatedly extending
deadlines or treating bounded partial data as an empty/complete account.

This batch does not establish latency distributions, heat/battery improvement,
all-project completeness, or completion of the broader private-API Goal.
