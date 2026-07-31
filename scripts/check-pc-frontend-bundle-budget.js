const fs = require('fs')
const path = require('path')
const zlib = require('zlib')

const KiB = 1024
const repoRoot = path.resolve(__dirname, '..')
const defaultDistDir = path.join(repoRoot, 'pc-frontend', 'dist')

function parseArgs(argv) {
  const args = { distDir: process.env.PC_FRONTEND_DIST_DIR || defaultDistDir }
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '--dist') {
      args.distDir = path.resolve(argv[i + 1] || '')
      i += 1
    } else if (arg === '--help' || arg === '-h') {
      args.help = true
    } else {
      throw new Error(`Unknown argument: ${arg}`)
    }
  }
  return args
}

function usage() {
  console.log('Usage: node scripts/check-pc-frontend-bundle-budget.js [--dist pc-frontend/dist]')
  console.log('Run npm run build first; this guard inspects dist/assets JS and CSS chunks.')
}

function gzipSize(buffer) {
  return zlib.gzipSync(buffer).length
}

function formatKiB(bytes) {
  return `${(bytes / KiB).toFixed(2)} KiB`
}

function collectAssets(distDir) {
  const assetsDir = path.join(distDir, 'assets')
  if (!fs.existsSync(assetsDir)) {
    throw new Error(`Cannot find ${assetsDir}. Run npm run build first.`)
  }
  return fs.readdirSync(assetsDir)
    .filter((name) => name.endsWith('.js') || name.endsWith('.css'))
    .map((name) => {
      const filePath = path.join(assetsDir, name)
      const buffer = fs.readFileSync(filePath)
      return {
        name,
        ext: path.extname(name),
        bytes: buffer.length,
        gzipBytes: gzipSize(buffer),
      }
    })
}

function sum(files, field) {
  return files.reduce((total, file) => total + file[field], 0)
}

function matchOne(files, pattern, label) {
  const matches = files.filter((file) => pattern.test(file.name))
  if (matches.length === 0) {
    return { label, missing: true }
  }
  if (matches.length > 1) {
    return { label, multiple: matches.map((file) => file.name) }
  }
  return { label, file: matches[0] }
}

function budgetResult(label, actualBytes, actualGzipBytes, spec, detail) {
  const failures = []
  const warnings = []
  if (spec.maxBytes != null && actualBytes > spec.maxBytes) {
    failures.push(`raw ${formatKiB(actualBytes)} > hard ${formatKiB(spec.maxBytes)}`)
  } else if (spec.warnBytes != null && actualBytes > spec.warnBytes) {
    warnings.push(`raw ${formatKiB(actualBytes)} > soft ${formatKiB(spec.warnBytes)}`)
  }
  if (spec.maxGzipBytes != null && actualGzipBytes > spec.maxGzipBytes) {
    failures.push(`gzip ${formatKiB(actualGzipBytes)} > hard ${formatKiB(spec.maxGzipBytes)}`)
  } else if (spec.warnGzipBytes != null && actualGzipBytes > spec.warnGzipBytes) {
    warnings.push(`gzip ${formatKiB(actualGzipBytes)} > soft ${formatKiB(spec.warnGzipBytes)}`)
  }
  return {
    label,
    detail,
    actualBytes,
    actualGzipBytes,
    warnBytes: spec.warnBytes,
    warnGzipBytes: spec.warnGzipBytes,
    maxBytes: spec.maxBytes,
    maxGzipBytes: spec.maxGzipBytes,
    warnings,
    failures,
  }
}

function checkNamedBudget(files, spec) {
  const match = matchOne(files, spec.pattern, spec.label)
  if (match.missing) {
    return {
      label: spec.label,
      warnings: [],
      failures: [`required chunk is missing: ${spec.pattern}`],
    }
  }
  if (match.multiple) {
    return {
      label: spec.label,
      warnings: [],
      failures: [`expected one chunk, found ${match.multiple.length}: ${match.multiple.join(', ')}`],
    }
  }
  return budgetResult(
    spec.label,
    match.file.bytes,
    match.file.gzipBytes,
    spec,
    match.file.name,
  )
}

function checkGroupBudget(files, spec) {
  const matches = files.filter((file) => spec.pattern.test(file.name))
  if (matches.length === 0) {
    return {
      label: spec.label,
      warnings: [],
      failures: [`required chunks are missing: ${spec.pattern}`],
    }
  }
  return budgetResult(
    spec.label,
    sum(matches, 'bytes'),
    sum(matches, 'gzipBytes'),
    spec,
    matches.map((file) => file.name).join(', '),
  )
}

function checkTotalBudget(files, spec) {
  const scoped = files.filter((file) => file.ext === spec.ext)
  return budgetResult(
    spec.label,
    sum(scoped, 'bytes'),
    sum(scoped, 'gzipBytes'),
    spec,
    `${scoped.length} files`,
  )
}

