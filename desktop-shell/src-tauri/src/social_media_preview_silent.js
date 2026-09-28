// Only the temporary cover reader uses this; interactive reading tabs remain unchanged.
(() => {
  function stop(media) {
    if (!(media instanceof HTMLMediaElement)) return;
    media.muted = true;
    media.autoplay = false;
    media.pause();
  }
  HTMLMediaElement.prototype.play = function () { stop(this); return Promise.resolve(); };
  // Autoplay attributes can start playback without calling the JavaScript play method.
  for (const event of ['loadstart', 'play', 'playing']) {
    document.addEventListener(event, e => stop(e.target), true);
  }
})();
