# 女声情绪 TTS 部署说明

本项目的生产级 TTS 分为两层：

1. Rust 主服务：负责鉴权、角色/情绪/强度目录、台词改写、缓存和调用 Worker。
2. Python/ONNX TTS Worker：实际加载 IndexTTS2、CosyVoice3 或 GPT-SoVITS。

这样做的原因是模型依赖重、GPU 环境变化大，不应塞进 `elon-server` 主进程。

## APK 运行链路

```text
APK 开启喇叭朗读
→ POST /api/voice/tts
→ Rust 校验登录、选择女声/情绪/强度
→ 可选 LLM 台词改写
→ 调用 TTS Worker /synthesize
→ 服务端缓存音频
→ APK 播放音频
→ Worker 不可用时 APK 自动回退 Android 系统 TTS
```

## 服务器环境变量

```bash
# 必填：TTS Worker 地址。未设置时 /api/voice/tts 返回 503，APK 会自动回退系统 TTS。
ELON_TTS_WORKER_URL=http://127.0.0.1:5010

# 可选：auto/index_tts2/cosyvoice3/gpt_sovits。
# auto 会把普通低强度路由给 CosyVoice3，把强情绪路由给 IndexTTS2。
ELON_TTS_PROVIDER=auto

# 可选：Worker 内部鉴权。
ELON_TTS_WORKER_TOKEN=

# 可选：合成超时与缓存。
ELON_TTS_TIMEOUT_SECS=120
ELON_TTS_CACHE_ENABLED=true

# 可选：开启后先用当前默认 LLM 把普通回复改写成适合朗读的台词。
# 不开也会做本地 Markdown 清理和轻量停顿改写。
ELON_TTS_LLM_REWRITE_ENABLED=false
```

## 当前生产部署

生产机 `43.139.149.158` 当前没有 GPU，也没有 Docker。为了先让 APK 真实播放服务器返回的女声情绪音频，
仓库提供了一个轻量 Python Worker：

```text
server/tts_worker/edge_tts_worker.py
```

它实现同一个 `/synthesize` 合约，内部使用 Edge 在线神经女声，并把本项目的
`voiceId`、`emotionId`、`speed` 映射到不同女声、语速、音高和音量。这样主服务、
APK、缓存、鉴权和降级链路都和生产级模型 Worker 保持一致；后续如果有 GPU 或模型资产，
只需要替换 Worker 内部实现，不需要改 Rust API 或 APK。

Windows 部署：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\deploy-tts-worker.ps1
```

脚本会：

1. 上传 `server/tts_worker` 到 `/root/Elon/server/tts_worker`
2. 创建 Python venv 并安装依赖
3. 创建并启动 `elon-tts-worker.service`
4. 写入 `/root/Elon/server/.env` 的 `ELON_TTS_WORKER_URL=http://127.0.0.1:5010`
5. 重启 `elon-server.service`
6. 验证 `/health` 和 `/api/voice/tts/catalog`

## 真实模型 Worker：IndexTTS2 / CosyVoice

Edge Worker 只适合验证链路。要满足“不同女孩声线 + 情绪参考”的产品目标，必须部署模型 Worker：

```text
server/tts_worker/model_tts_worker.py
```

它和 Edge Worker 使用同一个 `/synthesize` 合约，但行为不同：

1. `index_tts2` 使用 `voiceAudio` 作为 `spk_audio_prompt`，使用 `emotionAudio` 作为 `emo_audio_prompt`，使用 `emoAlpha` 控制情绪参考强度。
2. `cosyvoice3` 使用 `voiceAudio` 作为 zero-shot / instruct2 prompt 音频，需要 prompt 音频对应的文字转写。
3. 缺模型、缺权重、缺参考音频时返回 503 诊断，不会静默切到 Edge。
4. 只有显式配置 `ELON_TTS_MODEL_FALLBACK_URL` 时才会回退到另一个 Worker，响应头会带 `x-elon-tts-worker-fallback: true`。

IndexTTS2 官方 Python 示例使用 `spk_audio_prompt`、`emo_audio_prompt`、`emo_alpha` 组合控制说话人和情绪：
https://github.com/index-tts/index-tts

CosyVoice 官方示例使用 `inference_zero_shot` / `inference_instruct2` 和 prompt 音频：
https://github.com/FunAudioLLM/CosyVoice/blob/main/example.py

### 本机启动模型 Worker

