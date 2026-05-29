#!/usr/bin/env python3
"""
诊断 /ws/voice/transcribe 端到端链路。
用法: python3 test-voice-chain.py <user_id> [project_id] [conversation_id]
"""
import sys, json, struct, math, threading, time

try:
    import websocket
except ImportError:
    print("缺少 websocket-client，正在安装...")
    import subprocess
    subprocess.run([sys.executable, "-m", "pip", "install", "websocket-client", "-q"])
    import websocket

SERVER = "ws://localhost:8080/ws/voice/transcribe"
USER_ID = sys.argv[1] if len(sys.argv) > 1 else "diag-test-001"
PROJECT_ID = sys.argv[2] if len(sys.argv) > 2 else None
CONVERSATION_ID = sys.argv[3] if len(sys.argv) > 3 else None

print(f"[TEST] 连接 {SERVER}")
print(f"[TEST] user_id={USER_ID}, project_id={PROJECT_ID}")

done = threading.Event()
result = {"steps": []}

def on_open(ws):
    hello = {"type": "hello", "user_id": USER_ID, "sample_rate": 24000, "channels": 1}
    if PROJECT_ID:
        hello["project_id"] = PROJECT_ID
    if CONVERSATION_ID:
        hello["conversation_id"] = CONVERSATION_ID
    ws.send(json.dumps(hello))
    result["steps"].append("sent:hello")
    print("[1] 已发送 hello")

def on_message(ws, msg):
    print(f"[服务端] {msg}")
    result["steps"].append(f"recv:{msg[:120]}")
    try:
        d = json.loads(msg)
        t = d.get("type", "")
        if t == "ready":
            # 发 0.3s 440Hz 正弦波 PCM16 LE @24000Hz
            n = 7200  # 0.3s * 24000
            samples = [int(2000 * math.sin(2 * math.pi * 440 * i / 24000)) for i in range(n)]
            pcm = struct.pack("<" + "h" * n, *samples)
            ws.send_binary(pcm)
            result["steps"].append("sent:pcm_0.3s")
            print("[2] 已发送 PCM 音频 (0.3s)")
            time.sleep(0.2)
            ws.send(json.dumps({"type": "commit"}))
            result["steps"].append("sent:commit")
            print("[3] 已发送 commit，等待转写结果...")
        elif t in ("transcript_delta", "transcript_final", "transcript"):
            print(f"[转写结果] {d}")
            result["transcript"] = d.get("text", d.get("delta", ""))
        elif t in ("cli_dispatched", "ai_progress", "ai_done", "ai_error"):
            print(f"[AI链路] {d}")
            result["ai"] = d
            done.set()
            ws.close()
        elif t == "error":
            print(f"[服务端错误] code={d.get('code')} msg={d.get('message')}")
            result["error"] = d
            done.set()
            ws.close()
    except Exception as e:
        print(f"[解析异常] {e}")

def on_error(ws, err):
    print(f"[WS错误] {err}")
    result["ws_error"] = str(err)
    done.set()

def on_close(ws, code, msg):
    print(f"[关闭] code={code} msg={msg}")
    done.set()

ws = websocket.WebSocketApp(SERVER,
    on_open=on_open, on_message=on_message,
    on_error=on_error, on_close=on_close)

t = threading.Thread(target=lambda: ws.run_forever())
t.daemon = True
t.start()

done.wait(timeout=15)
print("\n=== 诊断结果 ===")
print("步骤:", " → ".join(result["steps"]))
if "error" in result:
    print("❌ 服务端错误:", result["error"])
elif "transcript" in result:
    print("✅ 转写成功:", result["transcript"])
elif "ai" in result:
    print("✅ AI链路响应:", result["ai"])
else:
    print("⏱ 超时，最后步骤:", result["steps"][-1] if result["steps"] else "无")
