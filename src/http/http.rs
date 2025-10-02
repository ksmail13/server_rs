use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter},
    net::TcpStream,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    http::{
        handler::Handler,
        request::HttpRequest,
        response::HttpResponse,
        value::{Error, HttpVersion},
    },
    process::{self, Process},
};

pub struct Http1<T: Handler> {
    write_buffer_size: usize,
    handler: T,
}

impl<T> Process for Http1<T>
where
    T: Handler,
{
    fn process(
        &self,
        stream: TcpStream,
        client_addr: std::net::SocketAddr,
    ) -> Result<(usize, usize), process::Error> {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));

        let request = self.init_request(client_addr, BufReader::new(&stream));
        let mut response = HttpResponse::new(
            match request.as_ref().map(|r| r.version()) {
                Ok(r) => r.clone(),
                _ => HttpVersion::Http10,
            },
            BufWriter::with_capacity(self.write_buffer_size, &stream),
        );

        match request {
            Err(read_error) => {
                response.set_header("Content-Type".to_string(), "text/plain".to_string());
                let _ = response.write("Invalid request".as_bytes());
                let _ = response.flush();
                return Err(process::Error::ParseFail {
                    msg: format!("{:?}", read_error),
                });
            }
            Ok(req) => self.handler.handle(req, response),
        }

        return Ok((0, 0));
    }

    fn name(&self) -> String {
        return "http".to_string();
    }
}

impl<T> Http1<T>
where
    T: Handler,
{
    pub fn new(buffer_size: usize, handler: T) -> Self {
        return Http1 {
            write_buffer_size: buffer_size,
            handler,
        };
    }

    /**
     * Read header part of HTTP request
     */
    fn init_request<'a>(
        &self,
        client_addr: std::net::SocketAddr,
        mut reader: BufReader<&'a TcpStream>,
    ) -> Result<HttpRequest<'a>, Error> {
        let mut buf = String::new();
        let read_result = reader.read_line(&mut buf);
        if let Err(err) = read_result {
            return Err(Error::ReadFail(err.to_string()));
        }

        let req_line: Vec<&str> = buf.split(" ").collect();
        let version = HttpVersion::parse(req_line[2]).unwrap_or_default();
        let path_query = req_line[1];

        let (path, param) = parse_url(path_query);

        return Ok(HttpRequest::new(
            client_addr,
            req_line[0].to_string(),
            version,
            path,
            self.init_header(&mut reader),
            param,
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

            put_data_to_hashmap(&mut header_map, key, value);
        }

        return header_map;
    }
}

fn parse_url(query: &str) -> (String, HashMap<String, Vec<String>>) {
    let path_param: Vec<&str> = query.split("?").collect();

    if path_param.len() < 2 {
        return (path_param[0].to_string(), HashMap::new());
    }

    return (
        path_param[0].to_string(),
        path_param[1]
            .split("&")
            .filter(|p| !p.is_empty())
            .map(|s| s.split("=").collect::<Vec<&str>>())
            .fold(HashMap::new(), |mut m, p| {
                put_data_to_hashmap(
                    &mut m,
                    p[0].to_string(),
                    if p.len() >= 2 {
                        p[1].to_string()
                    } else {
                        "true".to_string()
                    },
                );
                return m;
            }),
    );
}

fn put_data_to_hashmap(map: &mut HashMap<String, Vec<String>>, key: String, value: String) {
    if map.contains_key(&key) {
        map.get_mut(&key).map(|v| v.push(value));
    } else {
        map.insert(key, vec![value]);
    }
}

#[cfg(test)]
mod test {
    use crate::http::http::parse_url;

    #[test]
    fn test_parse_url() {
        let (path, param) = parse_url("/test?asdf=asdf&asdf=fdsa");

        assert_eq!(path, "/test");
        assert_eq!(
            param.get("asdf"),
            Some(&vec!["asdf".to_string(), "fdsa".to_string()])
        );
    }

    #[test]
    fn test_no_param() {
        let (path, param) = parse_url("/test");

        assert_eq!(path, "/test");
        assert!(param.is_empty());
    }
}
