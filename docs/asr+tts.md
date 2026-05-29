
我现在 服务器 用 faster-whisper 中文识别很差怎么办？


我还想APK 可以 语音回答用户， 就是把 文字用语音回复给用户，

到底怎么样的架构最好呢？ 

结论：**不要在 faster-whisper 上死磕太久**。如果你的主要用户说中文，尤其有口音、噪声、手机麦克风、短句交互，我建议架构改成：

**APK 负责录音、播放、打断；服务器负责 ASR + LLM + TTS。ASR 主力换成 FunASR / SenseVoice / Paraformer，faster-whisper 只做备用。TTS 初期用 Android 自带 TTS，生产版再换服务器 TTS。**

---

## 1. 先把 faster-whisper 能救的地方救一下

faster-whisper 本身只是 Whisper 的 CTranslate2 高速实现，优势是快、省显存，但准确率本质上仍受 Whisper 模型影响；官方 README 也说明它是 Whisper 的重实现，速度可提升、显存更省。([GitHub][1]) 如果你现在用的是 `small`、`medium`、`int8 CPU`，中文效果差很正常。

优先这样试：

```python
# asr_faster_whisper_zh.py

from faster_whisper import WhisperModel


# 准确率优先：large-v3
# 延迟优先：turbo
# 注意：turbo 更快，但相比 large-v3 会有一点质量损失
MODEL_SIZE = "large-v3"

# GPU 推荐 float16
# 如果只能 CPU，可以用 compute_type="int8"，但中文准确率和速度都可能受影响
model = WhisperModel(
    MODEL_SIZE,
    device="cuda",
    compute_type="float16",
)


def transcribe_chinese(audio_path: str) -> str:
    """
    识别中文音频。
    audio_path 可以是 wav/mp3/m4a，但服务器端最好先统一转成 16k、单声道 wav。
    """

    segments, info = model.transcribe(
        audio_path,

        # 非常重要：不要让它自己猜语言
        # 短音频、噪声、夹英文时，自动语言判断容易出问题
        language="zh",

        # 非常重要：只做转写，不做翻译
        task="transcribe",

        # 一般 5 比较稳；太大延迟会上升
        beam_size=5,

        # 中文实时短句建议打开 VAD
        vad_filter=True,
        vad_parameters={
            # faster-whisper 官方示例也展示了可调 min_silence_duration_ms
            # 500ms 对语音助手比较适中
            "min_silence_duration_ms": 500,
        },

        # 实时对话建议关掉上下文依赖，避免上一句错了影响下一句
        condition_on_previous_text=False,

        # 给模型一点中文上下文提示，尤其适合领域词
        initial_prompt=(
            "以下是简体中文普通话语音识别文本，可能包含技术词："
            "APK、服务器、语音识别、语音合成、Rust、faster-whisper、FunASR、模型、接口。"
        ),

        # 如果你安装的 faster-whisper 版本支持 hotwords，可以加领域词
        hotwords="APK 服务器 语音识别 语音合成 Rust faster-whisper FunASR",
    )

    print(f"检测语言: {info.language}, 置信度: {info.language_probability}")

    text_parts = []
    for segment in segments:
        text_parts.append(segment.text.strip())

    return "".join(text_parts)


if __name__ == "__main__":
    result = transcribe_chinese("test.wav")
    print(result)
```

关键点是：**强制 `language="zh"`，强制 `task="transcribe"`，不要让模型自动猜语言**。faster-whisper 的参数文档也说明，`language` 不设置时会从音频前 30 秒检测语言，`task` 可以是 `transcribe` 或 `translate`。([GitHub][2]) VAD 也要打开，faster-whisper 集成了 Silero VAD，并且 README 里给了 `vad_filter=True` 和 `min_silence_duration_ms=500` 的用法。([GitHub][1])

还有一个很常见的坑：**不是模型差，是 APK 传上来的音频已经烂了**。你一定要在服务器保存用户上传的原始音频，自己听一下。如果声音发闷、断断续续、采样率错、双声道乱、Opus/WebM 解码错，ASR 一定差。服务端最好统一转：

```bash
ffmpeg -y -i input_audio_file -ac 1 -ar 16000 -vn normalized.wav
```

---

## 2. 中文识别真想好：建议换 FunASR / SenseVoice

如果你主要识别中文，我建议你不要把 faster-whisper 当主力。更合理的是：

