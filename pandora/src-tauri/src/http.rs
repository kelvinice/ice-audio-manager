//! Native Windows WinHTTP client wrapper for Pandora.
//!
//! Reuses Windows' built-in TLS/proxy stack via `Win32_Networking_WinHttp` (just like `ice-win`'s
//! LRCLIB client) so Pandora compiles and streams HTTPS audio/art with zero extra TLS crate trees.

use std::ffi::c_void;

use url::Url;
use windows::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTP_PORT, INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
    WINHTTP_ADDREQ_FLAG_ADD, WINHTTP_ADDREQ_FLAG_REPLACE, WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_QUERY_CONTENT_LENGTH, WINHTTP_QUERY_CONTENT_RANGE, WINHTTP_QUERY_CONTENT_TYPE,
    WINHTTP_QUERY_STATUS_CODE, WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen,
    WinHttpOpenRequest, WinHttpQueryDataAvailable, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetTimeouts,
};
use windows::core::{HSTRING, PCWSTR};

pub const DEFAULT_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

struct HInternet(*mut c_void);

// SAFETY: WinHTTP handles opened per-request are uniquely owned and safe to move across threads when not shared concurrently.
unsafe impl Send for HInternet {}

impl Drop for HInternet {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: handle was created by WinHTTP and is owned uniquely by HInternet.
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
}

pub struct HttpResponseMeta {
    pub status: u32,
    pub content_type: Option<String>,
    #[allow(dead_code)]
    pub content_length: Option<u64>,
    pub content_range: Option<String>,
}

pub struct WinHttpStream {
    _session: HInternet,
    _conn: HInternet,
    req: HInternet,
    pub meta: HttpResponseMeta,
}

impl WinHttpStream {
    pub fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, String> {
        let mut avail: u32 = 0;
        // SAFETY: `self.req.0` is an open WinHTTP request handle after WinHttpReceiveResponse.
        unsafe { WinHttpQueryDataAvailable(self.req.0, &raw mut avail) }
            .map_err(|e| format!("WinHttpQueryDataAvailable: {e}"))?;
        if avail == 0 {
            return Ok(0);
        }
        let to_read = (avail as usize).min(buf.len()).max(1);
        let mut read: u32 = 0;
        // SAFETY: `buf` has at least `to_read` bytes available for writing.
        unsafe {
            WinHttpReadData(self.req.0, buf.as_mut_ptr().cast::<c_void>(), to_read as u32, &raw mut read)
                .map_err(|e| format!("WinHttpReadData: {e}"))?;
        }
        Ok(read as usize)
    }

    pub fn read_all(mut self, max_bytes: usize) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let mut buf = [0u8; 32 * 1024];
        loop {
            let n = self.read_chunk(&mut buf)?;
            if n == 0 {
                break;
            }
            if out.len().saturating_add(n) > max_bytes {
                return Err(format!("response exceeded {max_bytes} byte limit"));
            }
            out.extend_from_slice(&buf[..n]);
        }
        Ok(out)
    }
}

