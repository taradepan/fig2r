use crate::error::Fig2rError;
use crate::figma::types::{FileNodesResponse, ImageResponse};

const BASE_URL: &str = "https://api.figma.com/v1";

pub struct FigmaClient {
    token: String,
    client: reqwest::Client,
    base_url: String,
}

/// Result of a single image download
pub struct DownloadResult {
    pub id: String,
    pub url: String,
    pub data: Result<Vec<u8>, Fig2rError>,
}

impl FigmaClient {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: reqwest::Client::new(),
            base_url: BASE_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn with_base_url(token: String, base_url: String) -> Self {
        Self {
            token,
            client: reqwest::Client::new(),
            base_url,
        }
    }

    /// Fetch nodes from a Figma file.
    pub async fn get_nodes(
        &self,
        file_key: &str,
        node_ids: Option<&[&str]>,
    ) -> Result<FileNodesResponse, Fig2rError> {
        let url = match node_ids {
            Some(ids) => {
                let ids_str = ids.join(",");
                format!(
                    "{}/files/{file_key}/nodes?ids={ids_str}&geometry=paths",
                    self.base_url
                )
            }
            None => format!("{}/files/{file_key}", self.base_url),
        };

        let body = self.get_json(&url).await?;
        serde_json::from_str(&body)
            .map_err(|e| Fig2rError::Message(format!("Failed to parse Figma response: {e}")))
    }

    /// Export node images from Figma (returns URLs).
    pub async fn get_image_urls(
        &self,
        file_key: &str,
        node_ids: &[&str],
        format: &str,
        scale: f64,
    ) -> Result<ImageResponse, Fig2rError> {
        let ids_str = node_ids.join(",");
        let url = format!(
            "{}/images/{file_key}?ids={ids_str}&format={format}&scale={scale}",
            self.base_url
        );

        let body = self.get_json(&url).await?;
        serde_json::from_str(&body)
            .map_err(|e| Fig2rError::Message(format!("Failed to parse image response: {e}")))
    }

    /// Download multiple images concurrently using tokio tasks.
    pub async fn download_images_parallel(
        &self,
        items: &[(String, String)],
    ) -> Vec<DownloadResult> {
        let mut handles = Vec::with_capacity(items.len());

        for (id, url) in items {
            let client = self.client.clone();
            let id = id.clone();
            let url = url.clone();
            handles.push(tokio::spawn(async move {
                let data = download_url(&client, &url).await;
                DownloadResult { id, url, data }
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(DownloadResult {
                        id: String::new(),
                        url: String::new(),
                        data: Err(Fig2rError::Message(format!("Task panicked: {e}"))),
                    });
                }
            }
        }
        results
    }

    async fn get_json(&self, url: &str) -> Result<String, Fig2rError> {
        self.client
            .get(url)
            .header("X-Figma-Token", &self.token)
            .send()
            .await
            .map_err(|e| Fig2rError::Message(format!("Figma API failed: {e}")))?
            .text()
            .await
            .map_err(|e| Fig2rError::Message(format!("Failed to read response: {e}")))
    }
}

async fn download_url(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, Fig2rError> {
    client
        .get(url)
        .send()
        .await
        .map_err(|e| Fig2rError::Message(format!("Download failed: {e}")))?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| Fig2rError::Message(format!("Read failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn start_mock_server(response_body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let body_bytes = response_body.as_bytes();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body_bytes.len(),
                    response_body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("http://{addr}/v1")
    }

    #[tokio::test]
    async fn test_get_nodes_with_mock_server() {
        let body = r#"{
            "name": "Mock File",
            "nodes": {
                "1:2": {
                    "document": {
                        "id": "1:2",
                        "name": "Root",
                        "type": "FRAME"
                    }
                }
            }
        }"#;
        let base_url = start_mock_server(body);
        let client = FigmaClient::with_base_url("token".into(), base_url);
        let resp = client.get_nodes("file-key", Some(&["1:2"])).await.unwrap();
        assert_eq!(resp.name, "Mock File");
        assert!(resp.nodes.contains_key("1:2"));
    }
}