| 场景              | 推荐                                   |
| --------------- | ------------------------------------ |
| 普通话、中文助手、中文客服   | **FunASR / Paraformer / SenseVoice** |
| 中文 + 英文 + 多语种都要 | faster-whisper `large-v3` 备用         |
| 手机端离线识别         | sherpa-onnx                          |
| 服务器实时流式识别       | FunASR streaming / SenseVoice        |

SenseVoice 官方介绍里明确说它是语音基础模型，包含 ASR、语言识别、情绪识别、音频事件检测等能力，并且重点强调多语种语音识别和低延迟；它的 README 写到支持 50+ 语言、训练数据超过 40 万小时，并称识别性能超过 Whisper。([GitHub][3]) FunASR 更偏工程落地，官方示例直接支持 `paraformer-zh + fsmn-vad + ct-punc`，也就是中文识别、VAD、标点恢复一套上。([GitHub][4]) FunASR 还提供实时识别 WebSocket 方案，官方说明适合 live captions、语音助手等场景。([FunASR][5])

我的建议是：

**第一优先级：FunASR / SenseVoice 做主 ASR。**
**第二优先级：faster-whisper large-v3 做备用。**
**第三优先级：如果用户有方言，就找对应方言模型或自己微调。**

---

## 3. 我推荐的整体架构

我建议你这样做：

```text
┌──────────────────────────────┐
│            APK               │
│                              │
│  AudioRecord 录音             │
│  降噪 / 回声消除 / VAD         │
│  WebSocket 发送音频帧          │
│  AudioTrack / ExoPlayer 播放   │
│  用户打断时 stop playback      │
└───────────────┬──────────────┘
                │ WebSocket
                ▼
┌──────────────────────────────┐
│        Rust Voice Gateway     │
│                              │
│  连接管理                     │
│  鉴权                         │
│  会话状态                     │
│  限流                         │
│  日志                         │
│  音频帧转发                   │
└───────────────┬──────────────┘
                │ gRPC / HTTP / Unix Socket
                ▼
┌──────────────────────────────┐
│        Python ASR Worker      │
│                              │
│  FunASR / SenseVoice 主力      │
│  faster-whisper 备用          │
│  输出 partial / final text    │
└───────────────┬──────────────┘
                ▼
┌──────────────────────────────┐
│        LLM / 业务逻辑服务      │
│                              │
│  意图识别                     │
│  RAG / 数据库查询              │
│  生成回复文字                 │
└───────────────┬──────────────┘
                ▼
┌──────────────────────────────┐
│          TTS Worker           │
│                              │
│  MVP: APK 本地 Android TTS     │
│  生产: 服务器 CosyVoice/Kokoro │
│  返回 wav/pcm/mp3 音频流       │
└───────────────┬──────────────┘
                ▼
┌──────────────────────────────┐
│            APK               │
│                              │
│  边收边播                     │
│  用户说话时打断当前语音        │
└──────────────────────────────┘
```

为什么我建议你用 **Rust 做 Gateway，Python 做模型 Worker**？因为 Rust 适合连接管理、WebSocket、高并发、鉴权、状态机；但 ASR/TTS 模型生态主要在 Python / ONNX。你不要把大模型硬塞进 Rust 主进程，后面排查问题会很痛苦。

---

## 4. APK 语音回复：先用 Android 自带 TTS，后面再升级

你说“APK 可以语音回答用户”，最简单版本就是：服务器只返回文字，APK 用 Android 自带 `TextToSpeech` 读出来。Android 官方 API 里 `speak()` 是异步的，会把文本加入 TTS 队列；也可以用 `synthesizeToFile()` 把文字合成到文件。([Android Developers][6])

先这样写就够用：

