export interface VoiceItem {
  id: string
  label: string
  description?: string
}

export interface TtsCatalog {
  voices: VoiceItem[]
  emotions: VoiceItem[]
  intensities: VoiceItem[]
  workerConfigured?: boolean
  defaultProvider?: string
}

export interface TtsStatus {
  running?: boolean
  health?: {
    defaultProvider?: string
  }
}

export interface RelayConfig {
  ttsWorkerUrl?: string
}

export type VoiceChannel = 'studio' | 'training' | 'sdk'

export interface TrainingPlan {
  name: string
  engine: string
  samples: string
  assetRoot: string
}