先创建资产目录骨架：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\new-tts-asset-pack.ps1 -AssetRoot "D:\tts-assets"
```

然后把授权参考音频放进去：

```text
D:\tts-assets\voices\female_warm_neutral.wav
D:\tts-assets\voices\female_bright_neutral.wav
D:\tts-assets\voices\female_mature_neutral.wav
D:\tts-assets\voices\female_cool_neutral.wav
D:\tts-assets\voices\female_sweet_neutral.wav

D:\tts-assets\emotions\female_neutral.wav
D:\tts-assets\emotions\female_gentle_comfort.wav
D:\tts-assets\emotions\female_crying_broken.wav
...
```

#### 安装 IndexTTS2

IndexTTS2 官方要求用 `uv` 管理依赖环境，并通过 HuggingFace 或 ModelScope 下载 `IndexTeam/IndexTTS-2` 权重。仓库脚本封装了这些步骤：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-indextts2-runtime.ps1 `
  -InstallRoot "D:\models\IndexTTS2" `
  -DownloadFrom huggingface
```

脚本默认把 `UV_CACHE_DIR` 放到 `D:\models\IndexTTS2\.uv-cache`，避免 PyTorch / CUDA wheel 缓存占满 C 盘。

脚本默认用 `GIT_LFS_SKIP_SMUDGE=1` clone 官方仓库，只拉源码，不拉 GitHub LFS 示例音频。原因是官方仓库 LFS 额度可能临时耗尽；示例音频不是本项目运行必需项。只有确认官方 LFS 可用时才加 `-PullLfsExamples`。

国内网络下载 HuggingFace 慢时可以改用：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-indextts2-runtime.ps1 `
  -InstallRoot "D:\models\IndexTTS2" `
  -DownloadFrom modelscope
```

安装完成后，使用 IndexTTS2 的 `uv` 项目环境启动 Worker：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\start-local-model-tts-worker.ps1 `
  -Provider index_tts2 `
  -AssetRoot "D:\tts-assets" `
  -UvProjectDir "D:\models\IndexTTS2\index-tts" `
  -ModelPythonPath "D:\models\IndexTTS2\index-tts" `
  -IndexTts2ModelDir "D:\models\IndexTTS2\index-tts\checkpoints" `
  -IndexTts2CfgPath "D:\models\IndexTTS2\index-tts\checkpoints\config.yaml"
```

本机 worker 启动后检查：

```powershell
curl.exe http://127.0.0.1:5011/health
```

注意：IndexTTS2 当前 PyTorch 依赖使用 CUDA 12.8 wheels。Windows 上如果合成时报 CUDA/driver 错误，需要升级 NVIDIA 驱动或 CUDA Toolkit；也可以先用 CPU 验证链路，但速度会明显慢。

#### 安装 CosyVoice3

CosyVoice 官方建议 Python 3.10 / conda 环境，并下载 `FunAudioLLM/Fun-CosyVoice3-0.5B-2512`。仓库脚本可先按当前 Python 环境安装：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-cosyvoice-runtime.ps1 `
  -InstallRoot "D:\models\CosyVoice" `
  -DownloadFrom modelscope
```

如果你有单独 conda 环境，可以把环境里的 `python.exe` 传给 `-PythonExe`。

启动 CosyVoice3 Worker：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\start-local-model-tts-worker.ps1 `
  -Provider cosyvoice3 `
  -AssetRoot "D:\tts-assets" `
  -ModelPythonPath "D:\models\CosyVoice\CosyVoice" `
  -CosyVoiceRepoDir "D:\models\CosyVoice\CosyVoice" `
  -CosyVoiceModelDir "D:\models\CosyVoice\CosyVoice\pretrained_models\Fun-CosyVoice3-0.5B"
```

CosyVoice 的 prompt 音频需要对应文本。每个声线 wav 旁边放同名 JSON，或在声线目录放 `profile.json`：

```json
{
  "promptText": "你好呀，我是你的 AI 助手，今天也会认真陪你聊天。"
}
```

#### 本机 5 声线联调

Worker 启动后用同一句话测试 5 个 voiceId：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-model-tts-worker.ps1 `
  -WorkerUrl "http://127.0.0.1:5011" `
  -Provider index_tts2 `
  -OutputDir ".runtime\tts-model-tests"
