use reqwest::header::{ACCEPT, COOKIE, RANGE, USER_AGENT};
use std::time::Duration;

use super::{
    AssetFetchAttempt, ConnectionError, FIRESTORM_USER_AGENT_HEADER, MESH_FETCH_ACCEPT_HEADER,
    MESH_FETCH_RANGE_HEADER_FIRESTORM, MESH_FETCH_RANGE_HEADER_FULL, TEXTURE_FETCH_ACCEPT_HEADER,
    extract_cookie_header_from_set_cookie, merge_cookie_header,
};

pub async fn fetch_asset_bytes(url: &str, timeout: Duration) -> Result<Vec<u8>, ConnectionError> {
    let urls = vec![url.to_string()];
    fetch_bytes_from_candidate_urls(&urls, timeout, None, None, None).await
}

pub async fn fetch_texture_asset_bytes(
    urls: &[String],
    timeout: Duration,
) -> Result<Vec<u8>, ConnectionError> {
    if urls.is_empty() {
        return Err(ConnectionError::MissingCapability(String::from(
            "GetTexture/ViewerAsset",
        )));
    }
    fetch_bytes_from_candidate_urls(urls, timeout, Some(TEXTURE_FETCH_ACCEPT_HEADER), None, None)
        .await
}

pub async fn fetch_mesh_asset_bytes(
    urls: &[String],
    timeout: Duration,
) -> Result<Vec<u8>, ConnectionError> {
    fetch_mesh_asset_bytes_with_attempts(urls, timeout).await.0
}

pub async fn fetch_mesh_asset_bytes_with_attempts(
    urls: &[String],
    timeout: Duration,
) -> (Result<Vec<u8>, ConnectionError>, Vec<AssetFetchAttempt>) {
    let mut attempts = Vec::new();
    let result = fetch_mesh_asset_bytes_inner(urls, timeout, Some(&mut attempts)).await;
    (result, attempts)
}

async fn fetch_mesh_asset_bytes_inner(
    urls: &[String],
    timeout: Duration,
    mut attempts: Option<&mut Vec<AssetFetchAttempt>>,
) -> Result<Vec<u8>, ConnectionError> {
    if urls.is_empty() {
        return Err(ConnectionError::MissingCapability(String::from(
            "ViewerAsset(mesh_id)",
        )));
    }
    // Firestorm parity: probe with a bounded first range and Firestorm user-agent shape.
    let mut mesh_cookie_header: Option<String> = None;
    let (first_probe, first_probe_cookie) = fetch_bytes_from_candidate_urls_with_cookie_state(
        urls,
        timeout,
        Some(MESH_FETCH_ACCEPT_HEADER),
        Some(MESH_FETCH_RANGE_HEADER_FIRESTORM),
        Some(FIRESTORM_USER_AGENT_HEADER),
        mesh_cookie_header.as_deref(),
        attempts.as_deref_mut(),
    )
    .await;
    mesh_cookie_header =
        merge_cookie_header(mesh_cookie_header.as_deref(), first_probe_cookie.as_deref());
    if first_probe.is_ok() {
        // Our decoder expects full payload bytes, so follow with full-body fetch after parity probe.
        let (full_fetch, full_fetch_cookie) = fetch_bytes_from_candidate_urls_with_cookie_state(
            urls,
            timeout,
            Some(MESH_FETCH_ACCEPT_HEADER),
            None,
            Some(FIRESTORM_USER_AGENT_HEADER),
            mesh_cookie_header.as_deref(),
            attempts.as_deref_mut(),
        )
        .await;
        mesh_cookie_header =
            merge_cookie_header(mesh_cookie_header.as_deref(), full_fetch_cookie.as_deref());
        return match full_fetch {
            Ok(bytes) => Ok(bytes),
            Err(_) => {
                let (full_range_fetch, full_range_cookie) =
                    fetch_bytes_from_candidate_urls_with_cookie_state(
                        urls,
                        timeout,
                        Some(MESH_FETCH_ACCEPT_HEADER),
                        Some(MESH_FETCH_RANGE_HEADER_FULL),
                        Some(FIRESTORM_USER_AGENT_HEADER),
                        mesh_cookie_header.as_deref(),
                        attempts.as_deref_mut(),
                    )
                    .await;
                let _merged_cookie_header = merge_cookie_header(
                    mesh_cookie_header.as_deref(),
                    full_range_cookie.as_deref(),
                );
                full_range_fetch
            }
        };
    }
    // Fallback for paths that reject bounded ranges.
    let (full_range_fetch, full_range_cookie) = fetch_bytes_from_candidate_urls_with_cookie_state(
        urls,
        timeout,
        Some(MESH_FETCH_ACCEPT_HEADER),
        Some(MESH_FETCH_RANGE_HEADER_FULL),
        Some(FIRESTORM_USER_AGENT_HEADER),
        mesh_cookie_header.as_deref(),
        attempts.as_deref_mut(),
    )
    .await;
    mesh_cookie_header =
        merge_cookie_header(mesh_cookie_header.as_deref(), full_range_cookie.as_deref());
    match full_range_fetch {
        Ok(bytes) => Ok(bytes),
        Err(_) => {
            let (full_fetch, _) = fetch_bytes_from_candidate_urls_with_cookie_state(
                urls,
                timeout,
                Some(MESH_FETCH_ACCEPT_HEADER),
                None,
                Some(FIRESTORM_USER_AGENT_HEADER),
                mesh_cookie_header.as_deref(),
                attempts,
            )
            .await;
            full_fetch
        }
    }
}

