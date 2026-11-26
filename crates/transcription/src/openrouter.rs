//! OpenRouter speech-to-text (`POST {base_url}/audio/transcriptions`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use base64::Engine as _;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::debug;

use assistant_core::Secret;
use assistant_llm_provider::retry::{
    ProviderHttpError, RetryConfig, classify_provider_error, parse_retry_after, with_retry_hinted,
};

use crate::converter::AudioFormat;
use crate::pipeline::AudioPipeline;
use crate::provider::{TranscriptionProvider, TranscriptionRequest, TranscriptionResult};

/// OpenRouter's default speech-to-text model.
pub const DEFAULT_MODEL: &str = "openai/whisper-1";

/// Formats OpenRouter transcribes as they are; others are converted first.
const ACCEPTED_FORMATS: &[AudioFormat] = &[
    AudioFormat::Wav,
    AudioFormat::Mp3,
    AudioFormat::Flac,
    AudioFormat::M4a,
    AudioFormat::Ogg,
    AudioFormat::Webm,
    AudioFormat::Aac,
];

pub struct OpenRouterTranscriber {
    http: ClientWithMiddleware,
    base_url: String,
    api_key: Secret,
    model: String,
    language: Option<String>,
    prompt: Option<String>,
    max_chunk_secs: u32,
    /// Set once the model rejected `verbose_json`; later requests use `json`.
    json_only: AtomicBool,
    pipeline: AudioPipeline,
    retry: RetryConfig,
}

impl OpenRouterTranscriber {
    pub fn new(base_url: String, api_key: Secret, max_chunk_secs: u32) -> anyhow::Result<Self> {
        Ok(Self {
            http: assistant_llm_provider::http::build_http_client(120, &RetryConfig::disabled())?,
            base_url,
            api_key,
            model: DEFAULT_MODEL.to_owned(),
            language: None,
            prompt: None,
            max_chunk_secs,
            json_only: AtomicBool::new(false),
            pipeline: AudioPipeline::default(),
            retry: RetryConfig::default(),
        })
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }
}

#[async_trait]
impl TranscriptionProvider for OpenRouterTranscriber {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn accepted_formats(&self) -> Option<&[AudioFormat]> {
        Some(ACCEPTED_FORMATS)
    }

    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> anyhow::Result<TranscriptionResult> {
        let format = AudioFormat::from_mime(&request.mime_type);
        let language = request.language.clone().or_else(|| self.language.clone());
        let duration = match self
            .pipeline
            .probe_duration(&request.audio_data, format)
            .await
        {
            Ok(duration) => duration,
            Err(e) => {
                debug!(error = %e, "Could not read the recording's duration; sending it whole");
                None
            }
        };
        let max = Duration::from_secs(u64::from(self.max_chunk_secs));
        if !duration.is_some_and(|d| d > max) {
            return self
                .transcribe_once(&request.audio_data, format, language.as_deref())
                .await;
        }

        let wav = if format == AudioFormat::Wav {
            request.audio_data
        } else {
            self.pipeline
                .to_wav_16k_mono(&request.audio_data, format)
                .await?
        };
        let chunks = self.pipeline.split(&wav, self.max_chunk_secs).await?;
        debug!(
            chunks = chunks.len(),
            "Transcribing a long recording in chunks"
        );
        let mut texts = Vec::with_capacity(chunks.len());
        let mut detected = None;
        for (i, chunk) in chunks.iter().enumerate() {
            let part = self
                .transcribe_once(chunk, AudioFormat::Wav, language.as_deref())
                .await
                .with_context(|| format!("Transcribing chunk {} of {}", i + 1, chunks.len()))?;
            detected = detected.or(part.language);
            texts.push(part.text.trim().to_owned());
        }
        Ok(TranscriptionResult {
            text: texts.join(" "),
            language: detected,
            duration_secs: duration.map(|d| d.as_secs_f64()),
        })
    }
}

impl OpenRouterTranscriber {
    /// One request, falling back once from `verbose_json` to `json`.
    async fn transcribe_once(
        &self,
        audio: &[u8],
        format: AudioFormat,
        language: Option<&str>,
    ) -> anyhow::Result<TranscriptionResult> {
        if !self.json_only.load(Ordering::SeqCst) {
            match self.post(audio, format, language, "verbose_json").await {
                Err(e) if is_bad_request(&e) => {
                    debug!(model = %self.model, "verbose_json rejected; using json from now on");
                    self.json_only.store(true, Ordering::SeqCst);
                }
                other => return other,
            }
        }
        let mut result = self.post(audio, format, language, "json").await?;
        result.language = language.map(str::to_owned);
        Ok(result)
    }

