use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter},
    net::TcpStream,
    time::Duration,
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
        client_addr: &std::net::SocketAddr,
    ) -> Result<(usize, usize), process::Error> {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));

        log::trace!("Read timeout: {:?}", stream.read_timeout());
        log::trace!("Write timeout: {:?}", stream.write_timeout());

        let mut reader = BufReader::new(&stream);
        let header: Result<(usize, Vec<String>), Error> = self.read_header(&mut reader);
        if let Err(err) = header {
            return Err(process::Error::ParseFail {
                msg: format!("Read header failed: ({})", err),
            });
        }
        let binding = header.unwrap();
        let request = self.init_request(client_addr, &binding.1, reader);
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

        return Ok((binding.0, 0));
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

    fn read_header<'a>(
        &self,
        reader: &mut BufReader<&'a TcpStream>,
    ) -> Result<(usize, Vec<String>), Error> {
        let mut res = vec![];
        let mut readed = 0;
        loop {
            let mut buf = String::new();
            let result = reader.read_line(&mut buf);
            if let Err(err) = result {
                return Err(Error::ReadFail(format!("{}", err)));
            }
            readed += result.unwrap();
            buf.remove(buf.len() - 1); // delete \n
            buf.remove(buf.len() - 1); // delete \r

            log::trace!("<< {}", buf);

            // head end
            if buf.is_empty() {
                break;
            }

            res.push(buf);
        }

        return Ok((readed, res));
    }

    /**
     * Read header part of HTTP request
     */
    fn init_request<'a>(
        &self,
        client_addr: &'a std::net::SocketAddr,
        header: &'a Vec<String>,
        reader: BufReader<&'a TcpStream>,
    ) -> Result<HttpRequest<'a>, Error> {
        let buf = &header[0];

        let req_line: Vec<&str> = buf.split(" ").collect();
        let version = HttpVersion::parse(req_line[2]).unwrap_or_default();
        let path_query = req_line[1];

        let (path, param) = parse_url(path_query);

        return Ok(HttpRequest::new(
            client_addr,
            req_line[0].to_string(),
            version,
            path,
            self.init_header(&header),
            param,
            reader,
        ));
    }

    fn init_header<'a>(&self, reader: &'a Vec<String>) -> HashMap<&'a str, Vec<&'a str>> {
        let mut header_map: HashMap<&str, Vec<&str>> = HashMap::new();
        for i in 1..reader.len() {
            let buf = reader[i].trim();

            let header_line: Vec<&str> = buf.split(":").collect();
            if header_line.len() <= 2 {
                continue;
            }
            let key = header_line[0].trim();
            let value = header_line[1].trim();
            put_data_to_hashmap(&mut header_map, key, value);
        }

        return header_map;
    }
}

fn parse_url(query: &str) -> (String, HashMap<&str, Vec<&str>>) {
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
                put_data_to_hashmap(&mut m, p[0], if p.len() >= 2 { p[1] } else { "true" });
                return m;
            }),
    );
}

fn put_data_to_hashmap<'a>(map: &mut HashMap<&'a str, Vec<&'a str>>, key: &'a str, value: &'a str) {
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
        assert_eq!(param.get("asdf"), Some(&vec!["asdf", "fdsa"]));
    }

    #[test]
    fn test_no_param() {
        let (path, param) = parse_url("/test");

        assert_eq!(path, "/test");
        assert!(param.is_empty());
    }
}
