//! Images a chat model returns in `message.images`.

use base64::Engine as _;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;

use assistant_core::{GeneratedMedia, MediaKind};

/// The MIME type assumed when a download names none.
const DEFAULT_MIME: &str = "image/png";

/// One `images[]` entry.
#[derive(Debug, Deserialize)]
pub struct ImageEntry {
    image_url: ImageUrl,
}

#[derive(Debug, Deserialize)]
struct ImageUrl {
    url: String,
}

/// Appends the URLs of `images` to `urls`, skipping ones already there.
pub fn collect_urls(urls: &mut Vec<String>, images: Vec<ImageEntry>) {
    for image in images {
        if !urls.contains(&image.image_url.url) {
            urls.push(image.image_url.url);
        }
    }
}

/// Decodes data URIs and downloads other URLs, in order.
pub async fn resolve(
    http: &ClientWithMiddleware,
    urls: &[String],
) -> anyhow::Result<Vec<GeneratedMedia>> {
    let mut media = Vec::with_capacity(urls.len());
    for url in urls {
        let (mime, data) = match url.strip_prefix("data:") {
            Some(uri) => decode_data_uri(uri)?,
            None => download(http, url).await?,
        };
        media.push(GeneratedMedia {
            kind: MediaKind::Image,
            mime,
            data,
        });
    }
    Ok(media)
}

/// `{mime};base64,{data}`, the part of a data URI after `data:`.
fn decode_data_uri(uri: &str) -> anyhow::Result<(String, bytes::Bytes)> {
    let (header, encoded) = uri
        .split_once(',')
        .ok_or_else(|| anyhow::anyhow!("OpenRouter returned a malformed image data URI"))?;
    let Some(mime) = header.strip_suffix(";base64") else {
        anyhow::bail!("OpenRouter returned an image data URI that is not base64");
    };
    let data = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|e| anyhow::anyhow!("OpenRouter returned invalid base64 image data: {e}"))?;
    let mime = if mime.is_empty() { DEFAULT_MIME } else { mime };
    Ok((mime.to_owned(), data.into()))
}

async fn download(
    http: &ClientWithMiddleware,
    url: &str,
) -> anyhow::Result<(String, bytes::Bytes)> {
    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to download a generated image: {e}"))?;
    if !resp.status().is_success() {
        anyhow::bail!("Downloading a generated image returned {}", resp.status());
    }
    let mime = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_owned())
        .filter(|m| m.starts_with("image/"))
        .unwrap_or_else(|| DEFAULT_MIME.to_owned());
    let data = resp
        .bytes()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read a generated image: {e}"))?;
    Ok((mime, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(urls: &[&str]) -> Vec<ImageEntry> {
        serde_json::from_value(serde_json::Value::Array(
            urls.iter()
                .map(|u| serde_json::json!({"type": "image_url", "image_url": {"url": u}}))
                .collect(),
        ))
        .unwrap()
    }

    #[test]
    fn repeated_urls_are_collected_once() {
        let mut urls = Vec::new();
        collect_urls(&mut urls, entries(&["a", "b"]));
        collect_urls(&mut urls, entries(&["b", "c"]));
        assert_eq!(urls, ["a", "b", "c"]);
    }

    #[test]
    fn data_uri_without_base64_is_rejected() {
        assert!(decode_data_uri("image/svg+xml,<svg/>").is_err());
        assert!(decode_data_uri("image/png;base64,!!!").is_err());
    }
}
