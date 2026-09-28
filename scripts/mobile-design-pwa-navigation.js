// This script is served only by the local acceptance fixture, never production.
// It follows existing UI controls; all content still comes from production renderers.
(() => {
  const scenario = new URLSearchParams(location.search).get('fixture');
  if (!['projects', 'empty', 'chat_result', 'account_security'].includes(scenario)) return;
  const deadline = Date.now() + 10000;
  let openedBrowser = false;
  const wait = () => {
    const app = document.getElementById('appView');
    const selector = scenario === 'account_security' ? '#accountIdentitiesRow'
      : openedBrowser ? '.project-browser-item[data-project-browser-index="0"]' : '#bottomMenuBtn';
    const control = document.querySelector(selector);
    if (app && !app.classList.contains('hidden') && control && control.getClientRects().length) {
      control.click();
      if (scenario !== 'chat_result' || openedBrowser) return;
      openedBrowser = true;
    }
    if (Date.now() < deadline) setTimeout(wait, 50);
    else console.error('Offline fixture navigation did not become ready: ' + scenario);
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', wait, { once: true });
  else wait();
})();
