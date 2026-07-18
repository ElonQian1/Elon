#!/usr/bin/env bash

elon_release_json_field() {
  JSON_INPUT="$1" python3 - "$2" <<'PY'
import json, os, sys
value = json.loads(os.environ.get("JSON_INPUT", "{}"))
for part in sys.argv[1].split("."):
    value = value.get(part) if isinstance(value, dict) else None
if isinstance(value, bool): print("true" if value else "false")
elif value is not None: print(value)
PY
}

elon_release_post() {
  curl --noproxy '*' -sS --fail --max-time 30 \
    -H 'Content-Type: application/json' -X POST -d "$2" "$1/$3"
}

release_post() {
  elon_release_post "$RELEASE_API_BASE" "${2:-}" "$1"
}

call_release_api() {
  elon_release_post "$RELEASE_API_BASE" "${2:-}" "$1"
}

elon_release_json_object() {
  JSON_INPUT="$1" python3 - "$2" <<'PY'
import json, os, sys
value = json.loads(os.environ.get("JSON_INPUT", "{}"))
for part in sys.argv[1].split("."):
    value = value.get(part) if isinstance(value, dict) else None
print(json.dumps(value or {}, separators=(",", ":")))
PY
}

wait_global_publish_lease() {
  local claim_json="$1" kind="$2" base="$3" action token position heartbeat status_json success
  action=$(elon_release_json_field "$claim_json" action)
  [[ "$action" == "coalesced" ]] && { printf '%s\n' "$claim_json"; return 0; }
  token=$(elon_release_json_field "$claim_json" token)
  while [[ "$action" == "wait" ]]; do
    position=$(elon_release_json_field "$claim_json" queuePosition)
    echo "   global publish lease waiting (FIFO ${position:-0}); heartbeat active" >&2
    sleep 5
    heartbeat=$(printf '{"kind":"%s","token":"%s","leaseSecs":3600}' "$kind" "$token")
    elon_release_post "$base" "$heartbeat" heartbeat >/dev/null
    status_json=$(curl --noproxy '*' -sS --fail --max-time 20 "$base/status?token=$token")
    action=$(elon_release_json_field "$status_json" tokenStatus.action)
    case "$action" in
      build|wait) claim_json="$status_json" ;;
      coalesced) printf '%s\n' "$status_json"; return 0 ;;
      finished)
        success=$(elon_release_json_field "$status_json" tokenStatus.success)
        [[ "$success" == "true" ]] && { printf '%s\n' "$status_json"; return 0; }
        echo "queued publish failed: $(elon_release_json_field "$status_json" tokenStatus.errorMessage)" >&2
        return 1 ;;
      *) echo "publish lease became invalid: $status_json" >&2; return 1 ;;
    esac
  done
  printf '%s\n' "$claim_json"
}

enter_global_publish_lease() {
  local claim action success
  claim=$(wait_global_publish_lease "$1" "$2" "$3") || return 1
  action=$(elon_release_json_field "$claim" action)
  [[ -z "$action" ]] && action=$(elon_release_json_field "$claim" tokenStatus.action)
  if [[ "$action" == "coalesced" || "$action" == "finished" ]]; then
    success=$(elon_release_json_field "$claim" tokenStatus.success)
    [[ "$action" == "coalesced" || "$success" == "true" ]] || return 1
    echo "   same SHA already published; coalesced without rebuilding" >&2
    return 0
  fi
  [[ "$action" == "build" ]] || { echo "global publish lease was not granted: $claim" >&2; return 1; }
  [[ -n "$(elon_release_json_field "$claim" action)" ]] || claim=$(elon_release_json_object "$claim" tokenStatus)
  printf '%s\n' "$claim"
}
