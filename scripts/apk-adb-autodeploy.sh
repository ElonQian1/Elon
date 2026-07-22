#!/usr/bin/env bash

elon_apk_adb_config_path() {
  if [[ -n "${ELON_APK_ADB_TARGETS_FILE:-}" ]]; then
    printf '%s\n' "$ELON_APK_ADB_TARGETS_FILE"
  else
    printf '%s\n' "$HOME/.elon/apk-adb-targets.json"
  fi
}

elon_apk_adb_json() {
  local config="$1" expression="$2"
  python3 - "$config" "$expression" <<'PY'
import json, sys
path, expression = sys.argv[1:]
with open(path, encoding="utf-8-sig") as stream:
    data = json.load(stream)
if expression == "settings":
    print("1" if data.get("enabled", True) else "0")
    print(data.get("schemaVersion", 1))
    print(data.get("adbPath", ""))
    print(data.get("packageName", "com.elon.app"))
    print(data.get("maxAttempts", 3))
    print(data.get("retryDelaySeconds", 5))
    print("1" if data.get("launchAfterInstall", True) else "0")
elif expression == "targets":
    for target in data.get("targets", []):
        if target.get("enabled", True):
            values = [target.get("serial", ""), target.get("hardwareSerial", ""), target.get("label", "")]
            print("\t".join(str(value).replace("\t", " ").replace("\n", " ") for value in values))
PY
}

elon_resolve_adb() {
  local configured="$1" candidate
  for candidate in "${ELON_ADB_PATH:-}" "$configured" \
    "${ANDROID_HOME:+$ANDROID_HOME/platform-tools/adb}" \
    "${ANDROID_SDK_ROOT:+$ANDROID_SDK_ROOT/platform-tools/adb}"; do
    [[ -n "$candidate" && -x "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
  done
  command -v adb 2>/dev/null || return 1
}

elon_adb_target_update() {
  local adb="$1" apk="$2" expected="$3" package_name="$4" max_attempts="$5"
  local retry_delay="$6" launch_after="$7" serial="$8" hardware_serial="$9" label="${10}"
  local attempt state actual_hardware install_output package_output installed_version last_error
  [[ -n "$label" ]] || label="$serial"
  if [[ -z "$serial" || -z "$hardware_serial" ]]; then
    echo "[$label] serial 和 hardwareSerial 必须同时配置，防止安装到错误手机。" >&2
    return 1
  fi

  for ((attempt=1; attempt<=max_attempts; attempt++)); do
    if [[ "$serial" =~ ^[^:]+:[0-9]+$ ]]; then
      "$adb" connect "$serial" >/dev/null 2>&1 || true
    fi
    state=$("$adb" -s "$serial" get-state 2>&1 || true)
    if [[ "$state" != "device" ]]; then
      last_error="设备未进入 device 状态: $state"
    else
      actual_hardware=$("$adb" -s "$serial" shell getprop ro.serialno 2>&1 | tr -d '\r' || true)
      if [[ "$(printf '%s' "$actual_hardware" | tr '[:upper:]' '[:lower:]')" != \
        "$(printf '%s' "$hardware_serial" | tr '[:upper:]' '[:lower:]')" ]]; then
        last_error="硬件序列号不匹配：期望 $hardware_serial，实际 $actual_hardware"
      else
        echo "   ⬆️  [$label] 安装 Release APK（第 $attempt/$max_attempts 次）..."
        if install_output=$("$adb" -s "$serial" install -r "$apk" 2>&1) && grep -qE '^Success[[:space:]]*$' <<<"$install_output"; then
          package_output=$("$adb" -s "$serial" shell dumpsys package "$package_name" 2>&1 || true)
          installed_version=$(sed -nE 's/.*versionCode=([0-9]+).*/\1/p' <<<"$package_output" | head -n 1)
          if [[ "$installed_version" == "$expected" ]]; then
            if [[ "$launch_after" == "1" ]]; then
              "$adb" -s "$serial" shell am force-stop "$package_name" >/dev/null 2>&1 || true
              if ! "$adb" -s "$serial" shell monkey -p "$package_name" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1; then
                last_error="安装成功但自动拉起 APP 失败"
              else
                echo "   ✅ [$label] 已无人值守更新到 build $expected"
                return 0
              fi
            else
              echo "   ✅ [$label] 已无人值守更新到 build $expected"
              return 0
            fi
          else
            last_error="版本验收失败：期望 build $expected，手机实际 build ${installed_version:-unknown}"
          fi
        else
          last_error="adb install -r 未返回 Success: $install_output"
        fi
      fi
    fi
    if ((attempt < max_attempts)); then
      echo "   ⚠️  [$label] ADB 更新失败，${retry_delay}s 后自动重试: $last_error" >&2
      sleep "$retry_delay"
    fi
  done
  echo "[$label] 经过 $max_attempts 次尝试仍未完成 ADB 更新: $last_error" >&2
  return 1
}

invoke_elon_apk_adb_autodeploy() {
  local apk="$1" expected="$2" config settings enabled schema configured_adb package_name
  local max_attempts retry_delay launch_after adb targets_file failures=0 target_count=0
  config=$(elon_apk_adb_config_path)
  if [[ ! -f "$config" ]]; then
    echo "   ℹ️  未配置 ADB 自动部署，跳过：$config"
    return 0
  fi
  [[ -f "$apk" ]] || { echo "APK 不存在: $apk" >&2; return 1; }
  mapfile -t settings < <(elon_apk_adb_json "$config" settings)
  enabled="${settings[0]}"; schema="${settings[1]}"; configured_adb="${settings[2]}"
  package_name="${settings[3]}"; max_attempts="${settings[4]}"; retry_delay="${settings[5]}"; launch_after="${settings[6]}"
  [[ "$enabled" == "1" ]] || { echo "   ℹ️  ADB 自动部署已在本机配置中禁用"; return 0; }
  [[ "$schema" == "1" ]] || { echo "不支持的 ADB 配置版本: $schema" >&2; return 1; }
  ((max_attempts >= 1 && max_attempts <= 5)) || { echo "maxAttempts 必须在 1..5 之间。" >&2; return 1; }
  ((retry_delay >= 0 && retry_delay <= 60)) || { echo "retryDelaySeconds 必须在 0..60 之间。" >&2; return 1; }
  adb=$(elon_resolve_adb "$configured_adb") || { echo "ADB 未安装或不在 PATH 中。" >&2; return 1; }
  targets_file=$(mktemp)
  elon_apk_adb_json "$config" targets >"$targets_file"
  while IFS=$'\t' read -r serial hardware_serial label; do
    [[ -n "$serial$hardware_serial$label" ]] || continue
    ((target_count+=1))
    elon_adb_target_update "$adb" "$apk" "$expected" "$package_name" "$max_attempts" \
      "$retry_delay" "$launch_after" "$serial" "$hardware_serial" "$label" || failures=$((failures + 1))
  done <"$targets_file"
  rm -f "$targets_file"
  ((target_count > 0)) || { echo "ADB 自动部署已启用，但 targets 为空: $config" >&2; return 1; }
  if ((failures > 0)); then
    echo "APK_ADB_DEPLOY_STATUS=failed" >&2
    return 1
  fi
  echo "APK_ADB_DEPLOY_STATUS=updated"
}
