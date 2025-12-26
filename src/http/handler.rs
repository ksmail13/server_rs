use std::{io::Write, time::SystemTime};

use crate::{
    http::{
        header::{HttpHeaderValue, content_type, date, server},
        request::HttpRequest,
        response::{HeaderSetter, HttpResponse},
        value::{Error, HttpResponseCode},
    },
    process,
};

pub trait Handler {
    fn handle(&self, req: &mut HttpRequest, res: &mut HttpResponse);
}

pub trait ErrorHandler {
    fn handle(&self, req: &mut HttpRequest, res: &mut HttpResponse, err: &Error) -> process::Error;
}

struct DefaultErrorHandler {}

impl ErrorHandler for DefaultErrorHandler {
    fn handle(&self, req: &mut HttpRequest, res: &mut HttpResponse, err: &Error) -> process::Error {
        res.set_response_code(HttpResponseCode::BadRequest);
        res.set_header(&server(HttpHeaderValue::Str("server_rs")));
        res.set_header(&content_type(HttpHeaderValue::Str("text/plain")));
        res.set_header(&date(SystemTime::now()));
        let _ = res.write("Invalid request".as_bytes());
        let _ = res.flush();

        return process::Error::ParseFail(err.to_string());
    }
}
