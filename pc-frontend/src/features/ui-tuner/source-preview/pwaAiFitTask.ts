import { buildPwaDraftCliCompactHandoff, type PwaDesignDraft } from './pwaDesignDraft'

export interface PwaDraftAiFitTask {
  version: 1
  kind: 'elon.pwa.ai_fit_task'
  contractId: string
  generatedAt: string
  purpose: 'low-token-visual-fit-and-cross-platform-writeback'
  route: PwaDesignDraft['route']
  viewport: PwaDesignDraft['viewport']
  sourceRevision: string
  visualEvidence: PwaDesignDraft['visualReferences'] & {
    screenshotsEmbeddedAsBase64: false
  }
  summary: {
    changedElementCount: number
    changedPropertyCount: number
    bindingGapCount: number
    sourceFileCandidateCount: number
  }
  executionPolicy: {
    defaultRepositoryScan: false
    preferBoundStyleData: true
    useVisualSolverBeforeCodexSourceEdit: true
    requirePatchFreeVerification: true
  }
  compactHandoff: ReturnType<typeof buildPwaDraftCliCompactHandoff>
  instructions: string[]
}

export function buildPwaDraftAiFitTask(draft: PwaDesignDraft): PwaDraftAiFitTask {
  const compactHandoff = buildPwaDraftCliCompactHandoff(draft)
  const changedPropertyCount = compactHandoff.elements.reduce((total, element) => total + element.changedProperties.length, 0)
  const bindingGapCount = compactHandoff.elements.filter((element) => element.binding.needsBinding).length
  return {
    version: 1,
    kind: 'elon.pwa.ai_fit_task',
    generatedAt: new Date().toISOString(),
    purpose: 'low-token-visual-fit-and-cross-platform-writeback',
    route: draft.route,
    viewport: draft.viewport,
    sourceRevision: draft.project.sourceRevision,
    visualEvidence: {
      ...draft.visualReferences,
      screenshotsEmbeddedAsBase64: false,
    },
    summary: {
      changedElementCount: compactHandoff.elements.length,
      changedPropertyCount,
      bindingGapCount,
      sourceFileCandidateCount: compactHandoff.sourceFilesToInspect.length,
    },
    executionPolicy: {
      defaultRepositoryScan: false,
      preferBoundStyleData: true,
      useVisualSolverBeforeCodexSourceEdit: true,
      requirePatchFreeVerification: true,
    },
    compactHandoff,
    instructions: [
      '这是设计稿拟合任务，不是普通源码问答；先读取 compactHandoff 和 visualEvidence 路径，不要默认读取整仓库。',
      '优先使用已绑定样式、Token、Style JSON、PWA 候选和 Android 候选；只有 bindingGapCount 指出的缺口才请求更多源码。',
      '先用平台的草稿/拟合/差异工具收敛尺寸、圆角、颜色、字号和间距，再让 Codex 处理结构或缺失绑定。',
      '每轮只传目标裁剪、当前裁剪、diff 路径或数值摘要；不要把图片转成 Base64 塞进提示词。',
      '最终必须写回 PWA 与 APK 的源码数据，并在清空临时草稿后做真实运行验证。',
    ],
  }
}

export function stringifyPwaDraftAiFitTask(draft: PwaDesignDraft): string {
  return JSON.stringify(buildPwaDraftAiFitTask(draft), null, 2)
}
