use std::{
    collections::HashMap,
    io::{BufWriter, Write},
    net::TcpStream,
};

use crate::http::value::{HttpResponseCode, HttpVersion};

pub struct HttpResponse<'a> {
    version: HttpVersion,
    code: HttpResponseCode,
    header: HashMap<String, Vec<String>>,
    writer: BufWriter<&'a TcpStream>,
    first: bool,
}

impl<'a> HttpResponse<'a> {
    pub fn new(http_version: HttpVersion, writer: BufWriter<&'a TcpStream>) -> Self {
        return Self {
            version: http_version,
            code: HttpResponseCode::Ok,
            header: HashMap::new(),
            writer: writer,
            first: true,
        };
    }
}

impl HttpResponse<'_> {
    pub fn set_response_code(&mut self, code: HttpResponseCode) {
        self.code = code;
    }

    pub fn set_header(&mut self, key: String, value: String) {
        if self.header.contains_key(&key) {
            self.header.get_mut(&key).map(|v| v.push(value));
        } else {
            self.header.insert(key, vec![value]);
        }
    }

    pub fn write_header(&mut self) -> std::io::Result<usize> {
        let status_line = format!(
            "{} {} {}",
            self.version,
            self.code.code(),
            self.code.reason()
        );

        let mut written = match write!(self.writer, "{}\r\n", status_line) {
            Ok(_) => status_line.len() + 1,
            Err(e) => return Err(e),
        };

        for (k, v) in self.header.clone().into_iter() {
            let header_line = format!("{}: {}\r\n", k, v.join(";"));
            match writeln!(self.writer, "{}", header_line) {
                Err(err) => return Err(err),
                Ok(_) => written += header_line.len() + 1,
            }
        }

        return Ok(written);
    }

    pub fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.first {
            if let Err(err) = self.write_header() {
                return Err(err);
            }

            if let Err(e) = write!(self.writer, "\r\n") {
                return Err(e);
            }

            self.first = false;
        }

        return self.writer.write(buf).map(|_| buf.len());
    }

    #[allow(dead_code)]
    pub fn write_vectored(&mut self, buf: &Vec<u8>) -> std::io::Result<usize> {
        return self.write(buf.as_slice());
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        return match self.writer.flush() {
            Err(err) => return Err(err),
            _ => Ok(()),
        };
    }
}
