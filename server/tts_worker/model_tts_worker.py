import json
import os
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse, Response

from model_tts_common import (
    WorkerFailure,
    add_model_pythonpath,
    asset_status,
    choose_provider,
    env_bool,
    normalize_provider,
    resolve_asset,
    runtime_status,
)
from model_tts_engines import synthesize_cosyvoice, synthesize_index_tts2


app = FastAPI(title="Elon Model TTS Worker", version="0.1.0")

WORKER_TOKEN = os.getenv("ELON_TTS_WORKER_TOKEN", "").strip()
MAX_TEXT_CHARS = int(os.getenv("ELON_TTS_WORKER_MAX_TEXT_CHARS", "600"))
DEFAULT_PROVIDER = os.getenv("ELON_TTS_MODEL_PROVIDER", os.getenv("ELON_TTS_PROVIDER", "auto")).strip()
ASSET_ROOT = Path(
    os.getenv(
        "ELON_TTS_ASSET_ROOT",
        str(Path(__file__).resolve().parents[1] / "assets" / "tts"),
    )
).resolve()
MODEL_FALLBACK_URL = os.getenv("ELON_TTS_MODEL_FALLBACK_URL", "").strip().rstrip("/")
ALLOW_ABSOLUTE_ASSETS = env_bool("ELON_TTS_ALLOW_ABSOLUTE_ASSETS", False)

VOICE_ASSETS = [
    "voices/female_warm_neutral.wav",
    "voices/female_bright_neutral.wav",
    "voices/female_mature_neutral.wav",
    "voices/female_cool_neutral.wav",
    "voices/female_sweet_neutral.wav",
]

EMOTION_ASSETS = [
    "emotions/female_neutral.wav",
    "emotions/female_gentle_comfort.wav",
    "emotions/female_crying_broken.wav",
    "emotions/female_happy_soft.wav",
    "emotions/female_happy_excited.wav",
    "emotions/female_angry_repressed.wav",
    "emotions/female_cool_detached.wav",
    "emotions/female_shy_nervous.wav",
    "emotions/female_sad_low.wav",
    "emotions/female_surprised.wav",
    "emotions/female_serious_encourage.wav",
    "emotions/female_whisper.wav",
]

add_model_pythonpath()


@app.get("/health")
async def health() -> dict[str, Any]:
    return {
        "ok": True,
        "engine": "model-tts",
        "defaultProvider": normalize_provider(DEFAULT_PROVIDER),
        "assetRoot": str(ASSET_ROOT),
        "fallbackUrlConfigured": bool(MODEL_FALLBACK_URL),
        "maxTextChars": MAX_TEXT_CHARS,
        "runtime": runtime_status(),
        "assets": asset_status(ASSET_ROOT, VOICE_ASSETS, EMOTION_ASSETS),
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

    provider = choose_provider(payload, DEFAULT_PROVIDER)
    try:
        voice_audio = resolve_asset(ASSET_ROOT, ALLOW_ABSOLUTE_ASSETS, payload.get("voiceAudio"), "voiceAudio", True)
        emotion_audio = resolve_asset(ASSET_ROOT, ALLOW_ABSOLUTE_ASSETS, payload.get("emotionAudio"), "emotionAudio", False)
        if provider == "index_tts2":
            audio = synthesize_index_tts2(payload, text, voice_audio, emotion_audio)
            worker_name = "index-tts2"
        elif provider == "cosyvoice3":
            audio = synthesize_cosyvoice(payload, text, voice_audio)
            worker_name = "cosyvoice3"
        elif provider == "gpt_sovits":
            raise WorkerFailure(501, "gpt_sovits worker adapter is not implemented")
        else:
            raise WorkerFailure(400, f"unsupported provider: {provider}")
    except WorkerFailure as error:
        if MODEL_FALLBACK_URL:
            return call_fallback_worker(payload, error)
        raise HTTPException(status_code=error.status_code, detail=error.detail)

    return Response(
        content=audio,
        media_type="audio/wav",
        headers=worker_headers(payload, worker_name, provider, fallback=False),
    )


@app.exception_handler(Exception)
async def unhandled_error(_: Request, exc: Exception) -> JSONResponse:
    if isinstance(exc, HTTPException):
        return JSONResponse(status_code=exc.status_code, content={"detail": exc.detail})
    return JSONResponse(status_code=500, content={"detail": str(exc)})


def assert_authorized(request: Request) -> None:
    if not WORKER_TOKEN:
        return
    expected = f"Bearer {WORKER_TOKEN}"
    if request.headers.get("authorization", "") != expected:
        raise HTTPException(status_code=401, detail="unauthorized")


def call_fallback_worker(payload: dict[str, Any], model_error: WorkerFailure) -> Response:
    url = f"{MODEL_FALLBACK_URL}/synthesize"
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})
    if WORKER_TOKEN:
        request.add_header("Authorization", f"Bearer {WORKER_TOKEN}")
    try:
        with urllib.request.urlopen(request, timeout=90) as response:
            content = response.read()
            content_type = response.headers.get("Content-Type", "audio/wav")
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise HTTPException(status_code=503, detail=f"model failed: {model_error.detail}; fallback failed: {body}") from exc
    except Exception as exc:
        raise HTTPException(status_code=503, detail=f"model failed: {model_error.detail}; fallback failed: {exc}") from exc

    headers = worker_headers(payload, "model-fallback", choose_provider(payload, DEFAULT_PROVIDER), fallback=True)
    headers["x-elon-tts-worker-model-error"] = model_error.detail[:800]
    return Response(content=content, media_type=content_type, headers=headers)


def worker_headers(payload: dict[str, Any], worker: str, provider: str, fallback: bool) -> dict[str, str]:
    return {
        "x-elon-tts-worker": worker,
        "x-elon-tts-worker-provider": provider,
        "x-elon-tts-worker-requested-voice": str(payload.get("voiceId") or ""),
        "x-elon-tts-worker-voice": str(payload.get("voiceId") or ""),
        "x-elon-tts-worker-fallback": str(fallback).lower(),
        "x-elon-tts-worker-emotion": str(payload.get("emotionId") or ""),
    }


if __name__ == "__main__":
    import uvicorn

    host = os.getenv("ELON_TTS_WORKER_HOST", "127.0.0.1")
    port = int(os.getenv("ELON_TTS_WORKER_PORT", "5011"))
    uvicorn.run(app, host=host, port=port)
