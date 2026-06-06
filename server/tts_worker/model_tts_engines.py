import importlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from model_tts_common import (
    WorkerFailure,
    call_with_supported_kwargs,
    clamp_float,
    env_bool,
    filter_supported_kwargs,
    read_generated_audio,
    required_path_env,
    shell_quote,
)


MODEL_CACHE: dict[str, Any] = {}


def synthesize_index_tts2(
    payload: dict[str, Any],
    text: str,
    voice_audio: Path,
    emotion_audio: Path | None,
) -> bytes:
    command = os.getenv("ELON_INDEXTTS2_COMMAND", "").strip()
    if command:
        return run_command_adapter("index_tts2", command, payload, text, voice_audio, emotion_audio)

    tts = get_index_tts2()
    with tempfile.TemporaryDirectory(prefix="elon-indextts2-") as tmpdir:
        output_path = Path(tmpdir) / "output.wav"
        kwargs: dict[str, Any] = {
            "spk_audio_prompt": str(voice_audio),
            "text": text,
            "output_path": str(output_path),
            "verbose": False,
        }
        if emotion_audio is not None:
            kwargs["emo_audio_prompt"] = str(emotion_audio)
            kwargs["emo_alpha"] = clamp_float(payload.get("emoAlpha"), 0.0, 1.0, 1.0)
        if env_bool("ELON_INDEXTTS2_USE_RANDOM", False):
            kwargs["use_random"] = True
        call_with_supported_kwargs(tts.infer, kwargs)
        return read_generated_audio(output_path, "IndexTTS2")


def get_index_tts2() -> Any:
    cached = MODEL_CACHE.get("index_tts2")
    if cached is not None:
        return cached

    model_dir = required_path_env("ELON_INDEXTTS2_MODEL_DIR")
    cfg_path = Path(os.getenv("ELON_INDEXTTS2_CFG_PATH", str(model_dir / "config.yaml"))).resolve()
    if not cfg_path.exists():
        raise WorkerFailure(503, f"ELON_INDEXTTS2_CFG_PATH not found: {cfg_path}")

    try:
        module = importlib.import_module("indextts.infer_v2")
        index_tts2 = getattr(module, "IndexTTS2")
    except Exception as exc:
        raise WorkerFailure(
            503,
            "IndexTTS2 runtime is not importable. Set ELON_TTS_MODEL_PYTHONPATH to the "
            "index-tts repo or install its Python package.",
        ) from exc

    kwargs = {
        "cfg_path": str(cfg_path),
        "model_dir": str(model_dir),
        "use_fp16": env_bool("ELON_INDEXTTS2_USE_FP16", False),
        "use_cuda_kernel": env_bool("ELON_INDEXTTS2_USE_CUDA_KERNEL", False),
        "use_deepspeed": env_bool("ELON_INDEXTTS2_USE_DEEPSPEED", False),
    }
    model = index_tts2(**filter_supported_kwargs(index_tts2, kwargs))
    MODEL_CACHE["index_tts2"] = model
    return model


def synthesize_cosyvoice(payload: dict[str, Any], text: str, voice_audio: Path) -> bytes:
    command = os.getenv("ELON_COSYVOICE_COMMAND", "").strip()
    if command:
        return run_command_adapter("cosyvoice3", command, payload, text, voice_audio, None)

    cosyvoice = get_cosyvoice()
    prompt_text = prompt_text_for(voice_audio)
    instruction = cosyvoice_instruction(payload)
    with tempfile.TemporaryDirectory(prefix="elon-cosyvoice-") as tmpdir:
        output_path = Path(tmpdir) / "output.wav"
        save_cosyvoice_audio(cosyvoice, text, prompt_text, voice_audio, instruction, output_path)
        return read_generated_audio(output_path, "CosyVoice")


def get_cosyvoice() -> Any:
    cached = MODEL_CACHE.get("cosyvoice3")
    if cached is not None:
        return cached

    repo_dir = os.getenv("ELON_COSYVOICE_REPO_DIR", "").strip()
    if repo_dir:
        matcha = str(Path(repo_dir).resolve() / "third_party" / "Matcha-TTS")
        if matcha not in sys.path:
            sys.path.append(matcha)

    model_dir = required_path_env("ELON_COSYVOICE_MODEL_DIR")
    try:
        module = importlib.import_module("cosyvoice.cli.cosyvoice")
        auto_model = getattr(module, "AutoModel")
    except Exception as exc:
        raise WorkerFailure(
            503,
            "CosyVoice runtime is not importable. Set ELON_TTS_MODEL_PYTHONPATH or "
            "ELON_COSYVOICE_REPO_DIR to the CosyVoice repo.",
        ) from exc

    model = auto_model(model_dir=str(model_dir))
    MODEL_CACHE["cosyvoice3"] = model
    return model


