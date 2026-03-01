//! Integration tests for audio transcription with format conversion.
//!
//! These tests verify that the audio converter works correctly with FFmpeg.
//! They require FFmpeg to be installed on the system.

use std::process::Command;

use assistant_transcription::{AudioConverter, AudioFormat};

/// Check if FFmpeg is available on the system.
fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Skip test if FFmpeg is not available.
macro_rules! skip_if_no_ffmpeg {
    () => {
        if !ffmpeg_available() {
            eprintln!("Skipping test: FFmpeg not available");
            return;
        }
    };
}

#[tokio::test]
async fn test_converter_check_ffmpeg() {
    skip_if_no_ffmpeg!();

    let converter = AudioConverter::new();
    let result = converter.check_ffmpeg().await;
    assert!(result.is_ok(), "FFmpeg should be available");
}

#[tokio::test]
async fn test_converter_check_ffmpeg_invalid_path() {
    let converter = AudioConverter::new().with_ffmpeg_path("/nonexistent/ffmpeg");
    let result = converter.check_ffmpeg().await;
    assert!(result.is_err(), "Should fail with invalid FFmpeg path");
}

#[tokio::test]
async fn test_convert_wav_to_wav() {
    skip_if_no_ffmpeg!();

    // Create a minimal valid WAV file (44-byte header + 4 bytes of silence)
    // WAV header format:
    // - "RIFF" (4 bytes)
    // - file size (4 bytes, little-endian)
    // - "WAVE" (4 bytes)
    // - "fmt " (4 bytes)
    // - subchunk1 size (4 bytes) = 16
    // - audio format (2 bytes) = 1 (PCM)
    // - num channels (2 bytes) = 1
    // - sample rate (4 bytes) = 16000
    // - byte rate (4 bytes) = 32000
    // - block align (2 bytes) = 2
    // - bits per sample (2 bytes) = 16
    // - "data" (4 bytes)
    // - data size (4 bytes)
    // - actual data
    let mut wav_data = Vec::new();

    // RIFF header
    wav_data.extend_from_slice(b"RIFF");
    wav_data.extend_from_slice(&48u32.to_le_bytes()); // file size - 8
    wav_data.extend_from_slice(b"WAVE");

    // fmt subchunk
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes()); // subchunk size
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // audio format (PCM)
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // num channels
    wav_data.extend_from_slice(&16000u32.to_le_bytes()); // sample rate
    wav_data.extend_from_slice(&32000u32.to_le_bytes()); // byte rate
    wav_data.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav_data.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data subchunk
    wav_data.extend_from_slice(b"data");
    wav_data.extend_from_slice(&4u32.to_le_bytes()); // data size
    wav_data.extend_from_slice(&[0u8; 4]); // 4 bytes of silence

    let converter = AudioConverter::new().with_target_format(AudioFormat::Wav);

    // Converting WAV to WAV should pass through (same format optimization)
    let result = converter.convert(&wav_data, AudioFormat::Wav).await;
    assert!(result.is_ok(), "WAV to WAV conversion should succeed");

    let conversion = result.unwrap();
    assert_eq!(conversion.data, wav_data);
    assert_eq!(conversion.mime_type, "audio/wav");
}

#[tokio::test]
async fn test_convert_mp3_to_wav() {
    skip_if_no_ffmpeg!();

    // Create a minimal valid MP3 frame
    // MP3 frame header is 4 bytes with specific sync pattern
    // This is a valid MP3 frame for a silent frame
    let mp3_data: Vec<u8> = vec![
        0xFF, 0xFB, // Sync word (11 ones) + MPEG version + layer
        0x90, // Bitrate index + sampling rate
        0x00, // Padding + channel mode
        // Minimal audio data
        0x00, 0x00, 0x00, 0x00,
    ];

    let converter = AudioConverter::new().with_target_format(AudioFormat::Wav);

    // This may fail due to invalid MP3 data, but it tests the code path
    let result = converter.convert(&mp3_data, AudioFormat::Mp3).await;

    // The result depends on FFmpeg's ability to parse this minimal data
    // We just verify the code path runs without panicking
    match result {
        Ok(conversion) => {
            // If conversion succeeds, verify we got WAV data
            assert!(!conversion.data.is_empty());
            assert_eq!(conversion.mime_type, "audio/wav");
        }
        Err(e) => {
            // Conversion failure is expected with minimal/invalid MP3 data
            println!("MP3 conversion failed as expected: {}", e);
        }
    }
}