    async fn post(
        &self,
        audio: &[u8],
        format: AudioFormat,
        language: Option<&str>,
        response_format: &str,
    ) -> anyhow::Result<TranscriptionResult> {
        let mut body = json!({
            "model": self.model,
            "input_audio": {
                "data": base64::engine::general_purpose::STANDARD.encode(audio),
                "format": format.extension(),
            },
            "response_format": response_format,
        });
        if let Some(language) = language {
            body["language"] = json!(language);
        }
        if let Some(prompt) = &self.prompt {
            body["prompt"] = json!(prompt);
        }
        let url = format!(
            "{}/audio/transcriptions",
            self.base_url.trim_end_matches('/')
        );
        let response: TranscriptionResponse = with_retry_hinted(
            &self.retry,
            "OpenRouter",
            classify_provider_error,
            || async {
                let resp = self
                    .http
                    .post(&url)
                    .bearer_auth(self.api_key.expose())
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("OpenRouter transcription request failed: {e}"))?;
                let status = resp.status();
                if !status.is_success() {
                    let retry_after = resp
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(parse_retry_after);
                    let text = resp.text().await.unwrap_or_default();
                    return Err(http_error(status.as_u16(), retry_after, &text).into());
                }
                resp.json::<TranscriptionResponse>().await.map_err(|e| {
                    anyhow::anyhow!("Failed to parse the OpenRouter transcription: {e}")
                })
            },
        )
        .await?;
        Ok(TranscriptionResult {
            text: response.text,
            language: response.language.as_deref().map(normalize_language),
            duration_secs: response.duration,
        })
    }
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    #[serde(default)]
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
}

fn is_bad_request(err: &anyhow::Error) -> bool {
    err.downcast_ref::<ProviderHttpError>()
        .is_some_and(|e| e.status == 400)
}

/// The error for a non-2xx response, with `error.message` and the status.
fn http_error(
    status: u16,
    retry_after: Option<std::time::Duration>,
    body: &str,
) -> ProviderHttpError {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| body.trim().to_owned());
    let message = match status {
        413 => format!("the audio exceeds the upstream size limit: {message}"),
        _ => format!("OpenRouter transcription: {message}"),
    };
    ProviderHttpError {
        status,
        message,
        error_type: None,
        retry_after,
    }
}