function checkMaxSingleBudget(files, spec) {
  const scoped = files
    .filter((file) => file.ext === spec.ext)
    .filter((file) => !spec.exclude.some((pattern) => pattern.test(file.name)))
    .sort((a, b) => b.bytes - a.bytes)
  const largest = scoped[0]
  if (!largest) {
    return { label: spec.label, warnings: [], failures: [`no ${spec.ext} chunks found`] }
  }
  return budgetResult(
    spec.label,
    largest.bytes,
    largest.gzipBytes,
    spec,
    largest.name,
  )
}

const namedBudgets = [
  {
    label: 'entry app js',
    pattern: /^app-[A-Za-z0-9_-]+\.js$/,
    maxBytes: 120 * KiB,
    maxGzipBytes: 45 * KiB,
  },
  {
    label: 'vendor js',
    pattern: /^vendor-[A-Za-z0-9_-]+\.js$/,
    maxBytes: 220 * KiB,
    maxGzipBytes: 75 * KiB,
  },
  {
    label: 'store js',
    pattern: /^store-[A-Za-z0-9_-]+\.js$/,
    maxBytes: 24 * KiB,
    maxGzipBytes: 10 * KiB,
  },
  {
    label: 'conversation page js',
    pattern: /^ConversationPage-[A-Za-z0-9_-]+\.js$/,
    maxBytes: 420 * KiB,
    maxGzipBytes: 140 * KiB,
  },
]

const groupedBudgets = [
  {
    label: 'conversation page css',
    pattern: /^ConversationPage-[A-Za-z0-9_-]+\.css$/,
    maxBytes: 190 * KiB,
    maxGzipBytes: 35 * KiB,
  },
]

const totalBudgets = [
  {
    label: 'total js',
    ext: '.js',
    warnBytes: 2200 * KiB,
    warnGzipBytes: 700 * KiB,
  },
  {
    label: 'total css',
    ext: '.css',
    warnBytes: 600 * KiB,
    warnGzipBytes: 120 * KiB,
  },
]

const maxSingleBudgets = [
  {
    label: 'largest async js',
    ext: '.js',
    exclude: [/^app-/, /^vendor-/, /^store-/],
    warnBytes: 480 * KiB,
    maxBytes: 520 * KiB,
    maxGzipBytes: 140 * KiB,
  },
  {
    label: 'largest css',
    ext: '.css',
    exclude: [],
    maxBytes: 190 * KiB,
    maxGzipBytes: 35 * KiB,
  },
]

function printResult(result) {
  const detail = result.detail ? ` ${result.detail}` : ''
  const actual = result.actualBytes == null
    ? ''
    : ` ${formatKiB(result.actualBytes)} gzip ${formatKiB(result.actualGzipBytes)}`
  const budget = result.maxBytes == null
    ? ''
    : ` hard ${formatKiB(result.maxBytes)} gzip ${formatKiB(result.maxGzipBytes)}`
  const soft = result.warnBytes == null && result.warnGzipBytes == null
    ? ''
    : ` soft${result.warnBytes == null ? '' : ` ${formatKiB(result.warnBytes)}`}`
      + `${result.warnGzipBytes == null ? '' : ` gzip ${formatKiB(result.warnGzipBytes)}`}`
  const status = result.failures.length > 0
    ? 'failed'
    : result.warnings.length > 0 ? 'warning' : 'passed'
  console.log(`PC_BUNDLE_BUDGET_CHECK=${status} ${result.label}${detail}${actual}${soft}${budget}`)
  for (const warning of result.warnings) {
    console.log(`  warning: ${warning}`)
  }
  for (const failure of result.failures) {
    console.log(`  failure: ${failure}`)
  }
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  if (args.help) {
    usage()
    return
  }

  const files = collectAssets(args.distDir)
  const results = [
    ...namedBudgets.map((spec) => checkNamedBudget(files, spec)),
    ...groupedBudgets.map((spec) => checkGroupBudget(files, spec)),
    ...maxSingleBudgets.map((spec) => checkMaxSingleBudget(files, spec)),
    ...totalBudgets.map((spec) => checkTotalBudget(files, spec)),
  ]
  const failures = results.flatMap((result) => result.failures)
  const warnings = results.flatMap((result) => result.warnings)

  for (const result of results) {
    printResult(result)
  }

  if (failures.length > 0) {
    console.error(`PC_BUNDLE_BUDGET=failed failures=${failures.length} warnings=${warnings.length}`)
    process.exit(1)
  }
  console.log(`PC_BUNDLE_BUDGET=passed assets=${files.length} warnings=${warnings.length}`)
}

try {
  main()
} catch (error) {
  console.error(`PC_BUNDLE_BUDGET=failed ${error instanceof Error ? error.message : String(error)}`)
  process.exit(1)
}
