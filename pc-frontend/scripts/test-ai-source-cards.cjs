const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const root = path.resolve(__dirname, '..')
const component = fs.readFileSync(path.join(root, 'src/features/ai/AiSourceLinks.tsx'), 'utf8')
const styles = fs.readFileSync(path.join(root, 'src/features/ai/AiSourceLinks.module.css'), 'utf8')
const sourceMark = fs.readFileSync(path.join(root, 'src/features/ai/AiSourceMark.tsx'), 'utf8')
const sourceMarkStyles = fs.readFileSync(path.join(root, 'src/features/ai/AiSourceMark.module.css'), 'utf8')
const presentation = fs.readFileSync(path.join(root, 'src/features/ai/aiSourcePresentation.ts'), 'utf8')
const answerStyles = fs.readFileSync(path.join(root, 'src/features/ai/AiChatPage.module.css'), 'utf8')
const messageRow = fs.readFileSync(path.join(root, 'src/features/ai/AiChatMessageRow.tsx'), 'utf8')
const markdown = fs.readFileSync(path.join(root, 'src/features/markdown/MarkdownContent.tsx'), 'utf8')
const markdownStyles = fs.readFileSync(path.join(root, 'src/features/markdown/MarkdownContent.module.css'), 'utf8')
const backend = fs.readFileSync(path.join(root, 'src/features/user-browser/useAiWebChatBackend.ts'), 'utf8')
const protocol = fs.readFileSync(path.join(root, 'src/features/user-browser/localAiBrowserProtocol.ts'), 'utf8')
const adapter = fs.readFileSync(path.join(root, '../android/app/src/main/assets/chatgpt_web_adapter_messages.js'), 'utf8')
const sanitizer = fs.readFileSync(path.join(root, '../desktop-shell/src-tauri/src/local_ai_browser/adapter_content.rs'), 'utf8')

