//! OpenRouter text-to-speech (`POST {base_url}/audio/speech`).

use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::{Value, json};
use tracing::warn;

use assistant_core::Secret;
use assistant_core::types::transcription::TtsFormat;
use assistant_llm_provider::retry::RetryConfig;

use crate::converter::AudioFormat;
use crate::pipeline::{AudioInput, AudioPipeline};
use crate::provider::{TtsProvider, TtsRequest, TtsResult};

/// OpenRouter's default text-to-speech model.
pub const DEFAULT_MODEL: &str = "openai/gpt-4o-mini-tts";
/// The voice used when none is configured.
pub const DEFAULT_VOICE: &str = "alloy";

/// Settings for [`OpenRouterTts::connect`].
#[derive(Debug, Clone)]
pub struct OpenRouterTtsSettings {
    pub base_url: String,
    pub api_key: Secret,
    pub model: String,
    pub voice: String,
    pub format: TtsFormat,
    pub speed: Option<f32>,
    pub instructions: Option<String>,
    pub pcm_sample_rate: Option<u32>,
}

pub struct OpenRouterTts {
    http: ClientWithMiddleware,
    settings: OpenRouterTtsSettings,
    /// The model's catalog entry, when the catalog was reachable.
    catalog: Option<SpeechModel>,
    pipeline: AudioPipeline,
}

impl OpenRouterTts {
    /// Builds the provider, reading the model's supported formats from the
    /// speech-model catalog. Fails when the configured format needs a
    /// transcode and ffmpeg can't be run. When the catalog can't be
    /// reached, formats are requested directly.
    pub async fn connect(
        settings: OpenRouterTtsSettings,
        pipeline: AudioPipeline,
    ) -> anyhow::Result<Self> {
        let http = assistant_llm_provider::http::build_http_client(60, &RetryConfig::default())?;
        let catalog_http =
            assistant_llm_provider::http::build_http_client(10, &RetryConfig::disabled())?;
        let catalog = match fetch_model(&catalog_http, &settings).await {
            Ok(model) => {
                model.validate(&settings)?;
                Some(model)
            }
            Err(e) => {
                warn!(
                    model = %settings.model,
                    error = %e,
                    "OpenRouter speech catalog unavailable; skipping validation and requesting formats directly"
                );
                None
            }
        };
        let tts = Self {
            http,
            settings,
            catalog,
            pipeline,
        };
        let plan = tts.plan(tts.settings.format)?;
        if plan.wire != tts.settings.format {
            tts.pipeline
                .require(audio_format(plan.wire))
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "OpenRouter model `{}` can't produce {} directly: {e}",
                        tts.settings.model,
                        tts.settings.format.as_str()
                    )
                })?;
        }
        Ok(tts)
    }

    /// Which format to request on the wire for delivering `target`.
    fn plan(&self, target: TtsFormat) -> anyhow::Result<Plan> {
        let wire = match self.catalog.as_ref().map(|c| &c.formats) {
            Some(native) if native.contains(&target) => target,
            Some(native) => *[TtsFormat::Wav, TtsFormat::Mp3, TtsFormat::Pcm]
                .iter()
                .find(|f| native.contains(f))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "OpenRouter model `{}` produces none of wav, mp3 or pcm",
                        self.settings.model
                    )
                })?,
            // Ogg isn't a wire format; ask for WAV and transcode.
            None if target == TtsFormat::Ogg => TtsFormat::Wav,
            None => target,
        };
        Ok(Plan { wire })
    }

    /// The sample rate of the model's raw PCM.
    fn pcm_sample_rate(&self) -> u32 {
        self.settings.pcm_sample_rate.unwrap_or_else(|| {
            if self.settings.model.starts_with("heygen/") {
                44_100
            } else {
                24_000
            }
        })
    }
}

struct Plan {
    wire: TtsFormat,
}

#[async_trait]
impl TtsProvider for OpenRouterTts {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn synthesize(&self, request: TtsRequest) -> anyhow::Result<TtsResult> {
        let target = match request.format.as_deref() {
            Some(name) => TtsFormat::parse(name)?,
            None => self.settings.format,
        };
        let plan = self.plan(target)?;
        let voice = request.voice.as_deref().unwrap_or(&self.settings.voice);
        if let Some(catalog) = &self.catalog {
            catalog.check_voice(&self.settings.model, voice)?;
        }
        let mut body = json!({
            "model": self.settings.model,
            "input": request.text,
            "voice": voice,
            "response_format": plan.wire.as_str(),
        });
        if let Some(speed) = request.speed.or(self.settings.speed) {
            body["speed"] = json!(decimal(speed));
        }
        if let Some(instructions) = &self.settings.instructions {
            body["instructions"] = json!(instructions);
        }
        let url = format!(
            "{}/audio/speech",
            self.settings.base_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .bearer_auth(self.settings.api_key.expose())
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenRouter speech request failed: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            let message = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
                .unwrap_or(text);
            anyhow::bail!("OpenRouter speech returned {status}: {message}");
        }
        let audio = resp
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read OpenRouter speech audio: {e}"))?
            .to_vec();

