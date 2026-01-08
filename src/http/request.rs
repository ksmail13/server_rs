use std::io::{BufReader, Read};
use std::collections::HashMap;
use std::net::SocketAddr;

use crate::http::value::{HttpMethod, HttpVersion};

#[allow(dead_code)]
pub struct HttpRequest<'a> {
    remote_addr: &'a SocketAddr,
    method: HttpMethod,
    http_version: HttpVersion,
    path: String,
    header: HashMap<&'a str, Vec<&'a str>>,
    param: HashMap<&'a str, Vec<&'a str>>,
    reader: BufReader<Box<dyn Read + 'a>>,
    // TODO : 필요한건 나중에 추가
}

#[allow(dead_code)]
impl<'a> HttpRequest<'a> {
    pub fn new(
        remote_addr: &'a SocketAddr,
        method: HttpMethod,
        http_version: HttpVersion,
        path: String,
        header: HashMap<&'a str, Vec<&'a str>>,
        param: HashMap<&'a str, Vec<&'a str>>,
        reader: BufReader<Box<dyn Read + 'a>>,
    ) -> Self {
        return HttpRequest {
            remote_addr,
            method: method,
            http_version,
            path,
            header,
            param,
            reader,
        };
    }

    pub fn version(&self) -> HttpVersion {
        return self.http_version.clone();
    }

    pub fn method(&self) -> HttpMethod {
        return self.method.clone();
    }

    pub fn remote_addr(&self) -> SocketAddr {
        return *self.remote_addr;
    }

    pub fn path(&self) -> &str {
        return self.path.as_str();
    }

    pub fn header(&self) -> &HashMap<&'a str, Vec<&'a str>> {
        return &self.header;
    }

    pub fn param(&self) -> &HashMap<&'a str, Vec<&'a str>> {
        return &self.param;
    }

    pub fn reader(&self) -> &BufReader<Box<dyn Read + 'a>> {
        return &self.reader;
    }
}
