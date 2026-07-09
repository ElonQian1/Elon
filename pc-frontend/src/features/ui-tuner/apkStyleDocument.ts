import colorsXml from '../../../../android/app/src/main/res/values/colors.xml?raw'
import dimensXml from '../../../../android/app/src/main/res/values/dimens.xml?raw'
import themesXml from '../../../../android/app/src/main/res/values/themes.xml?raw'
import activityMainXml from '../../../../android/app/src/main/res/layout/activity_main.xml?raw'
import type { UiTunerDocument, UiTunerElement, UiTunerElementKind, UiTunerSource } from './types'

const COLORS_PATH = 'android/app/src/main/res/values/colors.xml'
const DIMENS_PATH = 'android/app/src/main/res/values/dimens.xml'
const THEMES_PATH = 'android/app/src/main/res/values/themes.xml'
const ACTIVITY_MAIN_PATH = 'android/app/src/main/res/layout/activity_main.xml'

interface NamedXmlValue {
  name: string
  value: string
  file: string
  line: number
}

interface LayoutNodeSnapshot {
  id: string
  tag: string
  file: string
  line: number
  attrs: Record<string, string>
}

const now = () => new Date().toISOString()

function source(label: string, file: string, line?: number, token?: string, rawValue?: string): UiTunerSource {
  return {
    kind: 'apk',
    label,
    file,
    line,
    token,
    rawValue,
  }
}

function lineAt(text: string, index: number) {
  return text.slice(0, Math.max(0, index)).split(/\r?\n/).length
}

function stableHash(input: string) {
  let hash = 2166136261
  for (let index = 0; index < input.length; index += 1) {
    hash ^= input.charCodeAt(index)
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(16).padStart(8, '0')
}

function readNamedValues(xml: string, file: string, tag: string) {
  const values = new Map<string, NamedXmlValue>()
  const matcher = new RegExp(`<${tag}\\s+name="([^"]+)"[^>]*>([^<]+)<\\/${tag}>`, 'g')
  for (const match of xml.matchAll(matcher)) {
    const name = match[1]
    const value = match[2].trim()
    values.set(name, {
      name,
      value,
      file,
      line: lineAt(xml, match.index ?? 0),
    })
  }
  return values
}

const colors = readNamedValues(colorsXml, COLORS_PATH, 'color')
const dimens = readNamedValues(dimensXml, DIMENS_PATH, 'dimen')

function color(name: string, fallback: string) {
  return colors.get(name)?.value ?? fallback
}

function colorSource(name: string, label?: string) {
  const value = colors.get(name)
  return source(label ?? `@color/${name}`, value?.file ?? COLORS_PATH, value?.line, `@color/${name}`, value?.value)
}

function dimenValue(name: string, fallback: number) {
  const value = dimens.get(name)?.value
  if (!value) return fallback
  return parseAndroidSize(value, fallback)
}

function dimenSource(name: string, label?: string) {
  const value = dimens.get(name)
  return source(label ?? `@dimen/${name}`, value?.file ?? DIMENS_PATH, value?.line, `@dimen/${name}`, value?.value)
}

function parseAndroidSize(value: string | undefined, fallback: number) {
  if (!value) return fallback
  const match = value.trim().match(/^(-?\d+(?:\.\d+)?)(dp|sp|px)?$/)
  if (!match) return fallback
  return Number(match[1])
}

function resolveSize(value: string | undefined, fallback: number) {
  if (!value) return fallback
  if (value.startsWith('@dimen/')) return dimenValue(value.slice('@dimen/'.length), fallback)
  return parseAndroidSize(value, fallback)
}

function styleBlock(styleName: string) {
  const matcher = new RegExp(`<style\\s+name="${styleName}"[^>]*>([\\s\\S]*?)<\\/style>`)
  const match = themesXml.match(matcher)
  return match?.[1] ?? ''
}

function styleItem(styleName: string, itemName: string) {
  const block = styleBlock(styleName)
  const matcher = new RegExp(`<item\\s+name="${itemName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}">([^<]+)<\\/item>`)
  return block.match(matcher)?.[1]?.trim()
}

function styleLine(styleName: string) {
  const index = themesXml.indexOf(`<style name="${styleName}"`)
  return index >= 0 ? lineAt(themesXml, index) : undefined
}

function readNodeById(id: string): LayoutNodeSnapshot | null {
  const idIndex = activityMainXml.indexOf(`android:id="@+id/${id}"`)
  if (idIndex < 0) return null
  const start = activityMainXml.lastIndexOf('<', idIndex)
  const end = activityMainXml.indexOf('>', idIndex)
  if (start < 0 || end < 0) return null
  const openTag = activityMainXml.slice(start, end + 1)
  const tag = openTag.match(/^<([\w.]+)/)?.[1] ?? 'View'
  const attrs: Record<string, string> = {}
  for (const match of openTag.matchAll(/(?:android:|app:)?([\w:]+)="([^"]*)"/g)) {
    attrs[match[1]] = match[2]
  }
  return {
    id,
    tag,
    file: ACTIVITY_MAIN_PATH,
    line: lineAt(activityMainXml, start),
    attrs,
  }
}

