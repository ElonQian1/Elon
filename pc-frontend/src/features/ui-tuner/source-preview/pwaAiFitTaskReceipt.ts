import type { AiWritebackReceipt } from './aiWritebackReceipt'
import { buildPwaDraftAiFitTaskContractId } from './pwaAiFitTask'
import type { PwaDesignDraft } from './pwaDesignDraft'

export function validatePwaAiFitTaskReceipt(
  draft: PwaDesignDraft,
  receipt: AiWritebackReceipt,
): string {
  const expected = buildPwaDraftAiFitTaskContractId(draft)
  if (receipt.aiFitTaskContractId !== expected) {
    return `AI 回执未声明当前 PWA 拟合任务合约；期望 ${expected}，实际 ${receipt.aiFitTaskContractId || '缺失'}`
  }
  if (receipt.aiFitTaskHonored !== true) {
    return 'AI 回执未声明 aiFitTaskHonored=true；不能确认已按低 Token 拟合任务执行'
  }
  return ''
}