        let input = match plan.wire {
            TtsFormat::Pcm => AudioInput::Pcm {
                sample_rate: self.pcm_sample_rate(),
            },
            other => AudioInput::Encoded(audio_format(other)),
        };
        let audio_data = if plan.wire == target {
            audio
        } else if target == TtsFormat::Pcm {
            self.pipeline
                .to_pcm_s16le(&audio, input, self.pcm_sample_rate())
                .await?
        } else {
            self.pipeline
                .transcode(&audio, input, audio_format(target))
                .await?
        };
        Ok(TtsResult {
            audio_data,
            mime_type: target.mime_type().to_owned(),
        })
    }
}

/// `value` as the decimal it was written as, so `1.2` isn't sent as
/// `1.2000000476837158`.
fn decimal(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(f64::from(value))
}

/// The pipeline's format for a TTS format.
fn audio_format(format: TtsFormat) -> AudioFormat {
    match format {
        TtsFormat::Mp3 => AudioFormat::Mp3,
        TtsFormat::Wav | TtsFormat::Pcm => AudioFormat::Wav,
        TtsFormat::Ogg => AudioFormat::Ogg,
    }
}

/// What the speech-model catalog says about the configured model.
#[derive(Debug, Clone)]
struct SpeechModel {
    formats: Vec<TtsFormat>,
    /// The documented voices, when the catalog lists them as an enum.
    voices: Option<Vec<String>>,
    speed: Option<(f64, f64)>,
    instructions: bool,
}

impl SpeechModel {
    fn parse(entry: &Value) -> anyhow::Result<Self> {
        let params: Vec<&Value> = entry["output_modalities"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|m| &m["supported_parameters"])
            .collect();
        let strings = |key: &str| -> Vec<String> {
            params
                .iter()
                .filter_map(|p| p[key]["values"].as_array())
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        };
        let formats: Vec<TtsFormat> = strings("response_format")
            .iter()
            .filter_map(|name| TtsFormat::parse(name).ok())
            .collect();
        if formats.is_empty() {
            anyhow::bail!("the speech catalog lists no response formats");
        }
        let voices = params
            .iter()
            .any(|p| p["voice"]["type"] == "enum")
            .then(|| strings("voice"));
        let speed = params.iter().find_map(|p| {
            let range = &p["speed"];
            Some((range["min"].as_f64()?, range["max"].as_f64()?))
        });
        Ok(Self {
            formats,
            voices,
            speed,
            instructions: params.iter().any(|p| p.get("instructions").is_some()),
        })
    }

    /// Fails when `settings` asks for something the model doesn't offer.
    fn validate(&self, settings: &OpenRouterTtsSettings) -> anyhow::Result<()> {
        self.check_voice(&settings.model, &settings.voice)?;
        if let (Some(speed), Some((min, max))) = (settings.speed, self.speed) {
            let speed = decimal(speed);
            if !(min..=max).contains(&speed) {
                anyhow::bail!(
                    "[tts] speed {speed} is outside OpenRouter model `{}`'s range of {min} to {max}",
                    settings.model
                );
            }
        }
        if settings.instructions.is_some() && !self.instructions {
            anyhow::bail!(
                "OpenRouter model `{}` does not accept [tts] instructions",
                settings.model
            );
        }
        Ok(())
    }

    /// Fails when the model lists its voices and `voice` isn't one of them.
    fn check_voice(&self, model: &str, voice: &str) -> anyhow::Result<()> {
        match &self.voices {
            Some(voices) if !voices.iter().any(|v| v == voice) => anyhow::bail!(
                "OpenRouter model `{model}` has no voice `{voice}`; its voices are {}",
                voices.join(", ")
            ),
            _ => Ok(()),
        }
    }
}

