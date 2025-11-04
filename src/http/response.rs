use std::{
    collections::HashMap,
    io::{IoSlice, Write},
    net::TcpStream,
};

use crate::http::value::{HttpResponseCode, HttpVersion};

pub struct HttpResponse<'a> {
    version: HttpVersion,
    code: HttpResponseCode,
    header: HashMap<&'static str, Vec<String>>,
    header_str: HashMap<String, Vec<String>>,
    writer: &'a TcpStream,
    buffer: Vec<Vec<u8>>,
}

impl<'a> HttpResponse<'a> {
    pub fn new(http_version: HttpVersion, writer: &'a TcpStream) -> Self {
        return Self {
            version: http_version,
            code: HttpResponseCode::Ok,
            header: HashMap::new(),
            header_str: HashMap::new(),
            writer: writer,
            buffer: vec![],
        };
    }
}

impl<'a> Write for HttpResponse<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.push(buf.to_vec());

        return Ok(buf.len());
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.set_header(
            "Content-Length",
            self.buffer
                .iter()
                .map(|b| b.len())
                .sum::<usize>()
                .to_string(),
        );
        self.write_header()?;

        let data: Vec<IoSlice<'_>> = self.buffer.iter().map(|b| IoSlice::new(&b)).collect();
        self.writer.write_vectored(&data)?;

        return self.writer.flush();
    }
}

#[allow(dead_code)]
pub trait HeaderSetter<K, V> {
    fn set_header(&mut self, key: K, value: V);

    fn add_header(&mut self, key: K, value: V);
}

impl<'a> HeaderSetter<String, String> for HttpResponse<'a> {
    fn add_header(&mut self, key: String, value: String) {
        if self.header_str.contains_key(&key) {
            self.header_str.get_mut(&key).map(|v| v.push(value));
        } else {
            self.header_str.insert(key, vec![value]);
        }
    }

    fn set_header(&mut self, key: String, value: String) {
        if self.header_str.contains_key(&key) {
            self.header_str.get_mut(&key).map(|v| {
                v.clear();
                v.push(value);
            });
        } else {
            self.header_str.insert(key, vec![value]);
        }
    }
}

impl HeaderSetter<&'static str, String> for HttpResponse<'_> {
    fn set_header(&mut self, key: &'static str, value: String) {
        if self.header.contains_key(key) {
            self.header.get_mut(key).map(|v| {
                v.clear();
                v.push(value);
            });
        } else {
            self.header.insert(key, vec![value]);
        }
    }

    fn add_header(&mut self, key: &'static str, value: String) {
        if self.header.contains_key(key) {
            self.header.get_mut(key).map(|v| v.push(value));
        } else {
            self.header.insert(key, vec![value]);
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
            for (k, v) in self.header.clone().into_iter() {
                written += self.write_header_value(&k.as_bytes(), &v.join(";").as_bytes())?;
            }
        }

        if !self.header_str.is_empty() {
            for (k, v) in self.header_str.clone().into_iter() {
                written += self.write_header_value(&k.as_bytes(), &v.join(";").as_bytes())?;
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
