import importlib.util
import inspect
import os
import shlex
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable


class WorkerFailure(RuntimeError):
    def __init__(self, status_code: int, detail: str):
        super().__init__(detail)
        self.status_code = status_code
        self.detail = detail


def env_bool(name: str, default: bool) -> bool:
    value = os.getenv(name)
    if value is None:
        return default
    return value.strip().lower() in {"1", "true", "on", "yes"}


def add_model_pythonpath() -> None:
    raw_paths = os.getenv("ELON_TTS_MODEL_PYTHONPATH", "").strip()
    if not raw_paths:
        return
    for raw_path in raw_paths.split(os.pathsep):
        path = raw_path.strip()
        if path and path not in sys.path:
            sys.path.insert(0, path)


def choose_provider(payload: dict[str, Any], default_provider: str) -> str:
    requested = normalize_provider(str(payload.get("provider") or default_provider or "auto"))
    if requested != "auto":
        return requested
    emotion_id = str(payload.get("emotionId") or "normal")
    intensity = str(payload.get("intensity") or "normal")
    if intensity == "normal" and emotion_id in {"normal", "serious_encourage"}:
        return "cosyvoice3"
    return "index_tts2"


def normalize_provider(value: str) -> str:
    normalized = value.strip().lower().replace("-", "_")
    if normalized in {"indextts2", "index_tts2"}:
        return "index_tts2"
    if normalized in {"cosyvoice", "cosy_voice", "cosyvoice3", "cosy_voice3"}:
        return "cosyvoice3"
    if normalized in {"gptsovits", "gpt_sovits"}:
        return "gpt_sovits"
    return "auto"


def resolve_asset(
    asset_root: Path,
    allow_absolute: bool,
    raw: Any,
    field: str,
    required: bool,
) -> Path | None:
    value = str(raw or "").strip()
    if not value:
        if required:
            raise WorkerFailure(503, f"{field} is required for model TTS")
        return None

    path = Path(value)
    if path.is_absolute():
        if not allow_absolute:
            raise WorkerFailure(400, f"{field} must be relative to ELON_TTS_ASSET_ROOT")
        resolved = path.resolve()
    else:
        resolved = (asset_root / value.replace("\\", "/")).resolve()
        try:
            resolved.relative_to(asset_root)
        except ValueError as exc:
            raise WorkerFailure(400, f"{field} escapes ELON_TTS_ASSET_ROOT") from exc

    if not resolved.exists():
        raise WorkerFailure(503, f"{field} file not found: {resolved}")
    return resolved


def runtime_status() -> dict[str, Any]:
    return {
        "indexTts2": {
            "importable": module_available("indextts.infer_v2"),
            "modelDir": os.getenv("ELON_INDEXTTS2_MODEL_DIR", ""),
            "cfgPath": os.getenv("ELON_INDEXTTS2_CFG_PATH", ""),
            "commandConfigured": bool(os.getenv("ELON_INDEXTTS2_COMMAND", "").strip()),
        },
        "cosyVoice": {
            "importable": module_available("cosyvoice.cli.cosyvoice"),
            "modelDir": os.getenv("ELON_COSYVOICE_MODEL_DIR", ""),
            "repoDir": os.getenv("ELON_COSYVOICE_REPO_DIR", ""),
            "commandConfigured": bool(os.getenv("ELON_COSYVOICE_COMMAND", "").strip()),
        },
    }


def asset_status(asset_root: Path, voice_assets: list[str], emotion_assets: list[str]) -> dict[str, Any]:
    voices = [{"path": item, "exists": (asset_root / item).exists()} for item in voice_assets]
    emotions = [{"path": item, "exists": (asset_root / item).exists()} for item in emotion_assets]
    return {
        "voices": voices,
        "emotions": emotions,
        "missingVoices": [item["path"] for item in voices if not item["exists"]],
        "missingEmotions": [item["path"] for item in emotions if not item["exists"]],
    }


def module_available(name: str) -> bool:
    try:
        return importlib.util.find_spec(name) is not None
    except Exception:
        return False


def required_path_env(name: str) -> Path:
    value = os.getenv(name, "").strip()
    if not value:
        raise WorkerFailure(503, f"{name} is required")
    path = Path(value).resolve()
    if not path.exists():
        raise WorkerFailure(503, f"{name} not found: {path}")
    return path


def read_generated_audio(output_path: Path, engine: str) -> bytes:
    if not output_path.exists():
        raise WorkerFailure(503, f"{engine} did not create output audio: {output_path}")
    data = output_path.read_bytes()
    if not data:
        raise WorkerFailure(503, f"{engine} created empty output audio")
    return data


def call_with_supported_kwargs(fn: Callable[..., Any], kwargs: dict[str, Any]) -> Any:
    return fn(**filter_supported_kwargs(fn, kwargs))


def filter_supported_kwargs(fn: Callable[..., Any], kwargs: dict[str, Any]) -> dict[str, Any]:
    try:
        signature = inspect.signature(fn)
    except (TypeError, ValueError):
        return kwargs
    if any(param.kind == inspect.Parameter.VAR_KEYWORD for param in signature.parameters.values()):
        return kwargs
    return {key: value for key, value in kwargs.items() if key in signature.parameters}


def clamp_float(value: Any, low: float, high: float, fallback: float) -> float:
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        parsed = fallback
    return max(low, min(high, parsed))


def shell_quote(value: str) -> str:
    if os.name == "nt":
        return subprocess.list2cmdline([value])
    return shlex.quote(value)
