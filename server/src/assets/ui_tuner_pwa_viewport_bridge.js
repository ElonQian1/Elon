(() => {
  if (window.__ELON_UI_TUNER_VIEWPORT_STATE__) return;

  window.__ELON_UI_TUNER_VIEWPORT_STATE__ = () => {
    const visual = window.visualViewport;
    let pointer = 'none';
    if (typeof window.matchMedia === 'function') {
      pointer = window.matchMedia('(pointer: coarse)').matches
        ? 'coarse'
        : window.matchMedia('(pointer: fine)').matches ? 'fine' : 'none';
    }
    return {
      width: window.innerWidth,
      height: window.innerHeight,
      deviceScaleFactor: window.devicePixelRatio || 1,
      visualWidth: visual ? visual.width : window.innerWidth,
      visualHeight: visual ? visual.height : window.innerHeight,
      pointer,
    };
  };
})();
