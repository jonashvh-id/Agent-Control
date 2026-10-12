use axum::{
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
static ASSETS: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../frontend");
pub async fn serve(uri: Uri) -> Response {
    let path = if uri.path() == "/" {
        "index.html"
    } else if let Some(path) = uri.path().strip_prefix("/ui/") {
        path
    } else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if path.split('/').any(|s| matches!(s, ".." | "."))
        || !(path == "index.html" || path == "favicon.svg" || path.starts_with("src/"))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    match ASSETS.get_file(path) {
        Some(file) => (
            [(
                header::CONTENT_TYPE,
                mime_guess::from_path(path)
                    .first_or_octet_stream()
                    .to_string(),
            )],
            file.contents().to_vec(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