```kotlin
// VoiceSpeaker.kt

package com.example.voice

import android.content.Context
import android.os.Bundle
import android.speech.tts.TextToSpeech
import java.util.Locale
import java.util.UUID

class VoiceSpeaker(
    context: Context
) : TextToSpeech.OnInitListener {

    private var tts: TextToSpeech? = null
    private var ready: Boolean = false

    init {
        // 初始化 Android 系统 TTS 引擎
        tts = TextToSpeech(context.applicationContext, this)
    }

    override fun onInit(status: Int) {
        if (status != TextToSpeech.SUCCESS) {
            ready = false
            return
        }

        val engine = tts ?: return

        // 简体中文
        val result = engine.setLanguage(Locale.SIMPLIFIED_CHINESE)

        ready = result != TextToSpeech.LANG_MISSING_DATA &&
                result != TextToSpeech.LANG_NOT_SUPPORTED

        // 语速，1.0 是正常速度
        engine.setSpeechRate(1.0f)

        // 音调，1.0 是正常音调
        engine.setPitch(1.0f)
    }

    fun speak(text: String) {
        if (!ready) {
            return
        }

        val engine = tts ?: return

        val params = Bundle()

        // 每次说话都给一个唯一 ID，方便以后监听播放完成事件
        val utteranceId = UUID.randomUUID().toString()

        // QUEUE_FLUSH 表示打断之前没说完的内容，直接说新的
        engine.speak(
            text,
            TextToSpeech.QUEUE_FLUSH,
            params,
            utteranceId
        )
    }

    fun stop() {
        tts?.stop()
    }

    fun shutdown() {
        tts?.stop()
        tts?.shutdown()
        tts = null
        ready = false
    }
}
```

MVP 阶段这样最快：

```text
用户说话 → APK 上传音频 → 服务器识别文字 → LLM 生成回复文字 → APK 本地 TTS 朗读
```

这个方案优点是简单、省服务器算力、延迟低。缺点是不同手机的 TTS 声音不一样，中文自然度也不可控。

---

## 5. 生产版 TTS：服务器合成语音，再推给 APK 播放

如果你想要“固定的 AI 声音”、更自然的中文、可控语气，那就做服务器 TTS：

```text
用户说话
→ 服务器 ASR
→ LLM 生成回复
→ 服务器 TTS 合成音频
→ WebSocket 分片推给 APK
→ APK 边收边播
```

中文 TTS 可以看两类：

**高质量中文语音：CosyVoice。** CosyVoice 官方仓库说明它支持中文、英文、日语、韩语等 9 种常见语言，还支持 18+ 中文方言/口音，并支持文本流式输入和音频流式输出，最低延迟可到 150ms。([GitHub][7]) CosyVoice 2.0 页面也强调了流式与非流式统一建模，首包延迟可到 150ms。([funaudiollm.github.io][8])

**轻量部署：Kokoro / Piper / VITS / sherpa-onnx。** Kokoro 是 82M 参数开源 TTS，模型卡说明它轻量、快，并且是 Apache 许可。([Hugging Face][9]) sherpa-onnx 也支持 Android、iOS、服务器、嵌入式等平台，并且包含 STT、TTS、VAD 等能力；它的 Android 构建文档明确说同一节适用于 STT 和 TTS。([K2 FSA][10])

我的排序：

1. **MVP：Android 自带 TTS。**
2. **正式版：服务器 CosyVoice / Kokoro TTS，APK 播放服务器返回的音频。**
3. **离线版：sherpa-onnx 放到 APK，但 APK 会变大，手机耗电和性能要测试。**

---

## 6. WebSocket 协议建议

你可以设计成一个双向流：

```text
APK → Server:
audio.start
audio.frame  二进制 PCM/Opus
audio.end
user.cancel

Server → APK:
asr.partial
asr.final
llm.partial
tts.audio
tts.end
error
```

实际交互：

```text
1. APK 开始录音
2. 每 20ms / 40ms / 100ms 发一帧音频
3. ASR 返回 partial 文本
4. 用户停顿后 ASR 返回 final 文本
5. LLM 生成回复
6. TTS 开始合成
7. APK 边接收 tts.audio 边播放
8. 如果用户中途说话，APK 立刻发送 user.cancel，停止当前 TTS
```

这个“用户打断”非常重要。语音助手体验好不好，很多时候不取决于模型多强，而取决于能不能自然打断。

---

## 7. 你现在应该按这个顺序做

第一步，先保存 APK 上传到服务器的原始音频，人工听一遍。很多中文识别差，其实是音频上传/编码/采样率错了。

第二步，把 faster-whisper 改成：

```text
large-v3
language="zh"
task="transcribe"
vad_filter=True
condition_on_previous_text=False
beam_size=5
```

第三步，同一批音频拿去跑 FunASR / SenseVoice，对比字错误率。只要中文明显更好，就不要再纠结 faster-whisper。

第四步，APK 先接 Android `TextToSpeech`，让产品先跑通。

第五步，正式版再做服务器 TTS 流式播放。

我给你的最终推荐是：