/// A language name as Whisper reports it (`english`), as a BCP-47 code
/// (`en`). Codes and unknown names are returned lowercased.
fn normalize_language(language: &str) -> String {
    let language = language.trim().to_lowercase();
    let code = match language.as_str() {
        "english" => "en",
        "german" => "de",
        "french" => "fr",
        "spanish" => "es",
        "italian" => "it",
        "portuguese" => "pt",
        "dutch" => "nl",
        "polish" => "pl",
        "russian" => "ru",
        "ukrainian" => "uk",
        "turkish" => "tr",
        "arabic" => "ar",
        "hindi" => "hi",
        "japanese" => "ja",
        "korean" => "ko",
        "chinese" => "zh",
        "swedish" => "sv",
        "danish" => "da",
        "norwegian" => "no",
        "finnish" => "fi",
        "czech" => "cs",
        "greek" => "el",
        "hungarian" => "hu",
        "romanian" => "ro",
        "hebrew" => "he",
        "indonesian" => "id",
        "vietnamese" => "vi",
        "thai" => "th",
        _ => return language,
    };
    code.to_owned()
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use base64::Engine as _;
    use serde_json::{Value, json};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use assistant_core::Secret;
    use assistant_llm_provider::retry::RetryConfig;

    use super::OpenRouterTranscriber;
    use crate::provider::{TranscriptionProvider, TranscriptionRequest};

    fn transcriber(server: &MockServer) -> OpenRouterTranscriber {
        OpenRouterTranscriber::new(server.uri(), Secret::new("sk-or"), 50)
            .unwrap()
            .with_retry(RetryConfig {
                max_retries: 2,
                initial_delay_ms: 10,
                jitter: false,
                ..RetryConfig::default()
            })
    }

    fn ogg(data: &[u8]) -> TranscriptionRequest {
        TranscriptionRequest {
            audio_data: data.to_vec(),
            mime_type: "audio/ogg; codecs=opus".into(),
            filename: None,
            language: None,
        }
    }

    async fn bodies(server: &MockServer) -> Vec<Value> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|r| serde_json::from_slice(&r.body).unwrap())
            .collect()
    }

    async fn mount(server: &MockServer, status: u16, body: Value, times: u64) {
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .up_to_n_times(times)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn posts_json_with_input_audio_and_reports_language_and_duration() {
        let server = MockServer::start().await;
        mount(
            &server,
            200,
            json!({"text": "hello there", "language": "english", "duration": 2.5}),
            1,
        )
        .await;
        let provider = transcriber(&server)
            .with_language("en")
            .with_prompt("Names: Cedric");

        let result = provider.transcribe(ogg(b"OggS-data")).await.unwrap();

        assert_eq!(result.text, "hello there");
        assert_eq!(result.language.as_deref(), Some("en"), "english becomes en");
        assert_eq!(result.duration_secs, Some(2.5));
        let body = &bodies(&server).await[0];
        assert_eq!(
            *body,
            json!({
                "model": "openai/whisper-1",
                "input_audio": {
                    "data": base64::engine::general_purpose::STANDARD.encode(b"OggS-data"),
                    "format": "ogg",
                },
                "language": "en",
                "prompt": "Names: Cedric",
                "response_format": "verbose_json",
            })
        );
    }

    #[tokio::test]
    async fn verbose_json_rejection_falls_back_to_json_and_is_remembered() {
        let server = MockServer::start().await;
        mount(
            &server,
            400,
            json!({"error": {"code": 400, "message": "response_format verbose_json is not supported"}}),
            1,
        )
        .await;
        mount(&server, 200, json!({"text": "hi"}), 10).await;
        let provider = transcriber(&server)
            .with_model("openai/gpt-4o-transcribe")
            .with_language("de");

        let first = provider.transcribe(ogg(b"a")).await.unwrap();
        provider.transcribe(ogg(b"b")).await.unwrap();

        assert_eq!(first.text, "hi");
        assert_eq!(first.language.as_deref(), Some("de"), "the configured hint");
        let formats: Vec<Value> = bodies(&server)
            .await
            .iter()
            .map(|b| b["response_format"].clone())
            .collect();
        assert_eq!(
            formats,
            [json!("verbose_json"), json!("json"), json!("json")]
        );
    }

    #[tokio::test]
    async fn payload_too_large_says_so() {
        let server = MockServer::start().await;
        mount(
            &server,
            413,
            json!({"error": {"code": 413, "message": "Payload too large"}}),
            10,
        )
        .await;

        let err = transcriber(&server)
            .transcribe(ogg(b"a"))
            .await
            .unwrap_err()
            .to_string();

        assert!(err.contains("exceeds the upstream size limit"), "{err}");
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            1,
            "413 is not retried"
        );
    }

    #[tokio::test]
    async fn rate_limit_honors_retry_after() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "1")
                    .set_body_json(json!({"error": {"code": 429, "message": "slow down"}})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount(&server, 200, json!({"text": "ok"}), 1).await;

        let started = std::time::Instant::now();
        transcriber(&server).transcribe(ogg(b"a")).await.unwrap();

        assert!(started.elapsed() >= std::time::Duration::from_secs(1));
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }

    /// A `secs`-second WAV tone, or `None` without ffmpeg.
    fn tone(secs: u32) -> Option<Vec<u8>> {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("tone.wav");
        let source = format!("sine=duration={secs}");
        let ok = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &source,
                "-ar",
                "16000",
                "-ac",
                "1",
            ])
            .arg(&out)
            .status()
            .is_ok_and(|s| s.success());
        ok.then(|| std::fs::read(out).unwrap())
    }

    fn wav(data: Vec<u8>) -> TranscriptionRequest {
        TranscriptionRequest {
            audio_data: data,
            mime_type: "audio/wav".into(),
            filename: None,
            language: None,
        }
    }

    /// Answers request `n` with the text `part{n}`.
    async fn mount_numbered(server: &MockServer) {
        let count = AtomicUsize::new(0);
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .respond_with(move |_: &Request| {
                let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                ResponseTemplate::new(200).set_body_json(json!({"text": format!("part{n}")}))
            })
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn long_recordings_are_chunked_in_order() {
        let Some(audio) = tone(180) else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let server = MockServer::start().await;
        mount_numbered(&server).await;

        let result = transcriber(&server).transcribe(wav(audio)).await.unwrap();

        assert_eq!(result.text, "part1 part2 part3 part4");
        let bodies = bodies(&server).await;
        assert_eq!(bodies.len(), 4);
        assert!(bodies.iter().all(|b| b["input_audio"]["format"] == "wav"));
    }

    #[tokio::test]
    async fn one_failed_chunk_fails_the_transcription() {
        let Some(audio) = tone(120) else {
            eprintln!("skipped: ffmpeg is not installed");
            return;
        };
        let server = MockServer::start().await;
        mount(&server, 200, json!({"text": "first"}), 1).await;
        mount(
            &server,
            400,
            json!({"error": {"code": 400, "message": "bad chunk"}}),
            10,
        )
        .await;

        let err = transcriber(&server)
            .transcribe(wav(audio))
            .await
            .unwrap_err();
        assert!(format!("{err:#}").contains("bad chunk"), "{err:#}");
    }

    #[tokio::test]
    async fn audio_of_unknown_duration_is_sent_whole() {
        let server = MockServer::start().await;
        mount_numbered(&server).await;

        // Not real Ogg, so ffprobe can't read a duration.
        let result = transcriber(&server)
            .transcribe(ogg(b"not audio"))
            .await
            .unwrap();

        assert_eq!(result.text, "part1");
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
