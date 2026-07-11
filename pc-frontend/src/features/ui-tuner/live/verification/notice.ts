import type { LiveBuildVerifyResult } from '../liveUiApi'

export function buildVerificationNotice(result: LiveBuildVerifyResult) {
  const build = result.runtimeBuildId ?? '新 Debug APK'
  if (result.status === 'BUILD_VERIFIED') {
    return `BUILD VERIFIED：${build} 已安装，源码一致性和目标设计门禁均通过`
  }
  if (result.status === 'TARGET_MISMATCH') {
    return `TARGET MISMATCH：源码已经复现 Live 预览，但目标设计仍有差异 ${loss(result.visualDiff)}`
  }
  if (result.status === 'TARGET_NOT_CONFIGURED') {
    return 'TARGET NOT CONFIGURED：源码一致性已通过，请先在左侧框选设计区域并与右侧 Runtime Node 配对'
  }
  return result.sourceParityDiff
    ? `SOURCE MISMATCH：源码构建结果与 Live 预览仍有差异 ${loss(result.sourceParityDiff)}`
    : 'SOURCE MISMATCH：本机节点未返回源码一致性结果，请更新 Windows PC 节点后重试'
}

function loss(diff?: { visualLoss: number }) {
  return diff ? diff.visualLoss.toFixed(4) : '未知'
}
