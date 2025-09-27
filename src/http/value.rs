use std::fmt::Display;

pub enum HttpVersion {
    Http10,
}

impl HttpVersion {
    pub fn parse(_: &str) -> Option<Self> {
        return Some(HttpVersion::Http10);
    }
}

impl Default for HttpVersion {
    fn default() -> Self {
        return HttpVersion::Http10;
    }
}

impl Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_str("HTTP/1.0");
    }
}

impl Clone for HttpVersion {
    fn clone(&self) -> Self {
        match self {
            Self::Http10 => Self::Http10,
        }
    }
}

pub enum HttpMethod {
    GET,
    POST,
    HEAD, // HTTP 1.0
    UNDEFINED(String),
}

impl HttpMethod {
    pub fn parse(str: &str) -> Self {
        return match str.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "HEAD" => HttpMethod::HEAD,
            _ => HttpMethod::UNDEFINED(str.to_string()),
        };
    }
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_fmt(format_args!(
            "{}",
            match self {
                HttpMethod::GET => "GET",
                HttpMethod::POST => "POST",
                HttpMethod::HEAD => "HEAD",
                HttpMethod::UNDEFINED(v) => v,
            }
        ));
    }
}

pub enum HttpResponseCode {
    Ok,
    Created,
    Accepted,
    NoContent,
    MovedPermanetly,
    MovedTemporarily,
    NotModified,
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
}

impl HttpResponseCode {
    pub fn code(&self) -> i32 {
        return match self {
            HttpResponseCode::Ok => 200,
            HttpResponseCode::Created => 201,
            HttpResponseCode::Accepted => 202,
            HttpResponseCode::NoContent => 204,
            HttpResponseCode::MovedPermanetly => 301,
            HttpResponseCode::MovedTemporarily => 302,
            HttpResponseCode::NotModified => 304,
            HttpResponseCode::BadRequest => 400,
            HttpResponseCode::Unauthorized => 401,
            HttpResponseCode::Forbidden => 403,
            HttpResponseCode::NotFound => 404,
            HttpResponseCode::InternalServerError => 500,
            HttpResponseCode::NotImplemented => 501,
            HttpResponseCode::BadGateway => 502,
            HttpResponseCode::ServiceUnavailable => 503,
        };
    }

    pub fn reason(&self) -> &str {
        return match self {
            HttpResponseCode::Ok => "OK",
            HttpResponseCode::Created => "Created",
            HttpResponseCode::Accepted => "Accepted",
            HttpResponseCode::NoContent => "No Content",
            HttpResponseCode::MovedPermanetly => "Moved Permanently",
            HttpResponseCode::MovedTemporarily => "Moved Temporarily",
            HttpResponseCode::NotModified => "Not Modified",
            HttpResponseCode::BadRequest => "Bad Request",
            HttpResponseCode::Unauthorized => "Unauthorized",
            HttpResponseCode::Forbidden => "Forbidden",
            HttpResponseCode::NotFound => "NotFound",
            HttpResponseCode::InternalServerError => "Interna Server Error",
            HttpResponseCode::NotImplemented => "Not Implemented",
            HttpResponseCode::BadGateway => "Bad Gateway",
            HttpResponseCode::ServiceUnavailable => "Service Unavailable",
        };
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    ParseFail(String),
    ReadFail(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Error::ParseFail(m) | Error::ReadFail(m) => {
                f.write_fmt(format_args!("{}: {}", self, m))
            }
        };
    }
}
