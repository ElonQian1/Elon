import type { LocalAiResearchCaptureStatus } from './localAiBrowserApi'

export interface LocalAiResearchCompatibilityNotice {
  title: string
  detail: string
}

export function localAiResearchCompatibilityNotice(
  status: LocalAiResearchCaptureStatus | undefined,
): LocalAiResearchCompatibilityNotice | null {
  if (!status || status.captureCount === 0) return null
  if (status.compatibility === 'renderer_upgrade_required') {
    return {
      title: '官网富内容结构已升级，Win 渲染适配待更新',
      detail: `已发现 ${status.unsupportedRichCount} 类当前 Win 渲染器尚未支持的私有组件；正文和已识别卡片继续显示，完整交互内容可在官网页查看。`,
    }
  }
  if (status.compatibility === 'structure_observed') {
    return {
      title: 'Google 富内容结构已识别，当前版本仍在适配',
      detail: '已解码官网私有响应，但尚未建立稳定的正文与富卡字段映射；当前继续使用官网 DOM 和原生卡片回退，不会把占位误报为完整内容。',
    }
  }
  if (['upstream_changed', 'parse_error', 'incomplete'].includes(status.compatibility)) {
    return {
      title: '官网回答结构可能已变化，Win 解析适配待更新',
      detail: `最近私有响应识别 ${status.acceptedFrameCount}/${status.decodedFrameCount} 帧；正文继续使用安全回退，建议更新 Win 客户端或查看官网完整内容。`,
    }
  }
  return null
}
