import asyncio
import os
from typing import Any

import edge_tts
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse, Response


app = FastAPI(title="Elon TTS Worker", version="0.1.0")

WORKER_TOKEN = os.getenv("ELON_TTS_WORKER_TOKEN", "").strip()
MAX_TEXT_CHARS = int(os.getenv("ELON_TTS_WORKER_MAX_TEXT_CHARS", "600"))
DEFAULT_VOICE = os.getenv("ELON_TTS_EDGE_DEFAULT_VOICE", "zh-CN-XiaoxiaoNeural")
ALLOW_CROSS_VOICE_FALLBACK = (
    os.getenv("ELON_TTS_EDGE_ALLOW_CROSS_VOICE_FALLBACK", "").strip().lower()
    in {"1", "true", "on", "yes"}
)

PRIMARY_VOICES = {
    "female_warm": "zh-CN-XiaoxiaoNeural",
    "female_bright": "zh-CN-XiaoyiNeural",
    "female_mature": "zh-CN-XiaoxuanNeural",
    "female_cool": "zh-CN-shaanxi-XiaoniNeural",
    "female_sweet": "zh-CN-liaoning-XiaobeiNeural",
}

VOICE_FALLBACKS = {
    "female_warm": ["zh-CN-XiaoyiNeural"],
    "female_bright": ["zh-CN-XiaoxiaoNeural"],
    "female_mature": ["zh-CN-XiaoxiaoNeural"],
    "female_cool": ["zh-CN-XiaoxuanNeural", "zh-CN-XiaoxiaoNeural"],
    "female_sweet": ["zh-CN-XiaoyiNeural", "zh-CN-XiaoxiaoNeural"],
}

EMOTION_TUNING = {
    "normal": {"rate": 0, "pitch": 0, "volume": 0},
    "gentle_comfort": {"rate": -8, "pitch": -2, "volume": -2},
    "wronged_crying": {"rate": -12, "pitch": -4, "volume": -5},
    "happy_sweet": {"rate": 8, "pitch": 7, "volume": 2},
    "excited_burst": {"rate": 16, "pitch": 10, "volume": 4},
    "angry_repressed": {"rate": -2, "pitch": -8, "volume": 2},
    "cool_detached": {"rate": -4, "pitch": -7, "volume": -1},
    "shy_nervous": {"rate": -5, "pitch": 4, "volume": -2},
    "sad_low": {"rate": -13, "pitch": -8, "volume": -4},
    "surprised_excited": {"rate": 12, "pitch": 9, "volume": 3},
    "crying_broken": {"rate": -18, "pitch": -5, "volume": -6},
    "serious_encourage": {"rate": -2, "pitch": -2, "volume": 1},
    "whisper_low": {"rate": -18, "pitch": -10, "volume": -14},
}


@app.get("/health")
async def health() -> dict[str, Any]:
    return {
        "ok": True,
        "engine": "edge-tts",
        "defaultVoice": DEFAULT_VOICE,
        "allowCrossVoiceFallback": ALLOW_CROSS_VOICE_FALLBACK,
        "primaryVoices": PRIMARY_VOICES,
        "maxTextChars": MAX_TEXT_CHARS,
    }


@app.post("/synthesize")
async def synthesize(request: Request) -> Response:
    assert_authorized(request)
    payload = await request.json()
    text = str(payload.get("text") or "").strip()
    if not text:
        raise HTTPException(status_code=400, detail="text is required")
    if len(text) > MAX_TEXT_CHARS:
        raise HTTPException(status_code=400, detail=f"text exceeds {MAX_TEXT_CHARS} chars")

    voice_id = str(payload.get("voiceId") or "female_warm").strip()
    emotion_id = str(payload.get("emotionId") or "normal").strip()
    speed = parse_float(payload.get("speed"), 1.0)
    primary_voice = primary_voice_for(voice_id)
    candidates = voice_candidates(voice_id)
    rate, pitch, volume = prosody(emotion_id, speed)

    last_error: Exception | None = None
    for voice in candidates:
        try:
            audio = await synthesize_edge(text, voice, rate, pitch, volume)
            return Response(
                content=audio,
                media_type="audio/mpeg",
                headers={
                    "x-elon-tts-worker": "edge-tts",
                    "x-elon-tts-worker-requested-voice": voice_id,
                    "x-elon-tts-worker-voice": voice,
                    "x-elon-tts-worker-primary-voice": primary_voice,
                    "x-elon-tts-worker-fallback": str(voice != primary_voice).lower(),
                    "x-elon-tts-worker-emotion": emotion_id,
                },
            )
        except Exception as error:  # edge voice availability can differ by region.
            last_error = error

    detail = str(last_error) if last_error else "unknown edge-tts error"
    raise HTTPException(status_code=503, detail=detail)


@app.exception_handler(Exception)
async def unhandled_error(_: Request, exc: Exception) -> JSONResponse:
    return JSONResponse(status_code=500, content={"detail": str(exc)})


def assert_authorized(request: Request) -> None:
    if not WORKER_TOKEN:
        return
    expected = f"Bearer {WORKER_TOKEN}"
    if request.headers.get("authorization", "") != expected:
        raise HTTPException(status_code=401, detail="unauthorized")


def voice_candidates(voice_id: str) -> list[str]:
    candidates = [primary_voice_for(voice_id)]
    if ALLOW_CROSS_VOICE_FALLBACK:
        candidates.extend(VOICE_FALLBACKS.get(voice_id, []))
        if DEFAULT_VOICE not in candidates:
            candidates.append(DEFAULT_VOICE)
    return dedupe(candidates)


def primary_voice_for(voice_id: str) -> str:
    return PRIMARY_VOICES.get(voice_id, DEFAULT_VOICE)


def dedupe(values: list[str]) -> list[str]:
    seen: set[str] = set()
    unique: list[str] = []
    for value in values:
        if value in seen:
            continue
        seen.add(value)
        unique.append(value)
    return unique


def prosody(emotion_id: str, speed: float) -> tuple[str, str, str]:
    tuning = EMOTION_TUNING.get(emotion_id, EMOTION_TUNING["normal"])
    speed_delta = int(round((speed - 1.0) * 100))
    rate = clamp(speed_delta + int(tuning["rate"]), -35, 35)
    pitch = clamp(int(tuning["pitch"]), -18, 18)
    volume = clamp(int(tuning["volume"]), -30, 15)
    return percent(rate), hertz(pitch), percent(volume)


async def synthesize_edge(text: str, voice: str, rate: str, pitch: str, volume: str) -> bytes:
    communicate = edge_tts.Communicate(text, voice=voice, rate=rate, pitch=pitch, volume=volume)
    chunks: list[bytes] = []
    async for chunk in communicate.stream():
        if chunk.get("type") == "audio":
            chunks.append(chunk["data"])
    if not chunks:
        raise RuntimeError("edge-tts returned empty audio")
    return b"".join(chunks)


def parse_float(value: Any, fallback: float) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return fallback


def clamp(value: int, low: int, high: int) -> int:
    return max(low, min(high, value))


def percent(value: int) -> str:
    sign = "+" if value >= 0 else ""
    return f"{sign}{value}%"


def hertz(value: int) -> str:
    sign = "+" if value >= 0 else ""
    return f"{sign}{value}Hz"


if __name__ == "__main__":
    import uvicorn

    host = os.getenv("ELON_TTS_WORKER_HOST", "127.0.0.1")
    port = int(os.getenv("ELON_TTS_WORKER_PORT", "5010"))
    uvicorn.run(app, host=host, port=port)
