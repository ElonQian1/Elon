#!/usr/bin/env python3
"""
本地 Whisper 转写服务（用于 elon 项目）
基于 faster-whisper + FastAPI，监听 127.0.0.1:5001。

接口：
  POST /transcribe  multipart: audio(WAV文件) + language(str, 默认"zh")
  GET  /health      健康检查

启动方式：
  uvicorn whisper_service:app --host 127.0.0.1 --port 5001
"""

import io
import logging
import os

from fastapi import FastAPI, File, Form, HTTPException, UploadFile
from faster_whisper import WhisperModel

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [whisper] %(levelname)s %(message)s",
)
logger = logging.getLogger("whisper-service")

# 模型选择：tiny(39MB,最快) / base(74MB,准确) / small(244MB,精准)
# 2核1.5GB 服务器推荐 base；CPU int8 量化，内存占用约 400MB
MODEL_SIZE = os.environ.get("WHISPER_MODEL_SIZE", "base")
logger.info(f"正在加载 Whisper {MODEL_SIZE} 模型（首次需下载，约数秒）...")
_model = WhisperModel(MODEL_SIZE, device="cpu", compute_type="int8")
logger.info(f"Whisper {MODEL_SIZE} 模型加载完成")

app = FastAPI(title="Elon Whisper ASR", version="1.0")


@app.post("/transcribe")
async def transcribe(
    audio: UploadFile = File(..., description="WAV 文件（PCM16 LE，任意采样率）"),
    language: str = Form("zh", description="目标语言代码，如 zh / en / ja"),
):
    """
    接收 WAV 音频，返回转写文本。
    Rust 端传入的是用 PCM16 手工封装的 WAV，采样率从 hello 帧取得。
    """
    data = await audio.read()
    if not data:
        raise HTTPException(status_code=400, detail="音频数据为空")

    logger.info(f"收到转写请求：{len(data)} 字节，语言={language}")

    try:
        buf = io.BytesIO(data)
        # beam_size=1 最快；对话场景不需要高 beam
        segments, info = _model.transcribe(
            buf,
            language=language,
            beam_size=1,
            best_of=1,
            vad_filter=True,          # 过滤空白段，减少幻觉
            vad_parameters={"min_silence_duration_ms": 300},
        )
        text = "".join(s.text for s in segments).strip()
        logger.info(f"转写完成：'{text[:60]}' (检测语言={info.language})")
        return {"transcript": text, "language": info.language}

    except Exception as exc:
        logger.error(f"转写失败：{exc}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(exc))


@app.get("/health")
def health():
    return {"status": "ok", "model": MODEL_SIZE}
