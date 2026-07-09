use anyhow::{bail, Result};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

pub(crate) fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
    if bytes.len() < 33 || &bytes[..8] != PNG_SIGNATURE {
        bail!("截图不是有效 PNG");
    }
    if &bytes[12..16] != b"IHDR" {
        bail!("PNG 缺少 IHDR");
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if width == 0 || height == 0 {
        bail!("PNG 尺寸无效");
    }
    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_png_ihdr_dimensions() {
        let mut bytes = Vec::from(PNG_SIGNATURE.as_slice());
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&1080u32.to_be_bytes());
        bytes.extend_from_slice(&2400u32.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        assert_eq!(png_dimensions(&bytes).unwrap(), (1080, 2400));
    }
}
