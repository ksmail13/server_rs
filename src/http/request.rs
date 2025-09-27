use std::{collections::HashMap, io::BufReader, net::TcpStream};

use crate::http::value::{HttpMethod, HttpVersion};

pub struct HttpRequest<'a> {
    remote_addr: std::net::SocketAddr,
    method: HttpMethod,
    http_version: HttpVersion,
    header: HashMap<String, Vec<String>>,
    param: HashMap<String, Vec<String>>,
    reader: BufReader<&'a TcpStream>,
    // TODO : 필요한건 나중에 추가
}

impl<'a> HttpRequest<'a> {
    pub fn new(
        remote_addr: std::net::SocketAddr,
        method: String,
        http_version: HttpVersion,
        header: HashMap<String, Vec<String>>,
        param: HashMap<String, Vec<String>>,
        reader: BufReader<&'a TcpStream>,
    ) -> Self {
        return HttpRequest {
            remote_addr,
            method: HttpMethod::parse(method.as_str()),
            http_version,
            header,
            param,
            reader,
        };
    }

    pub fn version(&self) -> HttpVersion {
        return self.http_version.clone();
    }
}
