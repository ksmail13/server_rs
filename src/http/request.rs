use std::{collections::HashMap, io::BufReader, net::TcpStream};

use crate::http::value::HttpVersion;

pub struct HttpRequest<'a> {
    remote_addr: std::net::SocketAddr,
    method: String,
    http_version: HttpVersion,
    header: HashMap<String, Vec<String>>,
    param: HashMap<String, Vec<String>>,
    reader: BufReader<&'a TcpStream>,
    // TODO : 필요한건 나중에 추가
}

impl HttpRequest<'_> {
    pub fn new<'a>(
        remote_addr: std::net::SocketAddr,
        method: String,
        http_version: HttpVersion,
        header: HashMap<String, Vec<String>>,
        param: HashMap<String, Vec<String>>,
        reader: BufReader<&'a TcpStream>,
    ) -> HttpRequest<'a> {
        return HttpRequest {
            remote_addr,
            method,
            http_version,
            header,
            param,
            reader,
        };
    }
}
