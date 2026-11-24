use axum::http::StatusCode;

/// Maps a domain or application error to the HTTP status returned by the API.
pub trait ApiErrorStatus {
    fn status_code(self) -> StatusCode;
}