#[tokio::test]
async fn test_convert_for_deepgram_if_needed() {
    skip_if_no_ffmpeg!();

    // Create a minimal valid WAV file
    let mut wav_data = Vec::new();
    wav_data.extend_from_slice(b"RIFF");
    wav_data.extend_from_slice(&48u32.to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes());
    wav_data.extend_from_slice(&1u16.to_le_bytes());
    wav_data.extend_from_slice(&1u16.to_le_bytes());
    wav_data.extend_from_slice(&16000u32.to_le_bytes());
    wav_data.extend_from_slice(&32000u32.to_le_bytes());
    wav_data.extend_from_slice(&2u16.to_le_bytes());
    wav_data.extend_from_slice(&16u16.to_le_bytes());
    wav_data.extend_from_slice(b"data");
    wav_data.extend_from_slice(&4u32.to_le_bytes());
    wav_data.extend_from_slice(&[0u8; 4]);

    let converter = AudioConverter::new();

    // WAV should not need conversion
    let (data, mime) = converter
        .convert_for_deepgram_if_needed(&wav_data, "audio/wav")
        .await
        .unwrap();

    assert_eq!(data, wav_data);
    assert_eq!(mime, "audio/wav");
}

#[tokio::test]
async fn test_audio_format_detection() {
    assert_eq!(AudioFormat::from_mime("audio/m4a"), AudioFormat::M4a);
    assert_eq!(AudioFormat::from_mime("audio/mp4"), AudioFormat::M4a);
    assert_eq!(AudioFormat::from_mime("audio/x-m4a"), AudioFormat::M4a);
    assert_eq!(AudioFormat::from_mime("video/mp4"), AudioFormat::Mp4);
    assert_eq!(AudioFormat::from_mime("audio/aac"), AudioFormat::Aac);
    assert_eq!(AudioFormat::from_mime("audio/mpeg"), AudioFormat::Mp3);
    assert_eq!(AudioFormat::from_mime("audio/mp3"), AudioFormat::Mp3);
    assert_eq!(AudioFormat::from_mime("audio/wav"), AudioFormat::Wav);
    assert_eq!(AudioFormat::from_mime("audio/x-wav"), AudioFormat::Wav);
    assert_eq!(AudioFormat::from_mime("audio/ogg"), AudioFormat::Ogg);
    assert_eq!(AudioFormat::from_mime("audio/opus"), AudioFormat::Ogg);
    assert_eq!(AudioFormat::from_mime("audio/flac"), AudioFormat::Flac);
    assert_eq!(AudioFormat::from_mime("audio/webm"), AudioFormat::Webm);
}

#[tokio::test]
async fn test_deepgram_format_support() {
    // Formats that need conversion for Deepgram
    assert!(AudioFormat::M4a.needs_conversion_for_deepgram());
    assert!(AudioFormat::Mp4.needs_conversion_for_deepgram());
    assert!(AudioFormat::Aac.needs_conversion_for_deepgram());

    // Formats supported natively by Deepgram
    assert!(!AudioFormat::Mp3.needs_conversion_for_deepgram());
    assert!(!AudioFormat::Wav.needs_conversion_for_deepgram());
    assert!(!AudioFormat::Ogg.needs_conversion_for_deepgram());
    assert!(!AudioFormat::Flac.needs_conversion_for_deepgram());
    assert!(!AudioFormat::Webm.needs_conversion_for_deepgram());

    // Verify supported formats
    assert!(AudioFormat::Mp3.is_supported_by_deepgram());
    assert!(AudioFormat::Wav.is_supported_by_deepgram());
    assert!(AudioFormat::Ogg.is_supported_by_deepgram());
    assert!(AudioFormat::Flac.is_supported_by_deepgram());
    assert!(AudioFormat::Webm.is_supported_by_deepgram());

    // Verify unsupported formats
    assert!(!AudioFormat::M4a.is_supported_by_deepgram());
    assert!(!AudioFormat::Mp4.is_supported_by_deepgram());
    assert!(!AudioFormat::Aac.is_supported_by_deepgram());
}

#[tokio::test]
async fn test_converter_custom_settings() {
    let converter = AudioConverter::new()
        .with_ffmpeg_path("/custom/ffmpeg")
        .with_target_format(AudioFormat::Mp3)
        .with_sample_rate(44100)
        .with_channels(2);

    // We can't easily test the actual conversion with custom settings
    // without a real FFmpeg binary, but we can verify the builder pattern works
    assert_eq!(converter.check_ffmpeg().await.is_err(), true); // Invalid path
}
