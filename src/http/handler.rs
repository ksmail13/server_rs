use crate::http::{request::HttpRequest, response::HttpResponse};

trait Handler {
    fn handle(&self, req: HttpRequest, res: HttpResponse);
}