/// The model's entry from the speech-model catalog.
async fn fetch_model(
    http: &ClientWithMiddleware,
    settings: &OpenRouterTtsSettings,
) -> anyhow::Result<SpeechModel> {
    let url = format!(
        "{}/audio/speech/models/{}",
        settings.base_url.trim_end_matches('/'),
        settings.model
    );
    let resp = http
        .get(&url)
        .bearer_auth(settings.api_key.expose())
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("OpenRouter speech catalog request failed: {e}"))?;
    if !resp.status().is_success() {
        anyhow::bail!("OpenRouter speech catalog returned {}", resp.status());
    }
    let entry: Value = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse the OpenRouter speech catalog: {e}"))?;
    SpeechModel::parse(&entry)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use serde_json::{Value, json};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_core::Secret;
    use assistant_core::types::transcription::TtsFormat;

    use super::{OpenRouterTts, OpenRouterTtsSettings};
    use crate::pipeline::AudioPipeline;
    use crate::provider::{TtsProvider, TtsRequest};

    fn settings(server: &MockServer, model: &str, format: TtsFormat) -> OpenRouterTtsSettings {
        OpenRouterTtsSettings {
            base_url: server.uri(),
            api_key: Secret::new("sk-or"),
            model: model.into(),
            voice: "alloy".into(),
            format,
            speed: None,
            instructions: None,
            pcm_sample_rate: None,
        }
    }

    /// Serves `model`'s speech catalog entry with `formats`.
    async fn mount_model(server: &MockServer, model: &str, formats: &[&str]) {
        Mock::given(method("GET"))
            .and(path(format!("/audio/speech/models/{model}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": model,
                "output_modalities": [{"type": "speech", "supported_parameters": {
                    "response_format": {"type": "enum", "values": formats},
                    "instructions": {"type": "unknown"},
                }}],
            })))
            .mount(server)
            .await;
    }

    async fn mount_speech(server: &MockServer, body: Vec<u8>, content_type: &str) {
        Mock::given(method("POST"))
            .and(path("/audio/speech"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, content_type))
            .mount(server)
            .await;
    }

    async fn speech_bodies(server: &MockServer) -> Vec<Value> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/audio/speech")
            .map(|r| serde_json::from_slice(&r.body).unwrap())
            .collect()
    }

    fn say(text: &str) -> TtsRequest {
        TtsRequest {
            text: text.into(),
            voice: None,
            format: None,
            speed: None,
        }
    }

    /// One second of 16-bit mono PCM at `rate`, or `None` without ffmpeg.
    fn pcm(rate: u32) -> Option<Vec<u8>> {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("tone.pcm");
        let ok = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=duration=1",
            ])
            .args(["-ar", &rate.to_string(), "-ac", "1", "-f", "s16le"])
            .arg(&out)
            .status()
            .is_ok_and(|s| s.success());
        ok.then(|| std::fs::read(out).unwrap())
    }

    fn wav_tone() -> Option<Vec<u8>> {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("tone.wav");
        let ok = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=duration=1",
            ])
            .arg(&out)
            .status()
            .is_ok_and(|s| s.success());
        ok.then(|| std::fs::read(out).unwrap())
    }

    #[tokio::test]
    async fn sends_voice_speed_and_instructions_and_requests_the_native_format() {
        let server = MockServer::start().await;
        let model = "openai/gpt-4o-mini-tts";
        mount_model(&server, model, &["mp3", "pcm"]).await;
        mount_speech(&server, b"ID3-mp3-bytes".to_vec(), "audio/mpeg").await;
        let mut settings = settings(&server, model, TtsFormat::Mp3);
        settings.speed = Some(1.2);
        settings.instructions = Some("Speak calmly".into());
        let tts = OpenRouterTts::connect(settings, AudioPipeline::default())
            .await
            .unwrap();

        let result = tts
            .synthesize(TtsRequest {
                voice: Some("nova".into()),
                ..say("Hello")
            })
            .await
            .unwrap();

        assert_eq!(result.audio_data, b"ID3-mp3-bytes");
        assert_eq!(result.mime_type, "audio/mpeg");
        let body = &speech_bodies(&server).await[0];
        assert_eq!(
            *body,
            json!({
                "model": model,
                "input": "Hello",
                "voice": "nova",
                "response_format": "mp3",
                "speed": 1.2,
                "instructions": "Speak calmly",
            }),
            "the request voice overrides the configured one"
        );
    }

    #[tokio::test]
    async fn pcm_only_heygen_model_is_read_at_44_1_khz_and_returned_as_mp3() {
        let Some(pcm) = pcm(44_100) else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let server = MockServer::start().await;
        let model = "heygen/heygen-voice-1";
        mount_model(&server, model, &["pcm"]).await;
        mount_speech(&server, pcm, "audio/pcm").await;
        let tts = OpenRouterTts::connect(
            settings(&server, model, TtsFormat::Mp3),
            AudioPipeline::default(),
        )
        .await
        .unwrap();

        let result = tts.synthesize(say("Hi")).await.unwrap();

        assert_eq!(speech_bodies(&server).await[0]["response_format"], "pcm");
        assert_eq!(result.mime_type, "audio/mpeg");
        let wav = tts
            .pipeline
            .transcode(
                &result.audio_data,
                crate::pipeline::AudioInput::Encoded(crate::converter::AudioFormat::Mp3),
                crate::converter::AudioFormat::Wav,
            )
            .await
            .unwrap();
        let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        assert_eq!(rate, 44_100, "PCM was read at 44.1 kHz");
    }

    #[tokio::test]
    async fn gemini_is_asked_for_wav_and_returned_as_ogg() {
        let Some(wav) = wav_tone() else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let server = MockServer::start().await;
        let model = "google/gemini-2.5-flash-preview-tts";
        mount_model(&server, model, &["pcm", "wav"]).await;
        mount_speech(&server, wav, "audio/wav").await;
        let tts = OpenRouterTts::connect(
            settings(&server, model, TtsFormat::Ogg),
            AudioPipeline::default(),
        )
        .await
        .unwrap();

        let result = tts.synthesize(say("Hi")).await.unwrap();

        assert_eq!(speech_bodies(&server).await[0]["response_format"], "wav");
        assert_eq!(result.mime_type, "audio/ogg");
        assert_eq!(&result.audio_data[..4], b"OggS");
    }

    #[tokio::test]
    async fn pcm_sample_rate_override_is_used() {
        let Some(pcm) = pcm(16_000) else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let server = MockServer::start().await;
        let model = "acme/pcm-tts";
        mount_model(&server, model, &["pcm"]).await;
        mount_speech(&server, pcm, "audio/pcm").await;
        let mut settings = settings(&server, model, TtsFormat::Wav);
        settings.pcm_sample_rate = Some(16_000);
        let tts = OpenRouterTts::connect(settings, AudioPipeline::default())
            .await
            .unwrap();

        let result = tts.synthesize(say("Hi")).await.unwrap();

        assert_eq!(result.mime_type, "audio/wav");
        let rate = u32::from_le_bytes([
            result.audio_data[24],
            result.audio_data[25],
            result.audio_data[26],
            result.audio_data[27],
        ]);
        assert_eq!(rate, 16_000);
    }

    #[tokio::test]
    async fn pcm_from_a_wav_only_model_is_raw_samples() {
        let Some(wav) = wav_tone() else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let server = MockServer::start().await;
        let model = "acme/wav-tts";
        mount_model(&server, model, &["wav"]).await;
        mount_speech(&server, wav, "audio/wav").await;
        let tts = OpenRouterTts::connect(
            settings(&server, model, TtsFormat::Mp3),
            AudioPipeline::default(),
        )
        .await
        .unwrap();

        let result = tts
            .synthesize(TtsRequest {
                format: Some("pcm".into()),
                ..say("Hi")
            })
            .await
            .unwrap();

        assert_eq!(result.mime_type, "audio/pcm");
        assert_ne!(&result.audio_data[..4], b"RIFF", "no WAV header");
        // One second of 16-bit mono at the 24 kHz default rate.
        assert!(
            (result.audio_data.len() as i64 - 48_000).abs() < 2_000,
            "{} bytes",
            result.audio_data.len()
        );
    }

    #[tokio::test]
    async fn missing_ffmpeg_fails_the_build_when_transcoding_is_needed() {
        let server = MockServer::start().await;
        let model = "heygen/heygen-voice-1";
        mount_model(&server, model, &["pcm"]).await;
        let pipeline = AudioPipeline::default().with_ffmpeg_path("/nonexistent/ffmpeg");

        let err = OpenRouterTts::connect(settings(&server, model, TtsFormat::Mp3), pipeline)
            .await
            .err()
            .expect("must fail")
            .to_string();
        assert!(err.contains("ffmpeg"), "{err}");
    }

    /// Serves `model` with voices, a speed range and optional instructions.
    async fn mount_model_with(server: &MockServer, model: &str, instructions: bool) {
        let mut params = json!({
            "response_format": {"type": "enum", "values": ["mp3"]},
            "voice": {"type": "enum", "values": ["en_paul_neutral", "en_amy_bright"]},
            "speed": {"type": "range", "min": 0.7, "max": 1.2},
        });
        if instructions {
            params["instructions"] = json!({"type": "unknown"});
        }
        Mock::given(method("GET"))
            .and(path(format!("/audio/speech/models/{model}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": model,
                "output_modalities": [{"type": "speech", "supported_parameters": params}],
            })))
            .mount(server)
            .await;
    }

    const VOXTRAL: &str = "mistralai/voxtral-mini-tts-2603";

    async fn connect_err(settings: OpenRouterTtsSettings) -> String {
        OpenRouterTts::connect(settings, AudioPipeline::default())
            .await
            .err()
            .expect("must fail")
            .to_string()
    }

    #[tokio::test]
    async fn unknown_voice_fails_the_build_listing_the_voices() {
        let server = MockServer::start().await;
        mount_model_with(&server, VOXTRAL, false).await;
        let mut settings = settings(&server, VOXTRAL, TtsFormat::Mp3);
        settings.voice = "nova".into();

        let err = connect_err(settings).await;
        assert!(
            err.contains("nova") && err.contains("en_paul_neutral, en_amy_bright"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn speed_out_of_range_fails_the_build_stating_the_range() {
        let server = MockServer::start().await;
        mount_model_with(&server, VOXTRAL, false).await;
        let mut settings = settings(&server, VOXTRAL, TtsFormat::Mp3);
        settings.voice = "en_paul_neutral".into();
        settings.speed = Some(1.5);

        let err = connect_err(settings).await;
        assert!(err.contains("0.7") && err.contains("1.2"), "{err}");
    }

    #[tokio::test]
    async fn instructions_need_model_support() {
        let server = MockServer::start().await;
        mount_model_with(&server, VOXTRAL, false).await;
        let mut settings = settings(&server, VOXTRAL, TtsFormat::Mp3);
        settings.voice = "en_paul_neutral".into();
        settings.instructions = Some("Speak calmly".into());

        let err = connect_err(settings).await;
        assert!(err.contains("instructions"), "{err}");

        let server = MockServer::start().await;
        mount_model_with(&server, VOXTRAL, true).await;
        let mut settings = settings_for(&server);
        settings.instructions = Some("Speak calmly".into());
        OpenRouterTts::connect(settings, AudioPipeline::default())
            .await
            .expect("a model listing instructions accepts them");
    }

    fn settings_for(server: &MockServer) -> OpenRouterTtsSettings {
        let mut settings = settings(server, VOXTRAL, TtsFormat::Mp3);
        settings.voice = "en_paul_neutral".into();
        settings
    }

    #[tokio::test]
    async fn request_voice_outside_the_enum_fails_without_calling_openrouter() {
        let server = MockServer::start().await;
        mount_model_with(&server, VOXTRAL, false).await;
        mount_speech(&server, b"ID3".to_vec(), "audio/mpeg").await;
        let tts = OpenRouterTts::connect(settings_for(&server), AudioPipeline::default())
            .await
            .unwrap();

        let err = tts
            .synthesize(TtsRequest {
                voice: Some("nova".into()),
                ..say("Hi")
            })
            .await
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("nova") && err.contains("en_paul_neutral"),
            "{err}"
        );
        assert!(
            speech_bodies(&server).await.is_empty(),
            "no speech request was sent"
        );
    }

    #[tokio::test]
    async fn catalog_outage_skips_validation() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/audio/speech/models/{VOXTRAL}")))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        mount_speech(&server, b"ID3".to_vec(), "audio/mpeg").await;
        let mut settings = settings(&server, VOXTRAL, TtsFormat::Mp3);
        settings.voice = "anything".into();
        settings.speed = Some(9.0);

        let tts = OpenRouterTts::connect(settings, AudioPipeline::default())
            .await
            .expect("builds without validation");
        tts.synthesize(say("Hi")).await.unwrap();
        assert_eq!(speech_bodies(&server).await[0]["response_format"], "mp3");
    }

    #[tokio::test]
    async fn request_format_overrides_the_configured_one() {
        let server = MockServer::start().await;
        let model = "openai/gpt-4o-mini-tts";
        mount_model(&server, model, &["mp3", "wav"]).await;
        mount_speech(&server, b"RIFF....".to_vec(), "audio/wav").await;
        let tts = OpenRouterTts::connect(
            settings(&server, model, TtsFormat::Mp3),
            AudioPipeline::default(),
        )
        .await
        .unwrap();

        let result = tts
            .synthesize(TtsRequest {
                format: Some("wav".into()),
                ..say("Hi")
            })
            .await
            .unwrap();

        assert_eq!(speech_bodies(&server).await[0]["response_format"], "wav");
        assert_eq!(result.mime_type, "audio/wav");
    }
}