```text
ASR：FunASR / SenseVoice 主力，faster-whisper large-v3 备用
LLM：独立服务
TTS：MVP 用 Android TTS，生产用服务器 CosyVoice/Kokoro
网关：Rust WebSocket Gateway
模型服务：Python Worker / ONNX Worker
APK：只做录音、播放、打断、状态展示
```

这样最稳，也最容易一步一步升级。

[1]: https://github.com/SYSTRAN/faster-whisper "GitHub - SYSTRAN/faster-whisper: Faster Whisper transcription with CTranslate2 · GitHub"
[2]: https://github.com/SYSTRAN/faster-whisper/blob/master/faster_whisper/transcribe.py "faster-whisper/faster_whisper/transcribe.py at master · SYSTRAN/faster-whisper · GitHub"
[3]: https://github.com/FunAudioLLM/SenseVoice "GitHub - FunAudioLLM/SenseVoice: Multilingual speech understanding: ASR + emotion recognition + audio event detection. 50+ languages, 15x faster than Whisper, non-autoregressive. · GitHub"
[4]: https://github.com/modelscope/FunASR/blob/main/docs/tutorial/README.md "FunASR/docs/tutorial/README.md at main · modelscope/FunASR · GitHub"
[5]: https://www.funasr.com/en/ "FunASR - Industrial Speech Recognition | All-in-One Speech Understanding"
[6]: https://developer.android.com/reference/android/speech/tts/TextToSpeech "TextToSpeech  |  API reference  |  Android Developers"
[7]: https://github.com/FunAudioLLM/CosyVoice "GitHub - FunAudioLLM/CosyVoice: Multi-lingual large voice generation model, providing inference, training and deployment full-stack ability. · GitHub"
[8]: https://funaudiollm.github.io/cosyvoice2/ "CosyVoice2.0"
[9]: https://huggingface.co/hexgrad/Kokoro-82M "hexgrad/Kokoro-82M · Hugging Face"
[10]: https://k2-fsa.github.io/sherpa/onnx/android/build-sherpa-onnx.html "Build sherpa-onnx for Android — sherpa 1.3 documentation"


你推荐的 ASR  是否免费，满足各种语言？而不仅仅局限中文？

可以免费，但要分清楚两件事：

**第一，模型/代码是否免费开源。**
**第二，它是不是“各种语言都一样准”。**

我的结论是：

**如果你要做全球多语言，faster-whisper / Whisper 仍然应该保留，作为通用多语言兜底。
如果你要中文、粤语、亚洲语言效果更好，再加 FunASR / SenseVoice。
不要只押一个 ASR 模型。**

---

## 1. 哪些是免费的？

### Whisper / faster-whisper

**Whisper 是免费开源的，代码和模型权重都是 MIT License。** OpenAI 官方 Whisper 仓库明确写了代码和模型权重使用 MIT License。Whisper 本身支持多语言识别、翻译和语言识别。([GitHub][1])

**faster-whisper 也是免费开源的 MIT License。** 它本质上不是一个新 ASR 模型，而是用 CTranslate2 重新实现 Whisper 推理，让 Whisper 跑得更快、更省显存。([GitHub][2])

所以：

```text
Whisper / faster-whisper：
免费：是
可本地部署：是
商业使用友好：相对友好，MIT License
语言覆盖：最广
中文效果：能用，但不一定是最强
```

---

### FunASR

**FunASR 工具包是 MIT License。** 官方 GitHub 页面显示 FunASR 项目使用 MIT license，并且官方介绍它是工业级语音识别工具包，支持 50+ languages、说话人分离、情绪检测、流式识别和 OpenAI-compatible API。([GitHub][3])

但是要注意：**FunASR 的“代码许可证”和“模型权重许可证”不是完全一回事。** FunASR 的模型权重使用自己的 Model Open Source License Agreement，里面写了可以免费使用、复制、修改和分享模型，但使用时需要保留来源、作者信息和相关模型名称。([GitHub][4])

所以：

```text
FunASR：
免费：是，可以自部署
可本地部署：是
商业使用：需要遵守 FunASR 模型许可证，至少要保留署名/来源信息
语言覆盖：官方页面说 50+，具体模型可能是 31 或 50+，要看你选的模型
中文效果：通常比 Whisper 更适合中文场景
```

