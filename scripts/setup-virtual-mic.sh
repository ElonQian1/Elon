#!/usr/bin/env bash
# 在 Linux 服务器上创建一对"虚拟扬声器 + 虚拟麦克风"，
# 让 Rust 服务能把 Android 音频写入 codex_sink，再被任何选择
# codex_mic 的本地音频采集程序当成麦克风读到。
#
# 用途：方案 A（Android → Rust → pw-cat → codex_sink → codex_mic → Codex CLI）
#
# ⚠️ 注意：当前 Codex CLI Linux TUI 的 /realtime 本地采集是空实现，
#    本脚本只验证"音频管道存在"。是否能投喂 Codex 仍需 patch Codex CLI。
#    验证方法：
#      parecord -d codex_mic.monitor /tmp/probe.wav   # 录到声音
#      aplay /tmp/probe.wav

set -euo pipefail

SINK_NAME="${ELON_VOICE_SINK:-codex_sink}"
SOURCE_NAME="${ELON_VOICE_MIC:-codex_mic}"

if ! command -v pactl >/dev/null; then
    echo "需要安装 pactl：sudo apt install -y pulseaudio-utils pipewire-pulse" >&2
    exit 1
fi

# 防重复加载：先卸载旧的同名模块
pactl list short modules | awk '/module-null-sink/ {print $1}' | while read -r id; do
    pactl unload-module "$id" 2>/dev/null || true
done
pactl list short modules | awk '/module-remap-source/ {print $1}' | while read -r id; do
    pactl unload-module "$id" 2>/dev/null || true
done

pactl load-module module-null-sink \
    sink_name="$SINK_NAME" \
    sink_properties=device.description=ElonVoiceSink >/dev/null

pactl load-module module-remap-source \
    master="${SINK_NAME}.monitor" \
    source_name="$SOURCE_NAME" \
    source_properties=device.description=ElonVoiceMic >/dev/null

pactl set-default-source "$SOURCE_NAME" || true

echo "已创建虚拟设备："
pactl list short sinks   | grep -E "$SINK_NAME"   || true
pactl list short sources | grep -E "$SOURCE_NAME" || true