```

输出的 5 个 wav 必须听起来是不同女孩的音色；如果只是音高、语速或情绪不同，说明 `voices/*.wav` 不是 5 个真正不同的授权声线。

### 服务器部署模型 Worker

服务器已有模型源码、权重和资产时：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\deploy-model-tts-worker.ps1 `
  -Provider index_tts2 `
  -RemoteAssetRoot "/root/Elon/server/assets/tts" `
  -ModelPythonPath "/root/models/index-tts" `
  -IndexTts2ModelDir "/root/models/index-tts/checkpoints" `
  -IndexTts2CfgPath "/root/models/index-tts/checkpoints/config.yaml"
```

脚本会：

1. 上传 `model_tts_worker.py`、`model_tts_common.py`、`model_tts_engines.py` 和 `requirements-model.txt`
2. 创建 Python venv
3. 创建并启动 `elon-model-tts-worker.service`
4. 写入主服务 `.env`：`ELON_TTS_WORKER_URL=http://127.0.0.1:5011`
5. 写入主服务 `.env`：`ELON_TTS_PROVIDER=<Provider>`
6. 重启 `elon-server.service`
7. 验证模型 Worker `/health` 和主服务 `/api/voice/tts/catalog`

只想先把模型 Worker 放到服务器上检查 `/health`，但暂时不让 APK 流量切过去时，加：

```powershell
-SkipMainServerUpdate
```

如果服务器没有 GPU，可以先不要把主服务切到模型 Worker。当前生产机没有 GPU 时，继续用 Edge Worker 保持可用；真正模型可放在有 GPU 的本机或另一台机器，再通过内网、隧道或 PC relay 让主服务访问。

### 验证 5 个声线是否真不同

用同一句话、同一个情绪，只改 `voiceId`：

```powershell
$voices = "female_warm","female_bright","female_mature","female_cool","female_sweet"
foreach ($voice in $voices) {
  $body = @{
    text = "你好呀，我是你的 AI 助手，很高兴今天能陪你聊天。"
    voiceId = $voice
    emotionId = "normal"
    intensity = "normal"
    provider = "index_tts2"
    rewrite = $false
  } | ConvertTo-Json
  curl.exe -H "Content-Type: application/json" -H "Authorization: Bearer <token>" `
    -d $body "http://43.139.149.158:8080/api/voice/tts" `
    --output "$voice.wav"
}
```

如果 5 个文件听起来只是同一个人的语气变化，说明 `voices/*.wav` 不是 5 个独立女声参考；需要更换授权素材，而不是继续改 APK。

## Worker HTTP 合约

Rust 主服务会请求：

```http
POST /synthesize
Content-Type: application/json
Authorization: Bearer <ELON_TTS_WORKER_TOKEN>
```

请求 JSON：

```json
{
  "provider": "index_tts2",
  "text": "你一直没有回我……其实我等了你好久……",
  "originalText": "你一直没有回我。其实我等了很久。",
  "voiceId": "female_warm",
  "voiceLabel": "温柔姐姐",
  "voicePrompt": "温柔、亲近、稳定，有陪伴感的年轻女声",
  "voiceAudio": "voices/female_warm_neutral.wav",
  "emotionId": "wronged_crying",
  "emotionLabel": "委屈快哭",
  "emotionAudio": "emotions/female_crying_broken.wav",
  "textStyle": "短句，多停顿，省略号，轻微重复",
  "pauseStyle": "broken",
  "intensity": "immersive",
  "emoAlpha": 0.72,
  "speed": 0.9
}
```

Worker 可以直接返回音频：

```http
200 OK
Content-Type: audio/wav

<wav bytes>
```

也可以返回 JSON：

```json
{
  "audioBase64": "...",
  "mime": "audio/wav"
}
```

## 资产目录约定

第一版建议准备：

```text
voices/female_warm_neutral.wav
voices/female_bright_neutral.wav
voices/female_mature_neutral.wav
voices/female_cool_neutral.wav
voices/female_sweet_neutral.wav

emotions/female_neutral.wav
emotions/female_gentle_comfort.wav
emotions/female_crying_broken.wav
emotions/female_happy_soft.wav
emotions/female_happy_excited.wav
emotions/female_angry_repressed.wav
emotions/female_cool_detached.wav
emotions/female_shy_nervous.wav
emotions/female_sad_low.wav
emotions/female_surprised.wav
emotions/female_serious_encourage.wav
emotions/female_whisper.wav
```

所有预设女声和情绪参考音频必须来自授权配音演员或明确可商用素材。不要使用明星、主播、声优或短视频博主声音做默认公开声线。

## 本机检查

Windows：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\check-tts-stack.ps1
```

它会检查 Python、conda、常见 TTS 包和 `ELON_TTS_WORKER_URL` 是否存在。
