(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptRealtimeVoicePolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const END_VOICE = /(?:end|exit|leave|stop|close|hang\s*up).*(?:voice|call)|(?:voice|call).*(?:end|exit|leave|stop|close|hang\s*up)|挂断|结束.*(?:语音|通话)|退出.*语音|离开.*语音|关闭.*语音/i;
  const UNMUTE_VOICE = /unmute|turn.on.*(?:mic|microphone)|取消静音|打开麦克风|开启麦克风/i;
  const MUTE_VOICE = /(?:^|\b)mute(?:\b|$)|turn.off.*(?:mic|microphone)|麦克风静音|关闭麦克风/i;
  const START_VOICE = /voice.mode|start.voice|microphone|启动语音|语音功能|麦克风/i;

  function classify(signal) {
    const value = String(signal || '').trim();
    if (!value) return '';
    if (END_VOICE.test(value)) return 'close';
    if (UNMUTE_VOICE.test(value)) return 'voice_unmute';
    if (MUTE_VOICE.test(value)) return 'voice_mute';
    if (START_VOICE.test(value)) return 'voice_mode';
    return '';
  }

  return Object.freeze({ classify });
});