def save_cosyvoice_audio(
    cosyvoice: Any,
    text: str,
    prompt_text: str,
    voice_audio: Path,
    instruction: str,
    output_path: Path,
) -> None:
    try:
        import torch
        import torchaudio
    except Exception as exc:
        raise WorkerFailure(503, "CosyVoice requires torch and torchaudio") from exc

    if hasattr(cosyvoice, "inference_instruct2"):
        chunks = cosyvoice.inference_instruct2(text, instruction, str(voice_audio), stream=False)
    elif hasattr(cosyvoice, "inference_zero_shot"):
        chunks = cosyvoice.inference_zero_shot(text, prompt_text, str(voice_audio), stream=False)
    else:
        raise WorkerFailure(503, "CosyVoice model exposes no supported inference method")

    tensors = [chunk["tts_speech"] for chunk in chunks if "tts_speech" in chunk]
    if not tensors:
        raise WorkerFailure(503, "CosyVoice returned empty audio")
    audio = tensors[0] if len(tensors) == 1 else torch.cat(tensors, dim=-1)
    torchaudio.save(str(output_path), audio, int(getattr(cosyvoice, "sample_rate", 22050)))


def prompt_text_for(voice_audio: Path) -> str:
    for metadata_path in [voice_audio.with_suffix(".json"), voice_audio.parent / "profile.json"]:
        if not metadata_path.exists():
            continue
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        for key in ["promptText", "prompt_text", "transcript", "text"]:
            value = str(metadata.get(key) or "").strip()
            if value:
                return value
    value = os.getenv("ELON_COSYVOICE_PROMPT_TEXT", "").strip()
    if value:
        return value
    raise WorkerFailure(
        503,
        "CosyVoice requires prompt text. Add a JSON file next to the voice wav with promptText, "
        "or set ELON_COSYVOICE_PROMPT_TEXT.",
    )


def cosyvoice_instruction(payload: dict[str, Any]) -> str:
    parts = [
        "You are a helpful assistant.",
        str(payload.get("voicePrompt") or "").strip(),
        f"Emotion: {str(payload.get('emotionLabel') or payload.get('emotionId') or '').strip()}",
        f"Style: {str(payload.get('textStyle') or '').strip()}",
    ]
    speed = clamp_float(payload.get("speed"), 0.5, 1.5, 1.0)
    if speed > 1.05:
        parts.append("Speak faster.")
    elif speed < 0.95:
        parts.append("Speak slower.")
    return " ".join(part for part in parts if part).strip() + "<|endofprompt|>"


def run_command_adapter(
    provider: str,
    command: str,
    payload: dict[str, Any],
    text: str,
    voice_audio: Path,
    emotion_audio: Path | None,
) -> bytes:
    with tempfile.TemporaryDirectory(prefix=f"elon-{provider}-cmd-") as tmpdir:
        tmp = Path(tmpdir)
        request_path = tmp / "request.json"
        output_path = tmp / "output.wav"
        command_payload = dict(payload)
        command_payload.update(
            {
                "resolvedVoiceAudio": str(voice_audio),
                "resolvedEmotionAudio": str(emotion_audio) if emotion_audio else None,
                "outputPath": str(output_path),
            }
        )
        request_path.write_text(json.dumps(command_payload, ensure_ascii=False), encoding="utf-8")
        env = os.environ.copy()
        env.update(
            {
                "ELON_TTS_REQUEST_JSON": str(request_path),
                "ELON_TTS_OUTPUT_WAV": str(output_path),
                "ELON_TTS_TEXT": text,
                "ELON_TTS_VOICE_AUDIO": str(voice_audio),
                "ELON_TTS_EMOTION_AUDIO": str(emotion_audio) if emotion_audio else "",
            }
        )
        rendered = command.replace("{request_json}", shell_quote(str(request_path))).replace(
            "{output_wav}", shell_quote(str(output_path))
        )
        completed = subprocess.run(
            rendered,
            shell=True,
            cwd=os.getenv("ELON_TTS_COMMAND_CWD") or None,
            env=env,
            text=True,
            capture_output=True,
            timeout=int(os.getenv("ELON_TTS_COMMAND_TIMEOUT_SECS", "180")),
        )
        if completed.returncode != 0:
            detail = (completed.stderr or completed.stdout or "").strip()[-1200:]
            raise WorkerFailure(503, f"{provider} command failed with {completed.returncode}: {detail}")
        return read_generated_audio(output_path, provider)
