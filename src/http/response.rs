use std::{
    collections::HashMap,
    io::{BufWriter, Result, Write},
    net::TcpStream,
};

pub struct HttpResponse<'a> {
    header: HashMap<String, Vec<String>>,
    writer: BufWriter<&'a TcpStream>,
    buf: Vec<u8>,
}

impl HttpResponse<'_> {
    pub fn new<'a>(writer: BufWriter<&'a TcpStream>) -> HttpResponse<'a> {
        return HttpResponse {
            header: HashMap::new(),
            writer: writer,
            buf: vec![0; 8196],
        };
    }

    pub fn set_header(&mut self, key: String, value: String) {
        if self.header.contains_key(&key) {
            self.header.get_mut(&key).map(|v| v.push(value));
        } else {
            self.header.insert(key, vec![value]);
        }
    }

    pub fn write(&mut self, buf: &mut Vec<u8>) {
        self.buf.append(buf);
    }

    pub fn flush(&mut self) -> Result<()> {
        if let Err(err) = self.writer.write_all(self.buf.as_slice()) {
            return Err(err);
        }
        if let Err(err) = self.writer.flush() {
            return Err(err);
        }

        self.buf.clear();

        return Ok(());
    }
}
