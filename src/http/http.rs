use std::{
    collections::HashMap,
    fmt::Display,
    io::{BufRead, BufReader, BufWriter, Result, Write},
    net::TcpStream,
    time::Duration,
};

use crate::{
    http::{request::HttpRequest, response::HttpResponse, value::HttpVersion},
    process::Process,
};

pub struct Http1 {}

impl Process for Http1 {
    fn process(
        &self,
        stream: TcpStream,
        client_addr: std::net::SocketAddr,
    ) -> std::io::Result<(usize, usize)> {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));

        let request = self.init_request(client_addr, BufReader::new(&stream));
        if let Err(read_error) = request {
            return Err(read_error);
        }
        let response = HttpResponse::new(BufWriter::new(&stream));

        return Ok((0, 0));
    }

    fn name(&self) -> String {
        return "http".to_string();
    }
}

impl Http1 {
    /**
     * Read header part of HTTP request
     */
    fn init_request<'a>(
        &self,
        client_addr: std::net::SocketAddr,
        mut reader: BufReader<&'a TcpStream>,
    ) -> Result<HttpRequest<'a>> {
        let mut buf = String::new();
        let read_result = reader.read_line(&mut buf);

        if let Err(err) = read_result {
            return Err(err);
        }

        let req_line: Vec<&str> = buf.split(" ").collect();
        let method = req_line[0];
        let path_query = req_line[1];
        let version = req_line[2];

        return Ok(HttpRequest::new(
            client_addr,
            method.to_string(),
            HttpVersion::parse(version.to_string()),
            self.init_header(&mut reader),
            self.init_param(path_query),
            reader,
        ));
    }

    fn init_header(&self, reader: &mut BufReader<&TcpStream>) -> HashMap<String, Vec<String>> {
        let mut buf = String::new();
        let mut header_map: HashMap<String, Vec<String>> = HashMap::new();
        while let Ok(line_size) = reader.read_line(&mut buf) {
            if line_size <= 0 {
                break;
            }

            let header_line: Vec<&str> = buf.split(":").collect();
            if header_line.len() <= 2 {
                continue;
            }
            let key = header_line[0].trim().to_string();
            let value = header_line[1].trim().to_string();

            Self::put_data_to_hashmap(&mut header_map, key, value);
        }

        return header_map;
    }

    fn init_param(&self, query: &str) -> HashMap<String, Vec<String>> {
        return query
            .split("&")
            .map(|s| s.split("=").collect::<Vec<&str>>())
            .fold(HashMap::new(), |mut m, p| {
                Self::put_data_to_hashmap(&mut m, p[0].to_string(), p[1].to_string());
                return m;
            });
    }

    fn put_data_to_hashmap(map: &mut HashMap<String, Vec<String>>, key: String, value: String) {
        if map.contains_key(&key) {
            map.get_mut(&key).map(|v| v.push(value));
        } else {
            map.insert(key, vec![value]);
        }
    }
}
