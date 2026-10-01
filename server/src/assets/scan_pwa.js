import { openScanDialog } from '/assets/browserScanDialog.mjs';

let active;
function open({ imageUrl } = {}) {
  active?.close();
  active = openScanDialog({
    imageUrl, workerFactory: () => new Worker('/assets/social_source_worker.js'),
    onClose: () => { active = null; },
    onFriend: id => {
      document.getElementById('homeActionAddFriendBtn')?.click();
      const input = document.getElementById('friendSearchInput');
      if (input) { input.value = id; input.dispatchEvent(new Event('input', { bubbles: true })); input.focus(); }
    },
  });
}
globalThis.ElonScanner = { open };
const scanEntry = document.querySelector('.add-friend-scan-pill');
scanEntry?.addEventListener('click', () => open());
const menu = document.getElementById('homeActionMenu');
if (menu) {
  const button = document.createElement('button'); button.type = 'button'; button.textContent = '扫一扫'; button.id = 'homeActionScanBtn';
  button.onclick = () => { menu.classList.add('hidden'); menu.setAttribute('aria-hidden', 'true'); document.getElementById('bottomComposeBtn')?.setAttribute('aria-expanded', 'false'); open(); };
  menu.prepend(button);
}
