import { useState, useEffect } from 'react'
import { api } from '../../api/client'
import { localJson } from '../doctor/localApi'
import { safeNodeAdminUrl } from '../../lib/utils'
import type { TtsCatalog, TtsStatus, RelayConfig, VoiceChannel, TrainingPlan } from './types'
import styles from './VoicePage.module.css'

const PLAN_KEY = 'elon_voice_training_plan'

function loadPlan(): TrainingPlan {
  try { return JSON.parse(localStorage.getItem(PLAN_KEY) ?? '{}') as TrainingPlan } catch { return {} as TrainingPlan }
}

export default function VoicePage() {
  const [channel, setChannel] = useState<VoiceChannel>('studio')

  return (
    <div className={styles.page}>
      <nav className={styles.tabs}>
        {(['studio', 'training', 'sdk'] as VoiceChannel[]).map((ch) => (
          <button
            key={ch}
            className={[styles.tab, channel === ch ? styles.activeTab : ''].join(' ')}
            onClick={() => setChannel(ch)}
          >
            {{ studio: '🎙️ 声音控制台', training: '🏋️ 训练方案', sdk: '🔌 对外 SDK' }[ch]}
          </button>
        ))}
      </nav>

      {channel === 'studio' && <StudioPanel />}
      {channel === 'training' && <TrainingPanel />}
      {channel === 'sdk' && <SdkPanel />}
    </div>
  )
}

