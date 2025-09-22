use std::fmt::Display;

pub enum HttpVersion {
    Http10,
}

impl HttpVersion {
    pub fn parse(_: String) -> Self {
        return HttpVersion::Http10;
    }
}

impl Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_str("http 1.0");
    }
}
