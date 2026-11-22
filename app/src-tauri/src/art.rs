use tauri::http::{Method, Request, Response, StatusCode, Uri, header};

/// Native iceart://<id> and WebView2's convertFileSrc(id, "iceart") transport.
/// An ID is one opaque token. Nothing here decodes, joins or opens a filesystem path.
fn art_id(uri: &Uri) -> Option<&str> {
    if uri.scheme_str()? != "iceart" || uri.query().is_some() {
        return None;
    }
    let authority = uri.authority()?.as_str();
    let id = match (authority, uri.path()) {
        ("localhost", path) => path.strip_prefix('/')?,
        (id, "" | "/") => id,
        _ => return None,
    };
    (!id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
        .then_some(id)
}

pub(super) fn serve(request: &Request<Vec<u8>>, lookup: impl FnOnce(&str) -> Option<Vec<u8>>) -> Response<Vec<u8>> {
    let mut response = Response::default();
    response.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS, header::HeaderValue::from_static("nosniff"));
    if request.method() != Method::GET {
        *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
    } else if let Some(bytes) = art_id(request.uri()).and_then(lookup).filter(|b| b.starts_with(b"\x89PNG\r\n\x1a\n")) {
        response.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("image/png"));
        *response.body_mut() = bytes;
    } else {
        *response.status_mut() = StatusCode::NOT_FOUND;
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use ice_core::engine::{Engine, fake::FakeEngine};

    #[test]
    fn both_art_urls_serve_only_engine_pngs() {
        let engine = FakeEngine::new();
        for uri in ["iceart://fake-art-0", "iceart://localhost/fake-art-0"] {
            let request = Request::get(uri).body(vec![]).unwrap();
            let response = serve(&request, |id| engine.art(id));
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
            assert_eq!(response.body(), &engine.art("fake-art-0").unwrap());
            assert_eq!(serve(&request, |_| Some(b"<svg/>".to_vec())).status(), StatusCode::NOT_FOUND);
        }
        let unknown = Request::get("iceart://fake-art-999").body(vec![]).unwrap();
        assert_eq!(serve(&unknown, |id| engine.art(id)).status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn paths_queries_and_other_methods_never_reach_the_engine() {
        for uri in [
            "iceart://localhost/",
            "iceart://id/extra",
            "iceart://localhost/a/b",
            "iceart://localhost/..",
            "iceart://localhost/%2e%2e%2fsecret",
            "iceart://id/?q=1",
            "iceart://user@id/",
            "iceart://id:80/",
            "http://localhost/id",
        ] {
            let request = Request::get(uri).body(vec![]).unwrap();
            assert_eq!(serve(&request, |_| panic!("unexpected lookup")).status(), StatusCode::NOT_FOUND, "{uri}");
        }
        let request = Request::post("iceart://fake-art-0").body(vec![]).unwrap();
        assert_eq!(serve(&request, |_| panic!("unexpected lookup")).status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
