import '../../../server/src/assets/group_roster.js'
import '../../../server/src/assets/group_roster.css'
import { offlineRoster } from './group-roster-offline.js'
const preview = new URLSearchParams(location.search).has('offline')
const controller = window.ElonGroupRoster.install({ api: preview ? offlineRoster : fetch.bind(window), getGroup: () => ({ id: 'fixture-group', name: '杀蟑螂' }), getUserId: () => 'member-0000',
  onChanged() {}, onExit() { document.querySelector('#notice').textContent = '已退出群聊' },
  onMention(member) { document.querySelector('#notice').textContent = `已插入 @${member.display_name}` }, onMessage() {}, onNotice(text) { document.querySelector('#notice').textContent = text } })
document.querySelector('#open').onclick = () => controller.open()
if (preview) controller.open(new URLSearchParams(location.search).get('state') !== 'preview')