pub fn open_request(
    method: &str,
    raw_url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    user_agent: Option<&str>,
    timeout_ms: i32,
) -> Result<WinHttpStream, String> {
    let parsed = Url::parse(raw_url).map_err(|e| format!("invalid URL: {e}"))?;
    let is_https = match parsed.scheme() {
        "https" => true,
        "http" => false,
        other => return Err(format!("unsupported URL scheme: {other}")),
    };
    let host_str = parsed.host_str().ok_or_else(|| "URL missing host".to_string())?;
    let port = parsed.port().unwrap_or(if is_https {
        INTERNET_DEFAULT_HTTPS_PORT as u16
    } else {
        INTERNET_DEFAULT_HTTP_PORT as u16
    });
    let mut path_and_query = parsed.path().to_string();
    if path_and_query.is_empty() {
        path_and_query.push('/');
    }
    if let Some(q) = parsed.query() {
        path_and_query.push('?');
        path_and_query.push_str(q);
    }

    let ua = HSTRING::from(user_agent.unwrap_or(DEFAULT_UA));
    // SAFETY: `ua` is valid null-terminated UTF-16.
    let session = HInternet(unsafe {
        WinHttpOpen(PCWSTR(ua.as_ptr()), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, PCWSTR::null(), PCWSTR::null(), 0)
    });
    if session.0.is_null() {
        return Err("WinHttpOpen failed".into());
    }
    // SAFETY: `session.0` is a valid session handle.
    unsafe {
        let _ = WinHttpSetTimeouts(session.0, 5_000, 5_000, timeout_ms, timeout_ms);
    }

    let host = HSTRING::from(host_str);
    // SAFETY: valid session handle and null-terminated host.
    let conn = HInternet(unsafe { WinHttpConnect(session.0, PCWSTR(host.as_ptr()), port, 0) });
    if conn.0.is_null() {
        return Err("WinHttpConnect failed".into());
    }

    let verb = HSTRING::from(method);
    let obj = HSTRING::from(path_and_query);
    let flags = if is_https { WINHTTP_FLAG_SECURE } else { WINHTTP_OPEN_REQUEST_FLAGS(0) };
    // SAFETY: valid connection handle and null-terminated verb/path strings.
    let req = HInternet(unsafe {
        WinHttpOpenRequest(
            conn.0,
            PCWSTR(verb.as_ptr()),
            PCWSTR(obj.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            flags,
        )
    });
    if req.0.is_null() {
        return Err("WinHttpOpenRequest failed".into());
    }

    for (k, v) in headers {
        let wide: Vec<u16> = format!("{k}: {v}\r\n").encode_utf16().collect();
        // SAFETY: `req.0` is valid and `wide` is a valid UTF-16 slice.
        unsafe {
            let _ = WinHttpAddRequestHeaders(
                req.0,
                &wide,
                WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE,
            );
        }
    }

    let (body_ptr, body_len) = match body {
        Some(b) if !b.is_empty() => (Some(b.as_ptr().cast::<c_void>()), b.len() as u32),
        _ => (None, 0),
    };

    // SAFETY: `req.0` is valid and `body_ptr` stays valid through `WinHttpSendRequest`.
    unsafe {
        WinHttpSendRequest(req.0, None, body_ptr, body_len, body_len, 0)
            .map_err(|e| format!("WinHttpSendRequest: {e}"))?;
        WinHttpReceiveResponse(req.0, std::ptr::null_mut())
            .map_err(|e| format!("WinHttpReceiveResponse: {e}"))?;
    }

    let status = query_header_string(req.0, WINHTTP_QUERY_STATUS_CODE)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let content_type = query_header_string(req.0, WINHTTP_QUERY_CONTENT_TYPE);
    let content_length = query_header_string(req.0, WINHTTP_QUERY_CONTENT_LENGTH).and_then(|s| s.parse::<u64>().ok());
    let content_range = query_header_string(req.0, WINHTTP_QUERY_CONTENT_RANGE);

    Ok(WinHttpStream {
        _session: session,
        _conn: conn,
        req,
        meta: HttpResponseMeta { status, content_type, content_length, content_range },
    })
}

pub fn post_json(raw_url: &str, headers: &[(&str, &str)], json_body: &str) -> Result<(u32, Vec<u8>), String> {
    post_json_with_ua(raw_url, headers, json_body, None)
}

pub fn post_json_with_ua(
    raw_url: &str,
    headers: &[(&str, &str)],
    json_body: &str,
    user_agent: Option<&str>,
) -> Result<(u32, Vec<u8>), String> {
    let mut all_headers = vec![("Content-Type", "application/json")];
    all_headers.extend_from_slice(headers);
    let stream = open_request("POST", raw_url, &all_headers, Some(json_body.as_bytes()), user_agent, 12_000)?;
    let status = stream.meta.status;
    let bytes = stream.read_all(8 * 1024 * 1024)?;
    Ok((status, bytes))
}

fn query_header_string(req: *mut c_void, query: u32) -> Option<String> {
    let mut len_bytes: u32 = 0;
    // SAFETY: probe buffer size first (`len_bytes == 0` -> ERROR_INSUFFICIENT_BUFFER with required bytes).
    unsafe {
        let _ = WinHttpQueryHeaders(req, query, PCWSTR::null(), None, &raw mut len_bytes, std::ptr::null_mut());
    }
    if len_bytes == 0 || len_bytes > 16 * 1024 {
        return None;
    }
    let words = (len_bytes as usize).div_ceil(2);
    let mut buf = vec![0u16; words];
    let mut actual_bytes = len_bytes;
    // SAFETY: `buf` is properly aligned and sized for `actual_bytes`.
    let ok = unsafe {
        WinHttpQueryHeaders(
            req,
            query,
            PCWSTR::null(),
            Some(buf.as_mut_ptr().cast::<c_void>()),
            &raw mut actual_bytes,
            std::ptr::null_mut(),
        )
    };
    if ok.is_err() {
        return None;
    }
    let char_len = (actual_bytes as usize / 2).min(buf.len());
    let s = String::from_utf16_lossy(&buf[..char_len]);
    let trimmed = s.trim_matches('\0').trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}
