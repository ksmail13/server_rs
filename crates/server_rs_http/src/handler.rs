use crate::{request::HttpRequest, response::HttpResponse};

pub trait Handler {
    fn handle(&self, req: &mut HttpRequest, res: &mut HttpResponse);
}