这里有一个细节：FunASR 官网写的是支持 50+ languages，并包括中文方言和自动语言检测。([FunASR][5]) 但 FunASR 教程里对 `Fun-ASR-Nano` 的说明是支持 31 种语言，包括中文方言。([GitHub][6]) 所以不要简单理解成“FunASR 所有模型都支持 50+ 语言”，要看具体模型版本。

---

### SenseVoice

**SenseVoice / SenseVoiceSmall 也是可以免费自部署的，但许可证要看它的 model-license。** Hugging Face 上的 SenseVoiceSmall 标的是 `model-license`，不是简单的 MIT。([Hugging Face][7])

SenseVoice 官方介绍里说它是语音基础模型，包含 ASR、语种识别、情绪识别、音频事件检测，并且训练数据超过 40 万小时，支持 50+ languages。([GitHub][8]) 不过 SenseVoiceSmall 页面也写到，2024 年开源的小模型主要提供 Mandarin、Cantonese、English、Japanese、Korean 等能力。([Hugging Face][7])

所以：

```text
SenseVoice：
免费：是，可以自部署
可本地部署：是
商业使用：需要看 model-license，不要当成 MIT
语言覆盖：官方宣传 50+，但 Small 版实际重点是中/粤/英/日/韩
中文效果：很适合中文、粤语、低延迟场景
```

---

## 2. “各种语言”应该选谁？

没有一个免费 ASR 能做到“所有语言都很准”。更现实的判断是：

| 目标            | 推荐                                            |
| ------------- | --------------------------------------------- |
| 全球多语言覆盖最广     | **Whisper / faster-whisper large-v3 或 turbo** |
| 中文、粤语、亚洲语言优先  | **FunASR / SenseVoice**                       |
| 中文客服、中文语音助手   | **FunASR / SenseVoice 主力，Whisper 备用**         |
| 多语言混说、用户国家不固定 | **Whisper 主力，FunASR/SenseVoice 中文增强**         |
| 想完全免费自建       | **可以，模型免费，但你要付服务器/GPU成本**                     |

Whisper 的多语言覆盖最稳。官方 tokenizer 代码里默认 `num_languages=99`，也就是说 Whisper 体系是按 99 种语言 token 设计的。([GitHub][9]) 但 Whisper 官方 README 也提醒：不同语言的表现差异很大，不能认为每种语言效果都一样好。([GitHub][1])

---

## 3. 我建议你最终架构这样改

你不要只部署一个 ASR。最好做成 **ASR Router**：

```text
APK 上传音频
    ↓
服务器统一转 16k 单声道 PCM/WAV
    ↓
VAD 切分语音
    ↓
语言检测 / 用户设置语言
    ↓
ASR Router 选择模型
    ↓
输出文本给 LLM
```

模型选择规则可以这样：

```text
用户语言 = 中文 / 粤语 / 中英混说：
    优先 SenseVoice 或 FunASR
    识别置信度低时，再跑 faster-whisper large-v3 兜底

用户语言 = 英语 / 西语 / 法语 / 德语 / 日语 / 韩语 / 俄语等：
    优先 faster-whisper large-v3 或 turbo

用户语言未知：
    先用 Whisper / SenseVoice 做 language detect
    再路由到对应 ASR

用户网络差 / 服务器压力大：
    使用小模型快速返回
    后台用大模型二次修正
```

---

## 4. 我的最终推荐

如果你的产品主要面向中国用户，但又想支持多语言，我会这样配：

```text
主 ASR 1：FunASR 或 SenseVoice
用途：中文、粤语、中英混说、亚洲语言、低延迟语音助手

备用 ASR 2：faster-whisper large-v3 / turbo
用途：全球多语言兜底、未知语言、英文/欧洲语言

ASR Router：
根据用户设置语言、自动语言检测、置信度决定走哪个模型

TTS：
MVP 用 Android 本地 TTS
正式版用服务器 TTS
```

简单说：

**中文效果差，不要只靠 faster-whisper。
多语言覆盖，不要丢掉 Whisper。
最稳方案是：FunASR/SenseVoice + faster-whisper 双引擎。**

如果你担心许可证，我建议商业产品里优先使用：

```text
Whisper / faster-whisper：许可证最清晰，MIT
FunASR / SenseVoice：可以用，但上线前把 model-license、署名、模型名称保留这些事情做好
```

