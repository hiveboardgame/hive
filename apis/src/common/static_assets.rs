use std::path::PathBuf;

use actix_files::{NamedFile, PathBufWrap};
use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::{
        header::{self, AcceptEncoding, ContentEncoding, Encoding, Header, HeaderValue},
        Method,
    },
    middleware::Next,
    web::Data,
    Error,
};
use leptos::prelude::LeptosOptions;

/// Serve build-time compression behind canonical asset URLs. Files and Compress
/// retain their normal behavior when no suitable sidecar is available.
pub async fn serve_precompressed(
    request: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    if let Some(file) = precompressed_file(&request).await {
        let mut response = file.into_response(request.request());
        response
            .headers_mut()
            .append(header::VARY, HeaderValue::from_static("Accept-Encoding"));
        return Ok(request.into_response(response).map_into_left_body());
    }

    Ok(next.call(request).await?.map_into_right_body())
}

async fn precompressed_file(request: &ServiceRequest) -> Option<NamedFile> {
    let options = request.app_data::<Data<LeptosOptions>>()?;
    if !options.hash_files
        || !matches!(*request.method(), Method::GET | Method::HEAD)
        // NamedFile's range handling assumes uncompressed bytes. Keep range
        // requests on the existing Files path rather than mislabelling a slice.
        || request.headers().contains_key(header::RANGE)
    {
        return None;
    }

    let (directory, relative) = if let Some(relative) = request.path().strip_prefix("/pkg/") {
        (
            PathBuf::from(options.site_root.as_ref()).join("pkg"),
            relative,
        )
    } else {
        (
            PathBuf::from(options.site_root.as_ref()),
            request.path().strip_prefix("/assets/")?,
        )
    };
    // Use the same traversal guard as Files, including percent-decoding rules.
    let relative = PathBufWrap::parse_path(relative, false).ok()?;
    let path = directory.join(relative.as_ref());
    let accept_encoding = AcceptEncoding::parse(request.request()).ok()?;
    let variants = [
        (Encoding::brotli(), ContentEncoding::Brotli, "br"),
        (Encoding::gzip(), ContentEncoding::Gzip, "gz"),
    ];
    let mut supported = vec![Encoding::identity()];
    for (encoding, _, extension) in &variants {
        if path.with_added_extension(extension).is_file() {
            supported.push(encoding.clone());
        }
    }
    let selected = accept_encoding.negotiate(supported.iter())?;
    let (_, content_encoding, extension) = variants
        .iter()
        .find(|(encoding, _, _)| *encoding == selected)?;

    // The canonical file supplies MIME type and filename, not the .br/.gz suffix.
    let original = NamedFile::open_async(&path).await.ok()?;
    let compressed = NamedFile::open_async(path.with_added_extension(extension))
        .await
        .ok()?;
    Some(
        compressed
            .set_content_type(original.content_type().clone())
            .set_content_disposition(original.content_disposition().clone())
            .set_content_encoding(*content_encoding),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use actix_files::Files;
    use actix_web::{
        http::{header, Method, StatusCode},
        middleware::{from_fn, Compress},
        test,
        web,
        App,
    };
    use leptos::prelude::LeptosOptions;
    use uuid::Uuid;

    use super::serve_precompressed;

    #[actix_web::test]
    async fn canonical_urls_serve_negotiated_bytes_with_original_mime_type() {
        let root = std::env::temp_dir().join(format!("hive-compression-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("pkg")).unwrap();
        // Distinct payloads prove the selected sidecar is sent verbatim and is
        // not recompressed by the production Compress middleware.
        for (name, bytes) in [
            ("app.wasm", b"raw".as_slice()),
            ("app.wasm.br", b"brotli".as_slice()),
            ("app.wasm.gz", b"gzip".as_slice()),
        ] {
            fs::write(root.join("pkg").join(name), bytes).unwrap();
        }
        fs::write(root.join("tile.svg"), b"<svg/>").unwrap();
        fs::write(root.join("tile.svg.br"), b"svg-brotli").unwrap();
        fs::write(root.join("pkg/plain.wasm"), b"no-sidecars").unwrap();
        fs::write(root.join("pkg/gzip-only.wasm"), b"raw").unwrap();
        fs::write(root.join("pkg/gzip-only.wasm.gz"), b"gzip-only").unwrap();
        let options = LeptosOptions::builder()
            .output_name("app")
            .site_root(root.to_str().unwrap())
            .hash_files(true)
            .build();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(options))
                .service(
                    web::scope("/pkg")
                        .wrap(from_fn(serve_precompressed))
                        .service(Files::new("", root.join("pkg"))),
                )
                .service(
                    web::scope("/assets")
                        .wrap(from_fn(serve_precompressed))
                        .service(Files::new("", &root)),
                )
                .wrap(Compress::default()),
        )
        .await;

        for (uri, accept, encoding, mime, bytes) in [
            (
                "/pkg/app.wasm",
                "gzip, br",
                "br",
                "application/wasm",
                "brotli",
            ),
            ("/pkg/app.wasm", "gzip", "gzip", "application/wasm", "gzip"),
            (
                "/pkg/gzip-only.wasm",
                "gzip, br",
                "gzip",
                "application/wasm",
                "gzip-only",
            ),
            (
                "/pkg/app.wasm",
                "br;q=0, gzip",
                "gzip",
                "application/wasm",
                "gzip",
            ),
            (
                "/assets/tile.svg",
                "gzip, br",
                "br",
                "image/svg+xml",
                "svg-brotli",
            ),
        ] {
            let request = test::TestRequest::get()
                .uri(uri)
                .insert_header((header::ACCEPT_ENCODING, accept))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert!(response.status().is_success());
            assert_eq!(
                response.headers().get(header::CONTENT_ENCODING).unwrap(),
                encoding
            );
            assert_eq!(response.headers().get(header::CONTENT_TYPE).unwrap(), mime);
            assert!(response
                .headers()
                .get_all(header::VARY)
                .any(|value| value == "Accept-Encoding"));
            assert_eq!(test::read_body(response).await.as_ref(), bytes.as_bytes());
        }
        for (uri, accept, expected) in [
            ("/pkg/app.wasm", "identity", "raw"),
            ("/pkg/plain.wasm", "identity", "no-sidecars"),
        ] {
            let request = test::TestRequest::get()
                .uri(uri)
                .insert_header((header::ACCEPT_ENCODING, accept))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert!(response.status().is_success());
            assert_eq!(
                test::read_body(response).await.as_ref(),
                expected.as_bytes()
            );
        }

        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/pkg/app.wasm").to_request(),
        )
        .await;
        assert_eq!(test::read_body(response).await.as_ref(), b"raw");

        let head = test::call_service(
            &app,
            test::TestRequest::default()
                .method(Method::HEAD)
                .uri("/pkg/app.wasm")
                .insert_header((header::ACCEPT_ENCODING, "br"))
                .to_request(),
        )
        .await;
        assert!(head.status().is_success());
        assert_eq!(head.headers().get(header::CONTENT_ENCODING).unwrap(), "br");
        let etag = head.headers().get(header::ETAG).unwrap().clone();
        let conditional = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/pkg/app.wasm")
                .insert_header((header::ACCEPT_ENCODING, "br"))
                .insert_header((header::IF_NONE_MATCH, etag))
                .to_request(),
        )
        .await;
        assert_eq!(conditional.status(), StatusCode::NOT_MODIFIED);
        assert!(conditional
            .headers()
            .get_all(header::VARY)
            .any(|value| value == "Accept-Encoding"));

        let range = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/pkg/app.wasm")
                .insert_header((header::ACCEPT_ENCODING, "br"))
                .insert_header((header::RANGE, "bytes=0-1"))
                .to_request(),
        )
        .await;
        assert_eq!(range.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(test::read_body(range).await.as_ref(), b"ra");
        for uri in [
            "/pkg/%2e%2e/.worktree.env",
            "/pkg/.worktree.env",
            "/pkg/absent.wasm",
        ] {
            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri(uri)
                    .insert_header((header::ACCEPT_ENCODING, "br"))
                    .to_request(),
            )
            .await;
            assert!(response.status().is_client_error());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