function nodeSource(node: LayoutNodeSnapshot | null, label: string) {
  return source(label, node?.file ?? ACTIVITY_MAIN_PATH, node?.line, node ? `@+id/${node.id}` : undefined)
}

function element(
  id: string,
  name: string,
  kind: UiTunerElementKind,
  overrides: Partial<UiTunerElement>,
): UiTunerElement {
  return {
    id,
    name,
    kind,
    x: 24,
    y: 24,
    width: 160,
    height: 48,
    text: name,
    fontSize: 14,
    lineHeight: 20,
    fontWeight: 600,
    letterSpacing: 0,
    paddingX: 12,
    paddingY: 10,
    borderRadius: 6,
    borderWidth: 1,
    color: color('elon_text_primary', '#D9D9D9'),
    background: color('elon_surface_header', '#1F2023'),
    borderColor: color('elon_border_subtle', '#4D4D4D'),
    opacity: 1,
    ...overrides,
  }
}

const sourceFiles = [COLORS_PATH, DIMENS_PATH, THEMES_PATH, ACTIVITY_MAIN_PATH]
export const APK_STYLE_SOURCE_SIGNATURE = stableHash([
  colorsXml,
  dimensXml,
  themesXml,
  activityMainXml,
].join('\n--- apk-style-source ---\n'))