assert.match(component, /MAX_VISIBLE_SOURCES\s*=\s*3/, 'source cards must start with a bounded summary')
assert.match(component, /uniqueSources/, 'duplicate source links must not create duplicate cards')
assert.match(component, /aiSiteIdentity/, 'source cards must expose a stable site identity without waiting for remote CSS')
assert.match(component, /AiSourceMark/, 'source cards must use the shared official-page source logo when available')
assert.match(sourceMark, /referrerPolicy="no-referrer"/, 'source logos must not send the conversation page as a referrer')
assert.match(sourceMark, /aiSourceIconCandidates/, 'source logos need a bounded fallback chain')
assert.match(presentation, /googleFaviconUrl\(sourceHost\)/, 'an incomplete official favicon URL must be rebuilt from the public source host')
assert.match(presentation, /searchParams\.set\('domain', host\)/, 'favicon lookup must disclose only the hostname, never the article path')
assert.match(presentation, /`\$\{origin\}\/favicon\.ico`/, 'source-origin favicon must remain available when the official cache fails')
assert.ok(
  presentation.indexOf('candidates.push(`${origin}/favicon.ico`)') < presentation.indexOf('candidates.push(googleFaviconUrl(sourceHost))'),
  'the publisher origin icon must be attempted before the optional public resolver',
)
assert.match(sourceMark, /setFailedUrls/, 'broken logos must advance through the fallback chain')
assert.match(sourceMark, /ICON_LOAD_TIMEOUT_MS\s*=\s*2_500/, 'a hanging logo request needs a bounded timeout')
assert.match(sourceMark, /loadedUrl === iconUrl/, 'an unconfirmed remote image must not cover the stable monogram')
assert.match(sourceMarkStyles, /\.logoReady\s*\{[\s\S]*opacity:\s*1/, 'a source logo becomes visible only after load succeeds')
assert.match(presentation, /safeIconUrl/, 'source logos must be validated again at the presentation boundary')
assert.match(component, /aiSourceDisplayTitle/, 'raw citation URLs must be reduced to a readable source title')
assert.match(component, /source\.snippet/, 'structured private search summaries must reach the native source card')
assert.match(component, /source\.thumbnail_url/, 'official public source thumbnails must remain available to the native source card')
assert.match(component, /referrerPolicy="no-referrer"/, 'source thumbnails must not disclose the conversation page as a referrer')
assert.match(presentation, /safeAiSourceThumbnail/, 'source thumbnails need the same public HTTPS presentation guard as logos')
assert.match(component, /aria-expanded=\{expanded\}/, 'the default source presentation must be a compact expandable entry')
assert.match(component, /logoStack/, 'the compact source entry must preview available logos')
assert.match(component, /全部显示/, 'long source collections need an explicit expand action')
assert.match(component, /href=\{source\.url\}[\s\S]*target="_blank"[\s\S]*aria-label=\{`使用系统浏览器打开/, 'the primary card action must use the reliable system-browser path')
assert.match(component, /className=\{styles\.internal\}[\s\S]*openInternalBrowserLink\(source\)/, 'the optional internal tab flow must remain available as a secondary action')
assert.match(component, /aria-label=\{`回答来源，共 \$\{uniqueSources\.length\} 个`\}/, 'source count must be announced accessibly')
assert.match(styles, /\.panel\s*\{/, 'source collection needs a distinct panel')
assert.match(styles, /\.card\s*\{/, 'each source needs a card surface')
assert.match(sourceMarkStyles, /\.logo\s*\{/, 'official source logos need an explicit bounded presentation')
assert.match(styles, /\.summary\s*\{/, 'sources need a compact ChatGPT-style summary entry')
assert.match(styles, /\.logoStack\s*\{/, 'the compact summary needs a recognizable logo stack')
assert.match(markdownStyles, /\.citationLink\s*\{/, 'answer citations need a compact official-style inline surface')
assert.match(markdownStyles, /vertical-align:\s*middle/, 'inline citations must stay aligned to the answer baseline')
assert.match(sourceMarkStyles, /\.inline\s*\{/, 'inline citations need a dedicated favicon size')
assert.match(answerStyles, /\.msgContent\s*\{[\s\S]*font-size:\s*15px/, 'native answer typography must keep the readable official-page density')
assert.match(styles, /@media \(max-width: 560px\)/, 'source cards need a narrow-layout contract')
assert.doesNotMatch(component, /stylesheet|dangerouslySetInnerHTML/i, 'native cards must not import or execute official-page presentation code')
assert.match(protocol, /iconUrl\?: string/, 'the local AI protocol must carry a sanitized source logo URL')
assert.match(protocol, /markerText\?: string/, 'the local AI protocol must preserve an exact structurally linked citation marker')
assert.match(protocol, /citationId\?: string/, 'the local AI protocol must preserve a bounded local citation identity')
assert.match(protocol, /groupSize\?: number/, 'the local AI protocol must preserve the official citation group size')
assert.match(protocol, /snippet\?: string/, 'the local AI protocol must preserve a bounded public result summary')
assert.match(protocol, /thumbnailUrl\?: string/, 'the local AI protocol must preserve a sanitized public result thumbnail')
assert.match(backend, /icon_url: part\.iconUrl/, 'citation logo metadata must reach the source-card model')
assert.match(backend, /marker_text: part\.markerText/, 'structured citation markers must reach the source-card model')
assert.match(backend, /group_size: part\.groupSize/, 'structured citation group counts must reach the inline label')
assert.match(backend, /snippet: part\.snippet/, 'public result summaries must reach the source-card model')
assert.match(backend, /thumbnail_url: part\.thumbnailUrl/, 'public result thumbnails must reach the source-card model')
assert.match(markdown, /findCitation\(citationIndex, safe\)/, 'inline citation matching must use the structured citation index instead of visible label text')
assert.match(markdown, /byHost/, 'inline citation matching needs an unambiguous host fallback for vendor redirect links')
assert.match(markdown, /aiInlineCitationLabel\(citation, inlineText\(children\)\)/, 'inline links must render a stable provider label and source count')
assert.match(markdown, /<AiSourceMark source=\{citation\} variant="inline" \/>/, 'inline citation pills must reuse the bounded source-logo fallback')
assert.match(messageRow, /citations=\{message\.sources\}/, 'AI messages must join markdown links to their semantic citation metadata')
assert.match(messageRow, /hasVisibleAiMessageContent/, 'citation work must preserve the upstream empty-answer pending state')
assert.match(adapter, /url: safeMarkdownHref\(node\)/, 'citation extraction must preserve the public article URL')
assert.match(adapter, /metadata\.iconUrl = iconUrl/, 'citation extraction must associate the nested logo with its source')
assert.match(adapter, /if \(node\.closest\('a\[href\]'\)\) return '';/, 'citation logos must not leak into answer markdown as image placeholders')
assert.match(adapter, /if \(node\.closest\('a\[href\]'\)\) return;[\s\S]*add\('image'/, 'citation logos must not be emitted as image attachments')
assert.match(sanitizer, /part_type == "citation"[\s\S]*"iconUrl"/, 'the Rust boundary must keep only citation logo URLs')
assert.match(sanitizer, /"markerText"/, 'the Rust boundary must preserve only bounded citation marker text')
assert.match(sanitizer, /"citationId"/, 'the Rust boundary must preserve only bounded citation identities')
assert.match(sanitizer, /"groupSize"/, 'the Rust boundary must preserve only bounded citation group counts')
assert.match(sanitizer, /part\.get\("snippet"\)/, 'the Rust boundary must preserve only bounded public result summaries')
assert.match(sanitizer, /part\.get\("thumbnailUrl"\)/, 'the Rust boundary must sanitize public result thumbnails')

const compiledPresentation = ts.transpileModule(presentation, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
const presentationModule = new Module('aiSourcePresentation.fixture.cjs')
presentationModule.filename = path.join(root, 'src/features/ai/aiSourcePresentation.fixture.cjs')
presentationModule.paths = module.paths
presentationModule._compile(compiledPresentation, presentationModule.filename)
const { aiInlineCitationLabel, aiSiteIdentity, aiSourceIconCandidates, normalizedAiSourceUrl } = presentationModule.exports
const liveReutersMarkdownUrl = 'https://www.reuters.com/business/us-stock-futures-rise-after-sharp-losses-prior-session-2026-08-21/'
const liveReutersCitationUrl = 'https://www.reuters.com/business/us-stock-futures-rise-after-sharp-losses-prior-session-2026-08-21/?utm_source=chatgpt.com'
assert.equal(
  normalizedAiSourceUrl(liveReutersMarkdownUrl),
  normalizedAiSourceUrl(liveReutersCitationUrl),
  'the real ChatGPT Reuters citation shape must join after public query sanitization',
)
const googleReutersRedirect = `https://www.google.com/url?sa=t&url=${encodeURIComponent(liveReutersCitationUrl)}`
assert.equal(
  normalizedAiSourceUrl(googleReutersRedirect),
  normalizedAiSourceUrl(liveReutersMarkdownUrl),
  'known Google redirect links must join the structured public citation before tracking data is dropped',
)
assert.equal(
  aiSiteIdentity(googleReutersRedirect).host,
  'reuters.com',
  'source identity and favicon fallback must use the public publisher rather than the redirect host',
)
const incompleteReutersCandidates = aiSourceIconCandidates({
    title: 'Reuters+1',
    url: liveReutersMarkdownUrl,
    icon_url: 'https://www.google.com/s2/favicons',
  })
assert.equal(
  incompleteReutersCandidates[0],
  'https://www.reuters.com/favicon.ico',
  'the publisher origin must stay usable when the remote favicon resolver hangs',
)
assert.match(
  incompleteReutersCandidates[1],
  /google\.com\/s2\/favicons\?domain=reuters\.com&sz=64/,
  'an incomplete ChatGPT favicon remains a last-resort lookup rebuilt from the public source host',
)
assert.equal(
  aiInlineCitationLabel({
    title: 'Reuters',
    url: liveReutersMarkdownUrl,
    marker_text: 'Reuters',
    group_size: 3,
  }),
  'Reuters +2',
  'the structured group size must render even when the visible link marker omits the suffix',
)

console.log('PASS: AI source links render as responsive native cards while preserving both browser-opening paths')
