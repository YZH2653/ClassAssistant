// PCM s16le → WAV 封装（MiMo ASR 仅支持 wav / mp3）

// WAV 文件头（44 字节）
pub fn wav_header(data_len: u32, sample_rate: u32, channels: u16) -> [u8; 44] {
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let mut out = [0u8; 44];
    out[0..4].copy_from_slice(b"RIFF");
    out[4..8].copy_from_slice(&(36 + data_len).to_le_bytes());
    out[8..12].copy_from_slice(b"WAVE");
    out[12..16].copy_from_slice(b"fmt ");
    out[16..20].copy_from_slice(&16u32.to_le_bytes());
    out[20..22].copy_from_slice(&1u16.to_le_bytes());
    out[22..24].copy_from_slice(&channels.to_le_bytes());
    out[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    out[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    out[32..34].copy_from_slice(&block_align.to_le_bytes());
    out[34..36].copy_from_slice(&16u16.to_le_bytes());
    out[36..40].copy_from_slice(b"data");
    out[40..44].copy_from_slice(&data_len.to_le_bytes());
    out
}

// 生成 WAV 字节
pub fn pcm_s16le_to_wav(pcm: &[u8], sample_rate: u32, channels: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(&wav_header(pcm.len() as u32, sample_rate, channels));
    out.extend_from_slice(pcm);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_correct() {
        let pcm = vec![0u8; 320];
        let wav = pcm_s16le_to_wav(&pcm, 16000, 1);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 320);
        let sample_rate = u32::from_le_bytes(wav[24..28].try_into().unwrap());
        assert_eq!(sample_rate, 16000);
        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(data_len, 320);
    }
}