export function createApkStyleDocument(): UiTunerDocument {
  const toolbar = readNodeById('toolbar')
  const projectTabs = readNodeById('projectTopTabs')
  const addButton = readNodeById('addButton')
  const contentContainer = readNodeById('contentContainer')
  const inputLayout = readNodeById('inputLayout')
  const pageTabs = readNodeById('pageTabs')

  const canvasWidth = 390
  const canvasHeight = 844
  const toolbarHeight = resolveSize(toolbar?.attrs.layout_height, 50)
  const bottomOuterHeight = resolveSize(pageTabs?.attrs.layout_height, dimenValue('main_bottom_menu_outer_height', 90))
  const bottomPanelY = canvasHeight - bottomOuterHeight
  const inputHeight = 64
  const mainTabTextSize = parseAndroidSize(styleItem('MainTabText', 'android:textSize'), 14)
  const mainTabHeight = parseAndroidSize(styleItem('MainTabText', 'android:layout_height'), 62)
  const tabLabels = ['好友', '项目', '我的', '商城', 'Agent']
  const tabWidth = (canvasWidth - 40) / tabLabels.length

  const documentSource: UiTunerSource = {
    kind: 'apk',
    label: '当前 APK 样式源码',
    signature: APK_STYLE_SOURCE_SIGNATURE,
    files: sourceFiles,
  }

  return {
    version: 1,
    updatedAt: now(),
    source: documentSource,
    canvas: {
      name: '当前 APK 样式画布',
      width: canvasWidth,
      height: canvasHeight,
      background: color('elon_bg_app', '#000000'),
      source: colorSource('elon_bg_app', '画布背景 @color/elon_bg_app'),
    },
    elements: [
      element('apk.toolbar', '顶部 Toolbar', 'card', {
        x: 0,
        y: 0,
        width: canvasWidth,
        height: toolbarHeight,
        text: `#toolbar\n${toolbarHeight}dp / @color/elon_bg_app`,
        fontSize: 12,
        lineHeight: 17,
        fontWeight: 700,
        paddingX: 18,
        paddingY: 8,
        borderRadius: 0,
        borderWidth: 0,
        color: color('elon_text_primary', '#D9D9D9'),
        background: color('elon_bg_app', '#000000'),
        borderColor: color('elon_bg_app', '#000000'),
        source: nodeSource(toolbar, 'activity_main.xml #toolbar'),
      }),
      element('apk.projectTopTabs', '项目顶部 Tab', 'text', {
        x: 24,
        y: 7,
        width: 228,
        height: 36,
        text: `我的项目     项目广场\nTab ${styleItem('MainTabText', 'android:textSize') ?? '14sp'} / 下划线 32x2dp`,
        fontSize: 16,
        lineHeight: 18,
        fontWeight: 800,
        paddingX: 0,
        paddingY: 0,
        borderWidth: 0,
        color: color('elon_text_primary', '#D9D9D9'),
        background: color('elon_bg_app', '#000000'),
        borderColor: color('elon_bg_app', '#000000'),
        source: nodeSource(projectTabs, 'activity_main.xml #projectTopTabs'),
      }),
      element('apk.topAddButton', '顶部新增按钮', 'button', {
        x: canvasWidth - 56,
        y: 3,
        width: resolveSize(addButton?.attrs.layout_width, 44),
        height: resolveSize(addButton?.attrs.layout_height, 44),
        text: '+',
        fontSize: 24,
        lineHeight: 24,
        fontWeight: 500,
        paddingX: 0,
        paddingY: 0,
        borderRadius: 999,
        borderWidth: 1,
        color: color('elon_icon_add_top', '#D3D3D3'),
        background: color('elon_surface_float', '#212121'),
        borderColor: color('elon_border_subtle', '#4D4D4D'),
        source: nodeSource(addButton, 'activity_main.xml #addButton'),
      }),
      element('apk.contentCanvas', '内容画布', 'card', {
        x: 0,
        y: toolbarHeight,
        width: canvasWidth,
        height: bottomPanelY - toolbarHeight,
        text: `#contentContainer\n@color/elon_bg_app`,
        fontSize: 13,
        lineHeight: 20,
        fontWeight: 700,
        paddingX: 20,
        paddingY: 24,
        borderRadius: 0,
        borderWidth: 0,
        color: color('elon_text_secondary', '#B8B8B8'),
        background: color('elon_bg_app', '#000000'),
        borderColor: color('elon_bg_app', '#000000'),
        source: nodeSource(contentContainer, 'activity_main.xml #contentContainer'),
      }),
      element('apk.surfaceCardToken', '@color/elon_surface_card', 'card', {
        x: 20,
        y: toolbarHeight + 42,
        width: 160,
        height: 94,
        text: `卡片主体\n${color('elon_surface_card', '#1A1A1A')}`,
        fontSize: 15,
        lineHeight: 22,
        fontWeight: 800,
        paddingX: 16,
        paddingY: 14,
        borderRadius: 18,
        color: color('elon_text_primary', '#D9D9D9'),
        background: color('elon_surface_card', '#1A1A1A'),
        borderColor: color('elon_border_subtle', '#4D4D4D'),
        source: colorSource('elon_surface_card'),
      }),
      element('apk.surfaceHeaderToken', '@color/elon_surface_header', 'card', {
        x: 210,
        y: toolbarHeight + 42,
        width: 160,
        height: 94,
        text: `卡片头部\n${color('elon_surface_header', '#1F2023')}`,
        fontSize: 15,
        lineHeight: 22,
        fontWeight: 800,
        paddingX: 16,
        paddingY: 14,
        borderRadius: 18,
        color: color('elon_text_primary', '#D9D9D9'),
        background: color('elon_surface_header', '#1F2023'),
        borderColor: color('elon_border_subtle', '#4D4D4D'),
        source: colorSource('elon_surface_header'),
      }),
      element('apk.primaryButtonToken', '@color/elon_button_primary_bg', 'button', {
        x: 20,
        y: toolbarHeight + 170,
        width: 160,
        height: 48,
        text: `主按钮 ${color('elon_button_primary_bg', '#FFFFFF')}`,
        fontSize: 14,
        lineHeight: 20,
        fontWeight: 800,
        paddingX: 14,
        paddingY: 10,
        borderRadius: 24,
        color: color('elon_button_primary_text', '#000000'),
        background: color('elon_button_primary_bg', '#FFFFFF'),
        borderColor: color('elon_button_primary_bg', '#FFFFFF'),
        source: colorSource('elon_button_primary_bg'),
      }),
      element('apk.statusTokens', '状态色 token', 'card', {
        x: 210,
        y: toolbarHeight + 170,
        width: 160,
        height: 86,
        text: `成功 ${color('elon_accent_primary', '#58BE6A')}\n项目 ${color('elon_status_project', '#F2C94C')}\n危险 ${color('elon_status_danger', '#E62129')}`,
        fontSize: 13,
        lineHeight: 19,
        fontWeight: 700,
        paddingX: 14,
        paddingY: 12,
        borderRadius: 18,
        color: color('elon_text_primary', '#D9D9D9'),
        background: color('elon_surface_card', '#1A1A1A'),
        borderColor: color('elon_divider_card', '#6D6E6F'),
        source: colorSource('elon_accent_primary', '@color/elon_accent_primary / status'),
      }),
      element('apk.inputLayout', '聊天输入栏', 'card', {
        x: 0,
        y: bottomPanelY - inputHeight - 8,
        width: canvasWidth,
        height: inputHeight,
        text: `#inputLayout\npadding 10/8dp · 输入 ${readNodeById('inputEdit')?.attrs.textSize ?? '15sp'}`,
        fontSize: 13,
        lineHeight: 19,
        fontWeight: 700,
        paddingX: 14,
        paddingY: 10,
        borderRadius: 0,
        borderWidth: 0,
        color: color('elon_text_primary', '#D9D9D9'),
        background: color('elon_nav_bg', '#1A1A1A'),
        borderColor: color('elon_nav_bg', '#1A1A1A'),
        source: nodeSource(inputLayout, 'activity_main.xml #inputLayout'),
      }),
      element('apk.pageTabs', '底部导航外层', 'card', {
        x: 0,
        y: bottomPanelY,
        width: canvasWidth,
        height: bottomOuterHeight,
        text: `#pageTabs\n@dimen/main_bottom_menu_outer_height = ${bottomOuterHeight}dp`,
        fontSize: 12,
        lineHeight: 18,
        fontWeight: 700,
        paddingX: 18,
        paddingY: 8,
        borderRadius: 0,
        borderWidth: 0,
        color: color('elon_text_secondary', '#B8B8B8'),
        background: color('elon_bg_app', '#000000'),
        borderColor: color('elon_bg_app', '#000000'),
        source: dimenSource('main_bottom_menu_outer_height', 'bottom menu outer height'),
      }),
      element('apk.bottomNavPanel', '底部导航面板', 'card', {
        x: 20,
        y: bottomPanelY + 8,
        width: canvasWidth - 40,
        height: mainTabHeight,
        text: `@color/elon_nav_bg ${color('elon_nav_bg', '#1A1A1A')}`,
        fontSize: 12,
        lineHeight: 18,
        fontWeight: 700,
        paddingX: 12,
        paddingY: 8,
        borderRadius: 24,
        borderWidth: 0,
        color: color('elon_text_nav', '#D6D6D6'),
        background: color('elon_nav_bg', '#1A1A1A'),
        borderColor: color('elon_nav_bg', '#1A1A1A'),
        source: colorSource('elon_nav_bg'),
      }),
      ...tabLabels.map((label, index) => element(`apk.bottomTab.${label}`, `底部 Tab ${label}`, 'text', {
        x: 20 + Math.round(tabWidth * index),
        y: bottomPanelY + 22,
        width: Math.round(tabWidth),
        height: 42,
        text: `◎\n${label}`,
        fontSize: mainTabTextSize,
        lineHeight: 19,
        fontWeight: 400,
        paddingX: 0,
        paddingY: 0,
        borderWidth: 0,
        color: color('elon_text_nav', '#D6D6D6'),
        background: color('elon_nav_bg', '#1A1A1A'),
        borderColor: color('elon_nav_bg', '#1A1A1A'),
        source: source('@style/MainTabText', THEMES_PATH, styleLine('MainTabText'), '@style/MainTabText', `textSize=${mainTabTextSize}sp`),
      })),
    ],
  }
}
