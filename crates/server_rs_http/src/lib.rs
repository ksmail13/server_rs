use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    time::{Duration, SystemTime},
};

use crate::{
    handler::Handler,
    header::{HttpHeaderValue, content_type, date, server},
    request::HttpRequest,
    response::{HeaderSetter, HttpResponse},
    value::{Error, HttpMethod, HttpResponseCode, HttpVersion},
};

use server_rs_tcp::process::{self, Process};

pub mod handler;
pub mod header;
pub mod request;
pub mod response;
pub mod value;

pub struct Http1<T: Handler> {
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
        let header_res: Result<(usize, Vec<String>), Error> = self.read_header(&mut reader);
        if let Err(err) = header_res {
            return Err(process::Error::IoFail {
                msg: format!("Read header failed: ({})", err),
            });
        }
        let (header_readed, headers) = header_res.unwrap();
        let res_request: Result<HttpRequest<'_>, Error> =
            self.init_request(client_addr, &headers, reader);
        if let Err(err) = res_request {
            let mut response = HttpResponse::new(HttpVersion::default(), &stream);

            response.set_response_code(HttpResponseCode::BadRequest);
            response.set_header(&server(HttpHeaderValue::Str("server_rs")));
            response.set_header(&content_type(HttpHeaderValue::Str("text/plain")));
            response.set_header(&date(SystemTime::now()));
            let _ = response.write("Invalid request".as_bytes());
            let _ = response.flush();

            return Err(process::Error::ParseFail {
                msg: err.to_string(),
            });
        }

        let mut request = res_request.unwrap();
        let mut response = HttpResponse::from_request(&request, &stream);

        self.handler.handle(&mut request, &mut response);

        response.set_header(&server(HttpHeaderValue::Str("server_rs")));

        if let Err(err) = response.flush() {
            return Err(process::Error::IoFail {
                msg: err.to_string(),
            });
        }

        return Ok((header_readed, 0));
    }

    fn name(&self) -> String {
        return "http".to_string();
    }
}

impl<T> Http1<T>
where
    T: Handler,
{
    pub fn new(handler: T) -> Self {
        return Http1 { handler };
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

            while buf
                .chars()
                .nth(0)
                .map(|v| v.is_whitespace())
                .unwrap_or(false)
            {
                buf.remove(0);
            }

            // head end
            if buf.is_empty() {
                break;
            }

            if !buf.ends_with("\r\n") {
                return Err(Error::ParseFail(format!("Invalid heaader {}", buf)));
            }

            buf.remove(buf.len() - 1); // delete \n
            buf.remove(buf.len() - 1); // delete \r

            log::trace!("<< {}", buf);

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

        let mut req_line = buf.split(" ");

        let method = req_line
            .next()
            .ok_or_else(|| Error::ParseFail(format!("invalid request line: {}", buf)))?;
        let path_query = req_line
            .next()
            .ok_or_else(|| Error::ParseFail(format!("invalid request line: {}", buf)))?;
        let ver_str = req_line
            .next()
            .ok_or_else(|| Error::ParseFail(format!("invalid request line: {}", buf)))?;

        let version = HttpVersion::parse(ver_str).unwrap_or_default();

        let (path, param) = parse_url(path_query);

        return Ok(HttpRequest::new(
            client_addr,
            HttpMethod::parse(method),
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
            let buf = &reader[i];

            let div_idx = match buf.find(':') {
                Some(idx) => idx,
                None => continue,
            };

            let (key, value) = buf.split_at(div_idx);

            put_data_to_hashmap(&mut header_map, key.trim(), value[1..].trim());
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
            .map(|s| match s.find('=') {
                Some(idx) => s.split_at(idx),
                None => (s, "=true"),
            })
            .fold(HashMap::new(), |mut m, p| {
                put_data_to_hashmap(&mut m, p.0, &p.1[1..]);
                return m;
            }),
    );
}

fn put_data_to_hashmap<'a>(map: &mut HashMap<&'a str, Vec<&'a str>>, key: &'a str, value: &'a str) {
    map.entry(key).or_default().push(value);
}

#[cfg(test)]
mod test {
    use crate::parse_url;

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