function StudioPanel() {
  const nodeAdminUrl = safeNodeAdminUrl()
  const [catalog, setCatalog] = useState<TtsCatalog | null>(null)
  const [status, setStatus] = useState<TtsStatus | null>(null)
  const [, setRelay] = useState<RelayConfig>({})
  const [loading, setLoading] = useState(true)

  const [previewText, setPreviewText] = useState('你好，我是一龙的 ai 声音。今天想用什么情绪陪你说话？')
  const [voiceId, setVoiceId] = useState('female_warm')
  const [emotionId, setEmotionId] = useState('normal')
  const [intensity, setIntensity] = useState('normal')
  const [previewResult, setPreviewResult] = useState('')

  const [relayUrl, setRelayUrl] = useState('')
  const [relayResult, setRelayResult] = useState('')

  useEffect(() => {
    Promise.allSettled([
      api.get<TtsCatalog>('/api/voice/tts/catalog'),
      localJson<TtsStatus>(nodeAdminUrl, '/api/tts-status'),
      localJson<RelayConfig>(nodeAdminUrl, '/api/tts-relay-config'),
    ]).then(([catalogRes, statusRes, relayRes]) => {
      if (catalogRes.status === 'fulfilled') setCatalog(catalogRes.value)
      if (statusRes.status === 'fulfilled') setStatus(statusRes.value)
      if (relayRes.status === 'fulfilled') {
        setRelay(relayRes.value)
        setRelayUrl(relayRes.value.ttsWorkerUrl ?? '')
      }
    }).finally(() => setLoading(false))
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  async function saveRelay() {
    setRelayResult('保存中…')
    try {
      const data = await localJson<RelayConfig>(nodeAdminUrl, '/api/tts-relay-config', {
        method: 'POST', body: JSON.stringify({ tts_worker_url: relayUrl || null }),
      })
      setRelayResult(data.ttsWorkerUrl ? `已启用中继：${data.ttsWorkerUrl}` : '已禁用中继')
    } catch (err) {
      setRelayResult(`保存失败：${(err as Error).message}`)
    }
  }

  async function playPreview() {
    setPreviewResult('合成中…')
    try {
      const result = await api.post<{ provider?: string; voice?: string; emotion?: string }>(
        '/api/voice/tts',
        { text: previewText, voiceId, emotionId, intensity },
      )
      setPreviewResult(`播放中：${result.provider ?? 'tts'} / ${result.voice ?? voiceId} / ${result.emotion ?? emotionId}`)
    } catch (err) {
      setPreviewResult(`试听失败：${(err as Error).message}`)
    }
  }

  if (loading) return <div className={styles.loading}>正在读取云端目录和本机 TTS Worker 状态…</div>

  const voices = catalog?.voices ?? []
  const emotions = catalog?.emotions ?? []
  const intensities = catalog?.intensities ?? []
  const workerOk = !!(catalog?.workerConfigured)
  const localRunning = !!(status?.running)
  const provider = status?.health?.defaultProvider ?? catalog?.defaultProvider ?? 'auto'

  return (
    <div className={styles.content}>
      <div className={styles.hero}>
        <section className={styles.panel}>
          <h3>ai声音</h3>
          <p>这里集中管理情绪女声 TTS：云端目录、PC 本机模型 Worker、中继地址、试听和外部 SDK。</p>
          <div className={styles.statusRow}>
            <span className={[styles.badge, workerOk ? styles.ok : styles.bad].join(' ')}>
              {workerOk ? '云端可合成' : '云端未配置'}
            </span>
            <span className={[styles.badge, localRunning ? styles.ok : styles.warn].join(' ')}>
              {localRunning ? '本机 Worker 运行中' : '本机 Worker 未运行'}
            </span>
            <span className={styles.badge}>引擎 {provider}</span>
          </div>
        </section>

        <section className={styles.panel}>
          <h3>本机 TTS 中继</h3>
          <p>把本机 GPU / 模型 Worker 贡献给平台。默认模型 Worker 地址是 <code>http://127.0.0.1:5011</code>。</p>
          <label className={styles.field}>
            <span>本机 TTS Worker 地址</span>
            <input value={relayUrl} onChange={(e) => setRelayUrl(e.target.value)} placeholder="http://127.0.0.1:5011" />
          </label>
          <div className={styles.actionRow}>
            <button className={styles.btn} onClick={saveRelay}>保存中继</button>
            <button className={[styles.btn, styles.secondary].join(' ')} onClick={() => window.location.reload()}>刷新</button>
          </div>
          {relayResult && <p className={styles.result}>{relayResult}</p>}
        </section>
      </div>

      <div className={styles.hero}>
        <section className={styles.panel}>
          <h3>试听</h3>
          <label className={styles.field}>
            <span>朗读文本</span>
            <textarea rows={4} value={previewText} onChange={(e) => setPreviewText(e.target.value)} />
          </label>
          <div className={styles.grid3}>
            <label className={styles.field}>
              <span>声线</span>
              <select value={voiceId} onChange={(e) => setVoiceId(e.target.value)}>
                {voices.map((v) => <option key={v.id} value={v.id}>{v.label}</option>)}
              </select>
            </label>
            <label className={styles.field}>
              <span>情绪</span>
              <select value={emotionId} onChange={(e) => setEmotionId(e.target.value)}>
                {emotions.map((v) => <option key={v.id} value={v.id}>{v.label}</option>)}
              </select>
            </label>
            <label className={styles.field}>
              <span>强度</span>
              <select value={intensity} onChange={(e) => setIntensity(e.target.value)}>
                {intensities.map((v) => <option key={v.id} value={v.id}>{v.label}</option>)}
              </select>
            </label>
          </div>
          <div className={styles.actionRow}>
            <button className={styles.btn} onClick={playPreview}>试听声音</button>
          </div>
          {previewResult && <p className={styles.result}>{previewResult}</p>}
        </section>

        <section className={styles.panel}>
          <h3>声线目录</h3>
          <div className={styles.voiceList}>
            {voices.slice(0, 6).map((v) => (
              <div key={v.id} className={styles.voiceItem}>
                <strong>{v.label}</strong>
                <span>{v.description}</span>
              </div>
            ))}
            {voices.length === 0 && <p>暂无声线目录</p>}
          </div>
        </section>
      </div>
    </div>
  )
}

function TrainingPanel() {
  const [plan, setPlan] = useState<TrainingPlan>(loadPlan)
  const [saved, setSaved] = useState(false)

  function save() {
    localStorage.setItem(PLAN_KEY, JSON.stringify(plan))
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  const assetRoot = plan.assetRoot || 'D:\\tts-assets'
  const samples = plan.samples || 'D:\\authorized-tts-samples'
  const engine = plan.engine || 'index_tts2'

  return (
    <div className={styles.content}>
      <div className={styles.hero}>
        <section className={styles.panel}>
          <h3>训练方案</h3>
          <label className={styles.field}><span>声音名称</span>
            <input value={plan.name ?? ''} onChange={(e) => setPlan({ ...plan, name: e.target.value })} />
          </label>
          <label className={styles.field}><span>模型引擎</span>
            <select value={engine} onChange={(e) => setPlan({ ...plan, engine: e.target.value })}>
              <option value="index_tts2">IndexTTS2：强情绪参考</option>
              <option value="cosyvoice3">CosyVoice3：自然克隆</option>
              <option value="gpt_sovits">GPT-SoVITS：自定义训练</option>
            </select>
          </label>
          <label className={styles.field}><span>授权声音素材目录</span>
            <input value={samples} onChange={(e) => setPlan({ ...plan, samples: e.target.value })} />
          </label>
          <label className={styles.field}><span>资产输出目录</span>
            <input value={assetRoot} onChange={(e) => setPlan({ ...plan, assetRoot: e.target.value })} />
          </label>
          <div className={styles.actionRow}>
            <button className={styles.btn} onClick={save}>{saved ? '已保存 ✓' : '保存训练方案'}</button>
          </div>
        </section>

        <section className={styles.panel}>
          <h3>训练步骤</h3>
          <div className={styles.voiceList}>
            <div className={styles.voiceItem}><strong>1. 授权采样</strong><span>只上传本人或已授权的 10-30 分钟干净人声，不使用明星、主播或声优声音。</span></div>
            <div className={styles.voiceItem}><strong>2. 导入资产</strong><span>用导入脚本转成 24kHz mono wav，并生成 voices / emotions 目录。</span></div>
            <div className={styles.voiceItem}><strong>3. 启动 Worker</strong><span>本机运行模型 Worker，再回到声音控制台启用中继。</span></div>
          </div>
        </section>
      </div>

      <section className={styles.panel}>
        <h3>推荐命令</h3>
        <pre className={styles.code}>{`powershell -ExecutionPolicy Bypass -File scripts\\import-tts-asset-pack.ps1 -AssetRoot "${assetRoot}" -SourceDir "${samples}" -FailOnMissing
powershell -ExecutionPolicy Bypass -File scripts\\start-local-model-tts-worker.ps1 -Provider ${engine} -AssetRoot "${assetRoot}"`}</pre>
      </section>
    </div>
  )
}

function SdkPanel() {
  const [copied, setCopied] = useState(false)
  const code = `<script src="${location.origin}/assets/voice_tts_sdk.js"><\/script>
<script>
const tts = ElonVoiceTts.createClient({
  baseUrl: '${location.origin}',
  token: '<用户登录 token>'
});
await tts.play({
  text: '你好，我是你的 ai 声音。',
  voiceId: 'female_warm',
  emotionId: 'gentle_comfort',
  intensity: 'normal'
});
<\/script>`

  async function copy() {
    await navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className={styles.content}>
      <div className={styles.hero}>
        <section className={styles.panel}>
          <h3>外部服务能力</h3>
          <p><code>GET /api/voice/tts/catalog</code> 返回声线、情绪、强度目录。</p>
          <p><code>POST /api/voice/tts</code> 返回音频，支持云端 Worker 或用户 PC 节点模型 Worker。</p>
          <p><code>/assets/voice_tts_sdk.js</code> 封装鉴权、目录读取、音频合成和浏览器播放。</p>
        </section>
        <section className={styles.panel}>
          <h3>接入示例</h3>
          <pre className={styles.code}>{code}</pre>
          <div className={styles.actionRow}>
            <button className={styles.btn} onClick={copy}>{copied ? '已复制 ✓' : '复制示例'}</button>
          </div>
        </section>
      </div>
    </div>
  )
}