async fn fetch_bytes_from_candidate_urls(
    urls: &[String],
    timeout: Duration,
    accept_header: Option<&str>,
    range_header: Option<&str>,
    user_agent_header: Option<&str>,
) -> Result<Vec<u8>, ConnectionError> {
    let (result, _) = fetch_bytes_from_candidate_urls_with_cookie_state(
        urls,
        timeout,
        accept_header,
        range_header,
        user_agent_header,
        None,
        None,
    )
    .await;
    result
}

async fn fetch_bytes_from_candidate_urls_with_cookie_state(
    urls: &[String],
    timeout: Duration,
    accept_header: Option<&str>,
    range_header: Option<&str>,
    user_agent_header: Option<&str>,
    cookie_header: Option<&str>,
    mut attempts: Option<&mut Vec<AssetFetchAttempt>>,
) -> (Result<Vec<u8>, ConnectionError>, Option<String>) {
    let client = reqwest::Client::builder()
        .timeout(timeout.max(Duration::from_secs(1)))
        .build();
    let client = match client {
        Ok(client) => client,
        Err(err) => return (Err(ConnectionError::Http(err)), None),
    };
    let mut last_error: Option<ConnectionError> = None;
    let mut merged_cookie_header = cookie_header.map(str::to_string);
    for url in urls {
        let mut request = client.get(url);
        if let Some(accept_header) = accept_header {
            request = request.header(ACCEPT, accept_header);
        }
        if let Some(range_header) = range_header {
            request = request.header(RANGE, range_header);
        }
        if let Some(user_agent_header) = user_agent_header {
            request = request.header(USER_AGENT, user_agent_header);
        }
        if let Some(cookie_header) = merged_cookie_header.as_deref() {
            request = request.header(COOKIE, cookie_header);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                if let Some(attempts) = attempts.as_mut() {
                    (**attempts).push(AssetFetchAttempt {
                        url: url.clone(),
                        range_header: range_header.map(str::to_string),
                        status: None,
                        error: Some(err.to_string()),
                        response_body: None,
                    });
                }
                last_error = Some(ConnectionError::Http(err));
                continue;
            }
        };
        merged_cookie_header = merge_cookie_header(
            merged_cookie_header.as_deref(),
            extract_cookie_header_from_set_cookie(response.headers()).as_deref(),
        );
        let status = response.status();
        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => return (Err(ConnectionError::Http(err)), merged_cookie_header),
        };
        if status.is_success() {
            if let Some(attempts) = attempts.as_mut() {
                (**attempts).push(AssetFetchAttempt {
                    url: url.clone(),
                    range_header: range_header.map(str::to_string),
                    status: Some(status.as_u16()),
                    error: None,
                    response_body: None,
                });
            }
            return (Ok(bytes.to_vec()), merged_cookie_header);
        }
        if let Some(attempts) = attempts.as_mut() {
            (**attempts).push(AssetFetchAttempt {
                url: url.clone(),
                range_header: range_header.map(str::to_string),
                status: Some(status.as_u16()),
                error: None,
                response_body: Some(String::from_utf8_lossy(&bytes).to_string()),
            });
        }
        last_error = Some(ConnectionError::HttpStatus {
            status,
            body: String::from_utf8_lossy(&bytes).to_string(),
        });
    }
    (
        Err(last_error.unwrap_or_else(|| {
            ConnectionError::CapabilityDecode(String::from("asset fetch failed"))
        })),
        merged_cookie_header,
    )
}