[1]: https://github.com/openai/whisper "GitHub - openai/whisper: Robust Speech Recognition via Large-Scale Weak Supervision · GitHub"
[2]: https://github.com/SYSTRAN/faster-whisper "GitHub - SYSTRAN/faster-whisper: Faster Whisper transcription with CTranslate2 · GitHub"
[3]: https://github.com/modelscope/FunASR "GitHub - modelscope/FunASR: Industrial-grade speech recognition toolkit: 170x realtime, 50+ languages, speaker diarization, emotion detection, streaming, and OpenAI-compatible API. · GitHub"
[4]: https://github.com/modelscope/FunASR/blob/main/MODEL_LICENSE "FunASR/MODEL_LICENSE at main · modelscope/FunASR · GitHub"
[5]: https://www.funasr.com/en/ "FunASR - Industrial Speech Recognition | All-in-One Speech Understanding"
[6]: https://github.com/modelscope/FunASR/blob/main/docs/tutorial/README.md "FunASR/docs/tutorial/README.md at main · modelscope/FunASR · GitHub"
[7]: https://huggingface.co/FunAudioLLM/SenseVoiceSmall "FunAudioLLM/SenseVoiceSmall · Hugging Face"
[8]: https://github.com/FunAudioLLM/SenseVoice "GitHub - FunAudioLLM/SenseVoice: Multilingual speech understanding: ASR + emotion recognition + audio event detection. 50+ languages, 15x faster than Whisper, non-autoregressive. · GitHub"
[9]: https://github.com/openai/whisper/blob/main/whisper/tokenizer.py "whisper/whisper/tokenizer.py at main · openai/whisper · GitHub"


需不需要 烧token 这个很关键


**你最该避免的是：不要把“用户整段语音”直接丢给大模型实时语音 API。**
那样可能会烧 **audio tokens**，成本会很快失控。

你比较稳的低成本架构应该是：

```text
APK 录音
  ↓
自部署 ASR：FunASR / SenseVoice / faster-whisper
  ↓
得到文字
  ↓
LLM 只处理文字
  ↓
得到回复文字
  ↓
APK 本地 TTS 或自部署 TTS
```

这样的话：

```text
ASR：不烧 token，只烧你的服务器 CPU/GPU
TTS：本地 TTS 不烧 token，自部署 TTS 也不烧 token
LLM：只有“用户文字 + 回复文字”才烧 token
```

---

## 1. 自部署 ASR 会不会烧 token？

**不会。**

如果你自己服务器部署这些：

```text
faster-whisper
FunASR
SenseVoice
sherpa-onnx
```

它们不是按 token 收费。它们只是模型推理，成本来自：

```text
服务器租金
GPU 显存
CPU
带宽
磁盘
运维
```

也就是说，用户说 1 分钟话，不会产生 API token 费用。只是你的服务器要跑 1 分钟左右的音频识别。

---

## 2. 什么时候会烧 token？

主要有 3 种情况。

### 情况一：你用云端 LLM 生成回答

例如：

```text
用户说：“帮我查一下订单”
ASR 转成文字
LLM 处理这句话并生成回复
```

这时烧的是 **文字 token**。

比如：

```text
输入 token：
用户问题 + 系统提示词 + 历史对话 + 工具返回内容

输出 token：
AI 回复文字
```

这个成本一般比直接处理音频便宜很多，因为用户 10 秒语音转成文字可能只有几十个汉字。

---

### 情况二：你用云端 TTS

如果你不用 Android 本地 TTS，也不用自部署 TTS，而是调用云 API 把文字转语音，那就可能按：

```text
字符数
语音分钟数
audio tokens
```

来收费。

OpenAI 现在的 TTS / realtime audio 模型确实存在 audio token 或按分钟计费的模式，官方价格页显示 GPT-Realtime-2 的 audio input 是 `$32 / 1M tokens`，audio output 是 `$64 / 1M tokens`；GPT-Realtime-Whisper 是 `$0.017 / minute`。([OpenAI][1])

---

### 情况三：你直接用实时语音大模型

例如这种：

```text
APK 直接把用户语音流发给 GPT Realtime
GPT 直接听语音、思考、再输出语音
```

这种体验最好，但也最容易烧钱。OpenAI 的 realtime 成本说明里写到，用户音频消息大约是 **100ms 一个 audio token**，助手音频大约是 **50ms 一个 audio token**。([OpenAI 开发者][2])

所以如果用户长时间开麦，或者 AI 语音回复很长，成本会明显上升。

---

## 3. 我建议你不要这么做

不建议：

```text
APK → GPT Realtime Audio → 直接语音回答
```

