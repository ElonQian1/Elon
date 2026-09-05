---
capability_id: android_chatgpt_snapshot_authoritative_handshake_v1
implementation_status: completed
production_default: true
verification_status: device_verified_v1_1_1536_adapter_259_without_manual_resume
---

# ChatGPT snapshot-authoritative handshake

The ChatGPT bridge handshake completes only after the current document emits a parsed page
snapshot. Adapter-loaded, manifest, control, and command events do not stop the bounded
reinjection chain before authentication and composer state are known.

Release `v1.1.1536 (1536)`, adapter `259`, entered the production friend-chat surface on Honor
after a data-preserving upgrade. It became ready without a Home/resume workaround and remained
ready at 5, 15, and 35 seconds. No message was sent, no conversation content was exported, and
cookies and application data were preserved.

The existing bounded official WebView session recovery remains the fallback. This capability
must not be reimplemented without current regression evidence.
