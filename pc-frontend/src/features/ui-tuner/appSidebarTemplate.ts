import type { UiTunerDocument, UiTunerElement, UiTunerElementKind, UiTunerSource } from './types'

const TEMPLATE_PREFIX = 'app-sidebar.'
const BASE_WIDTH = 390
const BASE_HEIGHT = 844

export const APP_SIDEBAR_TEMPLATE_SOURCE: UiTunerSource = {
  kind: 'manual',
  label: 'APP 侧边栏模板 / 自动铺层',
}

interface TemplateRect {
  x: number
  y: number
  width: number
  height: number
}

interface TemplateStyle {
  text?: string
  fontSize?: number
  lineHeight?: number
  fontWeight?: number
  paddingX?: number
  paddingY?: number
  borderRadius?: number
  borderWidth?: number
  color?: string
  background?: string
  borderColor?: string
  opacity?: number
}

export function isAppSidebarTemplateElement(element: UiTunerElement) {
  return element.id.startsWith(TEMPLATE_PREFIX)
}

export function createAppSidebarTemplateElements(canvas: UiTunerDocument['canvas']): UiTunerElement[] {
  const scaleX = canvas.width / BASE_WIDTH
  const scaleY = canvas.height / BASE_HEIGHT
  const scale = Math.max(0.75, Math.min(scaleX, scaleY))

  const x = (value: number) => Math.round(value * scaleX)
  const y = (value: number) => Math.round(value * scaleY)
  const size = (value: number) => Math.round(value * scale)
  const rect = (value: TemplateRect) => ({
    x: x(value.x),
    y: y(value.y),
    width: x(value.width),
    height: y(value.height),
  })

  const element = (
    id: string,
    name: string,
    kind: UiTunerElementKind,
    bounds: TemplateRect,
    style: TemplateStyle = {},
  ): UiTunerElement => ({
    id: `${TEMPLATE_PREFIX}${id}`,
    name,
    kind,
    ...rect(bounds),
    text: style.text ?? name,
    fontSize: style.fontSize ? size(style.fontSize) : size(14),
    lineHeight: style.lineHeight ? size(style.lineHeight) : size(20),
    fontWeight: style.fontWeight ?? 600,
    letterSpacing: 0,
    paddingX: style.paddingX === undefined ? size(12) : size(style.paddingX),
    paddingY: style.paddingY === undefined ? size(8) : size(style.paddingY),
    borderRadius: style.borderRadius === undefined ? size(8) : size(style.borderRadius),
    borderWidth: style.borderWidth ?? 1,
    color: style.color ?? '#D9D9D9',
    background: style.background ?? 'rgba(31, 32, 35, 0.52)',
    borderColor: style.borderColor ?? '#4D4D4D',
    opacity: style.opacity ?? 1,
    source: APP_SIDEBAR_TEMPLATE_SOURCE,
    standard: {
      scope: 'screen_override',
      role: `app-sidebar.${id}`,
      component: 'AppSidebar',
      variant: 'tunable-template',
      tokenRefs: {},
      reuseKey: `app-sidebar.${id}`,
      note: '由微调画布 APP 侧边栏模板自动铺层生成，可按截图继续拖动和改数值。',
    },
  })

  return [
    element('title', '页面标题', 'text', { x: 150, y: 80, width: 90, height: 38 }, {
      text: '项目',
      fontSize: 20,
      lineHeight: 28,
      fontWeight: 500,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      background: 'transparent',
    }),
    element('search', '搜索框', 'card', { x: 32, y: 140, width: 326, height: 56 }, {
      text: '搜索框',
      fontSize: 16,
      lineHeight: 24,
      fontWeight: 500,
      paddingX: 44,
      paddingY: 16,
      borderRadius: 28,
      borderWidth: 0,
      color: '#AFAFAF',
      background: '#272727',
      borderColor: '#272727',
    }),
    element('projectCenter', '项目中心入口', 'text', { x: 32, y: 250, width: 326, height: 44 }, {
      text: '项目中心    ›',
      fontSize: 22,
      lineHeight: 32,
      fontWeight: 500,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      background: 'transparent',
    }),
    element('recommendLabel', '推荐标题', 'text', { x: 32, y: 304, width: 120, height: 32 }, {
      text: '推荐  →',
      fontSize: 21,
      lineHeight: 30,
      fontWeight: 500,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      background: 'transparent',
    }),
    element('projectCover', '项目卡封面', 'media', { x: 32, y: 346, width: 88, height: 88 }, {
      text: '',
      borderRadius: 12,
      borderWidth: 0,
      background: '#FFFFFF',
      borderColor: '#FFFFFF',
    }),
    element('projectText', '项目卡文字区', 'text', { x: 144, y: 346, width: 180, height: 92 }, {
      text: '新项目 5\n创建者：60\n版本：1.0',
      fontSize: 18,
      lineHeight: 28,
      fontWeight: 500,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      color: '#D9D9D9',
      background: 'transparent',
    }),
    element('projectOpen', '项目打开箭头', 'text', { x: 306, y: 360, width: 52, height: 52 }, {
      text: '↗',
      fontSize: 36,
      lineHeight: 44,
      fontWeight: 400,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      background: 'transparent',
    }),
    element('intro', '应用介绍', 'text', { x: 32, y: 450, width: 326, height: 40 }, {
      text: '应用介绍：APK 创建的项目',
      fontSize: 20,
      lineHeight: 30,
      fontWeight: 500,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      background: 'transparent',
    }),
    element('screenshotOne', '截图位 1', 'media', { x: 32, y: 508, width: 132, height: 176 }, {
      text: '截图位',
      fontSize: 13,
      lineHeight: 20,
      fontWeight: 600,
      paddingX: 10,
      paddingY: 10,
      borderRadius: 10,
      borderWidth: 1,
      background: 'rgba(0, 0, 0, 0.16)',
      borderColor: '#4D4D4D',
      color: '#AFAFAF',
    }),
    element('screenshotTwo', '截图位 2', 'media', { x: 190, y: 508, width: 132, height: 176 }, {
      text: '截图位',
      fontSize: 13,
      lineHeight: 20,
      fontWeight: 600,
      paddingX: 10,
      paddingY: 10,
      borderRadius: 10,
      borderWidth: 1,
      background: 'rgba(0, 0, 0, 0.16)',
      borderColor: '#4D4D4D',
      color: '#AFAFAF',
    }),
    element('level', '等级文字', 'text', { x: 32, y: 706, width: 82, height: 26 }, {
      text: 'Lv.13',
      fontSize: 18,
      lineHeight: 26,
      fontWeight: 400,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      background: 'transparent',
    }),
    element('percent', '进度百分比', 'text', { x: 306, y: 706, width: 52, height: 26 }, {
      text: '97%',
      fontSize: 18,
      lineHeight: 26,
      fontWeight: 400,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      background: 'transparent',
    }),
    element('progressTrack', '进度条底', 'card', { x: 32, y: 740, width: 326, height: 10 }, {
      text: '',
      borderRadius: 999,
      borderWidth: 0,
      background: '#3A3B40',
      borderColor: '#3A3B40',
    }),
    element('progressGreen', '进度条绿色段', 'card', { x: 32, y: 740, width: 88, height: 10 }, {
      text: '',
      borderRadius: 0,
      borderWidth: 0,
      background: '#58BE6A',
      borderColor: '#58BE6A',
    }),
    element('progressMain', '进度条主进度', 'card', { x: 120, y: 740, width: 222, height: 10 }, {
      text: '',
      borderRadius: 0,
      borderWidth: 0,
      background: '#A8C7FA',
      borderColor: '#A8C7FA',
    }),
    element('userAvatar', '底部用户头像', 'media', { x: 32, y: 768, width: 64, height: 64 }, {
      text: '',
      borderRadius: 999,
      borderWidth: 0,
      background: '#FFFFFF',
      borderColor: '#FFFFFF',
    }),
    element('userName', '底部用户名', 'text', { x: 120, y: 776, width: 160, height: 30 }, {
      text: '夜云',
      fontSize: 22,
      lineHeight: 30,
      fontWeight: 500,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      background: 'transparent',
    }),
    element('userStatus', '底部在线状态', 'text', { x: 120, y: 810, width: 88, height: 24 }, {
      text: '在线',
      fontSize: 16,
      lineHeight: 24,
      fontWeight: 400,
      paddingX: 0,
      paddingY: 0,
      borderWidth: 0,
      color: '#777777',
      background: 'transparent',
    }),
  ]
}
