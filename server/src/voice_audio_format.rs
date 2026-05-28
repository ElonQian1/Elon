//! PCM16 音频格式工具：校验、静音生成、采样率检查。
//!
//! 仅做纯函数级别的工具，方便单元测试，不引入任何 IO 依赖。

use crate::voice_config::{PCM16_BYTES_PER_SAMPLE, REALTIME_CHANNELS, REALTIME_SAMPLE_RATE_HZ};

/// PCM16 输入字节流校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcmCheck {
    Ok,
    /// 字节数不是偶数，PCM16 帧不完整。
    OddBytes,
    /// 单次推送超过上限。
    TooLarge,
}

/// 检查单个二进制帧是否符合 PCM16 约束。
pub fn check_pcm16_frame(bytes: &[u8], max_bytes: usize) -> PcmCheck {
    if bytes.len() > max_bytes {
        return PcmCheck::TooLarge;
    }
    if bytes.len() % PCM16_BYTES_PER_SAMPLE != 0 {
        return PcmCheck::OddBytes;
    }
    PcmCheck::Ok
}

/// 校验客户端声明的格式是否与服务端约定一致。
pub fn check_format_declaration(sample_rate: u32, channels: u16) -> Result<(), String> {
    if sample_rate != REALTIME_SAMPLE_RATE_HZ {
        return Err(format!(
            "sample_rate 必须是 {} Hz，收到 {}",
            REALTIME_SAMPLE_RATE_HZ, sample_rate
        ));
    }
    if channels != REALTIME_CHANNELS {
        return Err(format!(
            "channels 必须是 {}，收到 {}",
            REALTIME_CHANNELS, channels
        ));
    }
    Ok(())
}

/// 生成指定时长的 PCM16 静音字节。
pub fn silence_pcm16(duration_ms: u64) -> Vec<u8> {
    let samples =
        (REALTIME_SAMPLE_RATE_HZ as u64 * duration_ms / 1000) as usize * REALTIME_CHANNELS as usize;
    vec![0u8; samples * PCM16_BYTES_PER_SAMPLE]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_parity_check() {
        assert_eq!(check_pcm16_frame(&[0u8; 960], 4096), PcmCheck::Ok);
        assert_eq!(check_pcm16_frame(&[0u8; 961], 4096), PcmCheck::OddBytes);
        assert_eq!(check_pcm16_frame(&[0u8; 5000], 4096), PcmCheck::TooLarge);
    }

    #[test]
    fn format_declaration_strict() {
        assert!(check_format_declaration(24000, 1).is_ok());
        assert!(check_format_declaration(16000, 1).is_err());
        assert!(check_format_declaration(24000, 2).is_err());
    }

    #[test]
    fn silence_size_matches() {
        // 100ms @ 24kHz mono PCM16 = 2400 samples * 2 bytes = 4800 bytes
        assert_eq!(silence_pcm16(100).len(), 4800);
    }
}
