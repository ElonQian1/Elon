export type TerminalColor =
  | 'black'
  | 'red'
  | 'green'
  | 'yellow'
  | 'blue'
  | 'magenta'
  | 'cyan'
  | 'white'
  | 'brightBlack'
  | 'brightRed'
  | 'brightGreen'
  | 'brightYellow'
  | 'brightBlue'
  | 'brightMagenta'
  | 'brightCyan'
  | 'brightWhite'

export interface TerminalSegment {
  text: string
  fg?: TerminalColor
  bg?: TerminalColor
  bold?: boolean
  dim?: boolean
  underline?: boolean
  inverse?: boolean
}

interface TerminalStyle {
  fg?: TerminalColor
  bg?: TerminalColor
  bold: boolean
  dim: boolean
  underline: boolean
  inverse: boolean
}

const ANSI_SEQUENCE = /\x1b\][\s\S]*?(?:\x07|\x1b\\)|\x1b\[[0-?]*[ -/]*[@-~]|\x1b[@-Z\\-_]/g
const CONTROL_CHARS = /[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g

const LOW_COLORS: TerminalColor[] = [
  'black',
  'red',
  'green',
  'yellow',
  'blue',
  'magenta',
  'cyan',
  'white',
]

const BRIGHT_COLORS: TerminalColor[] = [
  'brightBlack',
  'brightRed',
  'brightGreen',
  'brightYellow',
  'brightBlue',
  'brightMagenta',
  'brightCyan',
  'brightWhite',
]

export function normalizeTerminalChunk(value: string): string {
  return value
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    .replace(/\x08/g, '')
}

export function parseTerminalSegments(value: string): TerminalSegment[] {
  const segments: TerminalSegment[] = []
  let style = defaultStyle()
  let cursor = 0
  ANSI_SEQUENCE.lastIndex = 0

  for (const match of value.matchAll(ANSI_SEQUENCE)) {
    const index = match.index ?? 0
    pushPlain(segments, value.slice(cursor, index), style)
    applyAnsi(match[0], style)
    cursor = index + match[0].length
  }
  pushPlain(segments, value.slice(cursor), style)
  return segments
}

function pushPlain(segments: TerminalSegment[], raw: string, style: TerminalStyle) {
  const text = normalizeTerminalChunk(raw).replace(CONTROL_CHARS, '')
  if (!text) return
  const next: TerminalSegment = {
    text,
    fg: style.fg,
    bg: style.bg,
    bold: style.bold || undefined,
    dim: style.dim || undefined,
    underline: style.underline || undefined,
    inverse: style.inverse || undefined,
  }
  const prev = segments[segments.length - 1]
  if (prev && sameStyle(prev, next)) {
    prev.text += next.text
  } else {
    segments.push(next)
  }
}

function applyAnsi(sequence: string, style: TerminalStyle) {
  if (!sequence.startsWith('\x1b[') || !sequence.endsWith('m')) return
  const rawCodes = sequence.slice(2, -1).trim()
  const codes = rawCodes
    ? rawCodes.split(';').map((part) => Number.parseInt(part || '0', 10))
    : [0]

  for (let i = 0; i < codes.length; i += 1) {
    const code = Number.isFinite(codes[i]) ? codes[i] : 0
    if (code === 0) {
      style.fg = undefined
      style.bg = undefined
      style.bold = false
      style.dim = false
      style.underline = false
      style.inverse = false
    } else if (code === 1) {
      style.bold = true
      style.dim = false
    } else if (code === 2) {
      style.dim = true
      style.bold = false
    } else if (code === 22) {
      style.bold = false
      style.dim = false
    } else if (code === 4) {
      style.underline = true
    } else if (code === 24) {
      style.underline = false
    } else if (code === 7) {
      style.inverse = true
    } else if (code === 27) {
      style.inverse = false
    } else if (code === 39) {
      style.fg = undefined
    } else if (code === 49) {
      style.bg = undefined
    } else if (code >= 30 && code <= 37) {
      style.fg = LOW_COLORS[code - 30]
    } else if (code >= 90 && code <= 97) {
      style.fg = BRIGHT_COLORS[code - 90]
    } else if (code >= 40 && code <= 47) {
      style.bg = LOW_COLORS[code - 40]
    } else if (code >= 100 && code <= 107) {
      style.bg = BRIGHT_COLORS[code - 100]
    } else if ((code === 38 || code === 48) && codes[i + 1] === 5) {
      const mapped = colorFromAnsi256(codes[i + 2])
      if (mapped) {
        if (code === 38) style.fg = mapped
        else style.bg = mapped
      }
      i += 2
    }
  }
}

function colorFromAnsi256(code: number | undefined): TerminalColor | undefined {
  if (code === undefined || !Number.isFinite(code)) return undefined
  if (code >= 0 && code <= 7) return LOW_COLORS[code]
  if (code >= 8 && code <= 15) return BRIGHT_COLORS[code - 8]
  return undefined
}

function defaultStyle(): TerminalStyle {
  return {
    bold: false,
    dim: false,
    underline: false,
    inverse: false,
  }
}

function sameStyle(left: TerminalSegment, right: TerminalSegment): boolean {
  return left.fg === right.fg
    && left.bg === right.bg
    && left.bold === right.bold
    && left.dim === right.dim
    && left.underline === right.underline
    && left.inverse === right.inverse
}
