import type { UiTunerDocument, UiTunerElement } from '../types'

export function buildCliPatchPackage(document: UiTunerDocument) {
  const runtimeElements = document.elements
    .filter((element) => element.runtime)
    .map((element) => runtimePatchEntry(element))

  return {
    version: 1,
    kind: 'elon_ui_tuner_cli_patch_request',
    goal: '根据微调画布导出的运行时 XML、截图坐标和源码映射，修改 APK UI 源码并完成构建验证。',
    source: document.source,
    runtimeSnapshot: document.runtimeSnapshot,
    canvas: {
      width: document.canvas.width,
      height: document.canvas.height,
      referenceImage: document.canvas.referenceImage
        ? {
            name: document.canvas.referenceImage.name,
            width: document.canvas.referenceImage.width,
            height: document.canvas.referenceImage.height,
            visible: document.canvas.referenceImage.visible,
            opacity: document.canvas.referenceImage.opacity,
          }
        : null,
    },
    elements: runtimeElements,
    instructions: [
      '优先修改 source.file 指向的 Android XML / values token；没有 source.file 时先按 resourceId 在 android/app/src/main/res 下搜索。',
      '保留业务逻辑和资源命名，不要直接写死截图坐标；把坐标变化转换成 margin、padding、height、textSize 或约束关系。',
      '修改后运行 Android 或仓库要求的最小验证；涉及可安装端时按发布脚本完成 APK 闭环。',
    ],
    exportedAt: new Date().toISOString(),
  }
}

export function stringifyCliPatchPackage(document: UiTunerDocument) {
  return JSON.stringify(buildCliPatchPackage(document), null, 2)
}

function runtimePatchEntry(element: UiTunerElement) {
  const runtime = element.runtime!
  const original = runtime.originalBounds
  const current = {
    left: element.x,
    top: element.y,
    right: element.x + element.width,
    bottom: element.y + element.height,
    width: element.width,
    height: element.height,
  }
  return {
    id: element.id,
    name: element.name,
    resourceId: runtime.resourceId,
    className: runtime.className,
    xpath: runtime.xpath,
    indexPath: runtime.indexPath,
    source: element.source,
    originalBounds: original,
    currentBounds: current,
    delta: {
      x: current.left - original.left,
      y: current.top - original.top,
      width: current.width - original.width,
      height: current.height - original.height,
    },
    style: {
      text: element.text,
      fontSize: element.fontSize,
      lineHeight: element.lineHeight,
      fontWeight: element.fontWeight,
      paddingX: element.paddingX,
      paddingY: element.paddingY,
      borderRadius: element.borderRadius,
      color: element.color,
      background: element.background,
      opacity: element.opacity,
    },
  }
}
