const assert = require('node:assert/strict')
const fs = require('node:fs')
const vm = require('node:vm')
const path = require('node:path')
const ts = require('../pc-frontend/node_modules/typescript')
const output = ts.transpileModule(fs.readFileSync(path.join(__dirname, '../pc-frontend/src/features/friends/socialChatOperations.ts'), 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText
const exported = {}
let fetchImpl
vm.runInNewContext(output, { exports: exported, require: name => name.endsWith('/client') ? { getAuthToken: () => 'isolated-test-token' } : { resolveApiUrl: p => `https://fixture.invalid${p}` },
  fetch: (...args) => fetchImpl(...args), AbortController, setTimeout, clearTimeout, Date, Error })
const { canRecall, quoteText, messageEndpoint, socialRequest, sendSocialMessage, SocialRequestError } = exported
async function main() {
  const original = { id:'m', content:'原文 🐲\n第二行', sender_user_id:'u', created_at:'2026-09-15T08:00:00Z', outgoing:true, revision:3 }
  const now = Date.parse('2026-09-15T08:00:59Z')
  assert.equal(canRecall(original, true, now), true)
  for (const [m, own, at] of [[original,false,now],[original,true,now+2000],[{...original,recalled_at:'now'},true,now],[{...original,id:'tmp-a'},true,now]]) assert.equal(canRecall(m,own,at),false)
  assert.equal(messageEndpoint({kind:'group',id:'g /?'}),'/api/me/groups/g%20%2F%3F/messages')
  assert.equal(quoteText(original,'作者\n伪造行'),'> 引用 作者 伪造行 · 第 3 版\n> 原文 🐲\n> 第二行')
  const quoted = quoteText({...original,content:'🐲'.repeat(800)},'u')
  assert.equal(Array.from(quoted.split('\n')[1].slice(2)).length,400)
  let calls = []
  fetchImpl = async (url, init) => { calls.push({url,init}); return {ok:true,status:200,json:async()=>({message:{id:'new'}})} }
  await sendSocialMessage({kind:'friend',id:'f'}, {content:'保留',attachments:[{attachment_id:'a',url:'/a.wav',kind:'audio'}]})
  assert.equal(calls.length,1); assert.equal(calls[0].init.headers.Authorization,'Bearer isolated-test-token')
  assert.equal(JSON.parse(calls[0].init.body).attachments[0].kind,'audio')
  fetchImpl = async () => ({ok:false,status:403,json:async()=>({error:'无权撤回'})})
  await assert.rejects(socialRequest('/m',{method:'DELETE'}), e=>e instanceof SocialRequestError && !e.uncertain && e.status===403)
  let attempts=0
  fetchImpl=(_url,init)=>{attempts++;return new Promise((_,reject)=>init.signal.addEventListener('abort',()=>reject(new Error('aborted'))))}
  await assert.rejects(socialRequest('/m',{method:'POST'},5), e=>e.uncertain)
  assert.equal(attempts,1,'uncertain writes must not retry automatically')
  await assert.rejects(socialRequest('/m',{},5), e=>!e.uncertain)
  console.log('PASS social operations: ownership/recall window, Unicode quote snapshots, route encoding, authenticated media payload, permission rejection and no automatic uncertain-write retry')
}
main().catch(error=>{console.error(error);process.exitCode=1})
