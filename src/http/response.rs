use std::{
    collections::HashMap,
    io::{IoSlice, Write},
    net::TcpStream,
    rc::Rc,
    time::SystemTime,
};

use crate::http::{
    header::{HttpHeader, content_length, date},
    request::HttpRequest,
    value::{HttpMethod, HttpResponseCode, HttpVersion},
};

pub struct HttpResponse<'a> {
    version: HttpVersion,
    code: HttpResponseCode,
    header: HashMap<&'static str, Rc<dyn crate::http::header::ToString>>,
    header_str: HashMap<Rc<String>, Rc<dyn crate::http::header::ToString>>,
    writer: &'a TcpStream,
    buffer: Vec<Vec<u8>>,
    header_only: bool,
}

impl<'a> HttpResponse<'a> {
    pub fn new(version: HttpVersion, writer: &'a TcpStream) -> Self {
        return Self {
            version: version,
            code: HttpResponseCode::Ok,
            header: HashMap::new(),
            header_str: HashMap::new(),
            writer: writer,
            buffer: vec![],
            header_only: false,
        };
    }

    pub fn from_request(request: &HttpRequest, writer: &'a TcpStream) -> Self {
        return Self {
            version: request.version(),
            code: HttpResponseCode::Ok,
            header: HashMap::new(),
            header_str: HashMap::new(),
            writer: writer,
            buffer: vec![],
            header_only: request.method() == HttpMethod::HEAD,
        };
    }
}

impl<'a> Write for HttpResponse<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.push(buf.to_vec());

        return Ok(buf.len());
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.set_header(&content_length(
            self.buffer.iter().map(|b| b.len()).sum::<usize>(),
        ));
        self.set_header(&date(SystemTime::now()));
        self.write_header()?;

        if self.header_only {
            return self.writer.flush();
        }

        let data: Vec<IoSlice<'_>> = self.buffer.iter().map(|b| IoSlice::new(&b)).collect();
        self.writer.write_vectored(&data)?;

        return self.writer.flush();
    }
}

#[allow(dead_code)]
pub trait HeaderSetter<T> {
    fn set_header(&mut self, header: T);
}

impl<'a> HeaderSetter<&HttpHeader> for HttpResponse<'a> {
    fn set_header(&mut self, header: &HttpHeader) {
        let value = header.value().clone();
        if let Some(key) = header.key_str() {
            self.header.insert(key, value);
        } else if let Some(key) = header.key_string() {
            self.header_str.insert(key, value);
        }
    }
}

const LINE_END: &[u8] = "\r\n".as_bytes();
const KV_SEP: &[u8] = ": ".as_bytes();

impl HttpResponse<'_> {
    pub fn set_response_code(&mut self, code: HttpResponseCode) {
        self.code = code;
    }

    pub fn write_header(&mut self) -> std::io::Result<usize> {
        let status_line = format!(
            "{} {} {}",
            self.version,
            self.code.code(),
            self.code.reason()
        );

        let mut written = self.writer.write(status_line.as_bytes())?;
        written += self.writer.write(LINE_END)?;

        if !self.header.is_empty() {
            for (key, value) in self.header.clone().into_iter() {
                written +=
                    self.write_header_value(&key.as_bytes(), value.to_string().as_bytes())?;
            }
        }

        if !self.header_str.is_empty() {
            for (key, value) in self.header_str.clone().into_iter() {
                written +=
                    self.write_header_value(&key.as_bytes(), value.to_string().as_bytes())?;
            }
        }

        written += self.writer.write(LINE_END)?;

        return Ok(written);
    }

    fn write_header_value(&mut self, k: &[u8], v: &[u8]) -> std::io::Result<usize> {
        let mut written = self.writer.write(k)?;
        written += self.writer.write(KV_SEP)?;
        written += self.writer.write(v)?;
        written += self.writer.write(LINE_END)?;

        return Ok(written);
    }
}