原因：

```text
语音输入烧 audio token
语音输出也烧 audio token
用户沉默、噪声、误触也可能产生消耗
长对话成本不好控制
调试和降本都比较麻烦
```

这个架构适合资金比较充足、追求极致实时体验的产品，不适合你现在这个阶段。

---

## 4. 我建议你的低成本架构

### 方案 A：最低成本 MVP

```text
APK 录音
  ↓
服务器自部署 ASR
  ↓
LLM 生成文字回答
  ↓
APK 使用 Android TextToSpeech 朗读
```

成本结构：

```text
ASR：不烧 token
TTS：不烧 token
LLM：烧少量文字 token
```

这个方案最适合你先上线。

---

### 方案 B：中文效果更好，多语言也能兜底

```text
APK 录音
  ↓
ASR Router
  ├─ 中文 / 粤语 / 中英混说：FunASR 或 SenseVoice
  └─ 其他语言 / 未知语言：faster-whisper large-v3 / turbo
  ↓
LLM 文字回答
  ↓
APK 本地 TTS
```

成本结构：

```text
ASR：服务器成本
LLM：文字 token
TTS：手机本地成本
```

这个是我最推荐你的版本。

---

### 方案 C：正式商业版，声音更自然

```text
APK 录音
  ↓
自部署 ASR
  ↓
LLM 文字回答
  ↓
自部署 TTS：CosyVoice / Kokoro / sherpa-onnx
  ↓
APK 播放服务器返回的音频
```

成本结构：

```text
ASR：服务器成本
LLM：文字 token
TTS：服务器成本
```

这个版本比 Android 自带 TTS 好听，但你要多部署一个 TTS 服务。

---

## 5. 成本控制重点

你真正要控制的是 **LLM token**，不是 ASR token。

建议你这样做：

```text
1. ASR partial 结果不要发给 LLM
2. 只把 final 识别结果发给 LLM
3. 历史对话不要无限塞进 prompt
4. 每轮只保留最近 3~6 轮对话
5. 老历史做摘要
6. 限制 AI 回复长度
7. 常见问题缓存回答
8. TTS 只读最终回复，不读调试信息
9. 用户静音时不要上传
10. 用 VAD 自动切断无效音频
```

比如用户说：

```text
“今天有什么安排？”
```

不要给 LLM 发：

```text
系统提示词 3000 字
用户历史 50 轮
ASR partial 结果 10 条
数据库全部内容
```

而是发：

```text
系统提示词简短版
最近 3 轮对话
用户最终识别文字
必要的数据库查询结果
```

---

## 6. 一个很实用的成本分层

你可以把功能分成 3 档。

### 免费/低成本档

```text
ASR：自部署 faster-whisper / FunASR / SenseVoice
LLM：便宜文本模型
TTS：Android 本地 TTS
```

适合：

```text
普通用户
测试用户
低付费用户
```

---

### 标准档

```text
ASR：自部署 FunASR / SenseVoice + faster-whisper 兜底
LLM：中等文本模型
TTS：服务器自部署 Kokoro / CosyVoice
```

适合：

```text
正式用户
中文语音助手
客服机器人
```

---

### 高级档

```text
ASR/TTS/LLM：可以部分使用云端 realtime voice
```

适合：

```text
高付费用户
需要非常自然实时语音的场景
```

但这一档一定要加：

```text
每日语音分钟限制
单次对话时长限制
最大回复时长
用户余额/额度
超时自动断开
```

---

## 7. 最终建议

你的产品现在应该采用：

```text
自部署 ASR
+
文字 LLM
+
APK 本地 TTS
```

也就是：

```text
FunASR / SenseVoice / faster-whisper
    ↓
文字
    ↓
LLM
    ↓
文字
    ↓
Android TextToSpeech
```

这套架构的好处是：

```text
语音识别不烧 token
语音播放不烧 token
只有真正需要智能回答的文字才烧 token
成本最容易控制
后面也方便升级成服务器 TTS
```

一句话：

**不要让大模型直接听音频、直接吐音频。先 ASR 成文字，再让 LLM 处理文字，最后本地 TTS 朗读。这样最省 token。**

[1]: https://openai.com/api/pricing/?utm_source=chatgpt.com "API Pricing"
[2]: https://developers.openai.com/api/docs/guides/realtime-costs?utm_source=chatgpt.com "Managing costs | OpenAI API"

