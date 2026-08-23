---
capability_id: android_google_web_private_response_research_v1
implementation_status: research_probe
verification_status: controlled_device_shapes_verified
production_default: false
repeat_implementation: prohibited_without_new_research_question
---

# Google Web private response research

This probe observes clones of official same-origin Google responses without
creating or replaying requests. It never reads request headers, request bodies,
cookies, or account data. Only redacted endpoint shapes, response type/size,
bounded structural counts, and an exact controlled-marker match bit may enter
research logs.

The official WebView, DOM extractor, local snapshot cache, and recovery flow
remain authoritative. The reply path graduated separately as
`android_google_web_private_reply_observer_v1`. This research tap remains disabled in
production. The directory work graduated as
`android_google_web_private_conversation_directory_v1`; retain this probe only for a new,
explicit compatibility investigation.

Controlled device evidence identified these same-origin endpoints without logging
payload values:

- `GET /async/folif`: answer completion signal used by the production reply observer.
- `GET /httpservice/web/AimThreadsService/ListThreads`: official conversation directory;
  its bounded row shape and `csuir` navigation mapping were verified and moved into the
  production passive directory observer.
