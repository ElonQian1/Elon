#!/usr/bin/env python3
"""
纯 stdlib WebSocket 测试 - 诊断 /ws/voice/transcribe 链路
无需任何第三方包
"""
import socket, hashlib, base64, struct, json, math, sys, time, threading

HOST = "localhost"
PORT = 8080
PATH = "/ws/voice/transcribe"
USER_ID = sys.argv[1] if len(sys.argv) > 1 else "diag-001"
PROJECT_ID = sys.argv[2] if len(sys.argv) > 2 else None

def ws_key():
    import os
    return base64.b64encode(os.urandom(16)).decode()

def ws_accept(key):
    magic = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
    return base64.b64encode(hashlib.sha1((key + magic).encode()).digest()).decode()

def send_frame(sock, payload, opcode=1, mask=True):
    """发送 WebSocket 帧"""
    import os
    if isinstance(payload, str):
        payload = payload.encode()
    length = len(payload)
    header = bytes([0x80 | opcode])
    if length < 126:
        header += bytes([0x80 | length] if mask else [length])
    elif length < 65536:
        header += bytes([0x80 | 126, length >> 8, length & 0xff] if mask else [126, length >> 8, length & 0xff])
    else:
        header += bytes([0x80 | 127] if mask else [127]) + struct.pack(">Q", length)
    if mask:
        masking_key = os.urandom(4)
        masked = bytes([payload[i] ^ masking_key[i % 4] for i in range(len(payload))])
        sock.sendall(header + masking_key + masked)
    else:
        sock.sendall(header + payload)

def recv_frame(sock):
    """接收 WebSocket 帧，返回 (opcode, payload_bytes)"""
    def recv_exact(n):
        buf = b""
        while len(buf) < n:
            chunk = sock.recv(n - len(buf))
            if not chunk:
                raise ConnectionError("连接断开")
            buf += chunk
        return buf

    b0, b1 = recv_exact(2)
    opcode = b0 & 0x0f
    masked = (b1 & 0x80) != 0
    length = b1 & 0x7f
    if length == 126:
        length = struct.unpack(">H", recv_exact(2))[0]
    elif length == 127:
        length = struct.unpack(">Q", recv_exact(8))[0]
    masking_key = recv_exact(4) if masked else b""
    payload = recv_exact(length)
    if masked:
        payload = bytes([payload[i] ^ masking_key[i % 4] for i in range(len(payload))])
    return opcode, payload

# ── 建立 TCP 连接 ──
print(f"[1] 连接 {HOST}:{PORT}...")
sock = socket.create_connection((HOST, PORT), timeout=10)
sock.settimeout(10)

# ── WebSocket 握手 ──
key = ws_key()
handshake = (
    f"GET {PATH} HTTP/1.1\r\n"
    f"Host: {HOST}:{PORT}\r\n"
    f"Upgrade: websocket\r\n"
    f"Connection: Upgrade\r\n"
    f"Sec-WebSocket-Key: {key}\r\n"
    f"Sec-WebSocket-Version: 13\r\n"
    f"\r\n"
)
sock.sendall(handshake.encode())
response = b""
while b"\r\n\r\n" not in response:
    response += sock.recv(1024)
print("[2] HTTP 升级响应:")
first_line = response.decode(errors="replace").split("\r\n")[0]
print("   ", first_line)
if "101" not in first_line:
    print("❌ WebSocket 升级失败")
    sock.close()
    sys.exit(1)
print("   ✅ WebSocket 已建立")

# ── 发 hello ──
hello = {"type": "hello", "user_id": USER_ID, "sample_rate": 24000, "channels": 1}
if PROJECT_ID:
    hello["project_id"] = PROJECT_ID
send_frame(sock, json.dumps(hello), opcode=1)
print(f"[3] 已发 hello (user_id={USER_ID})")

# ── 读取服务端回复（最多等 12 秒）──
sock.settimeout(12)
print("[4] 等待服务端响应（最多12秒）...")
try:
    for _ in range(5):
        opcode, payload = recv_frame(sock)
        if opcode == 8:
            print("[关闭帧]", payload[:50])
            break
        text = payload.decode(errors="replace")
        print(f"[服务端] {text}")
        try:
            d = json.loads(text)
            t = d.get("type", "")
            if t == "ready":
                print("   ✅ 服务端已就绪，开始发 PCM...")
                # 生成 0.5s 440Hz PCM16 LE @24kHz
                n = 12000
                samples = [int(2000 * math.sin(2 * math.pi * 440 * i / 24000)) for i in range(n)]
                pcm = struct.pack("<" + "h" * n, *samples)
                send_frame(sock, pcm, opcode=2)  # binary frame
                time.sleep(0.1)
                send_frame(sock, json.dumps({"type": "commit"}), opcode=1)
                print("   已发 PCM(0.5s) + commit，等待转写结果...")
                sock.settimeout(15)
            elif t == "error":
                print(f"   ❌ 服务端错误: code={d.get('code')} msg={d.get('message')}")
                break
            elif t in ("transcript_delta", "transcript_final"):
                print(f"   ✅ 转写结果: {d}")
                break
            elif t in ("cli_dispatched", "ai_done", "ai_error"):
                print(f"   ✅ AI链路: {d}")
                break
        except json.JSONDecodeError:
            pass
except socket.timeout:
    print("   ⏱ 等待超时")
except ConnectionError as e:
    print(f"   ❌ 连接断开: {e}")
finally:
    sock.close()

print("\n=== 测试完成 ===")
