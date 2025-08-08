use hyper::{body::Body, service::HttpService};



#[derive(Debug)]
pub struct Runner {

}

#[derive(Debug)]
pub struct RequestBody {

}

impl hyper::body::Body for RequestBody {
    type Data;

    type Error;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        todo!()
    }
}




impl hyper::service::HttpService<RequestBody> for Runner {
    type ResBody;

    type Error;

    type Future;

    fn call(&mut self, req: hyper::Request<RequestBody>) -> Self::Future {
        todo!()
    }
}

impl hyper::sealed::Sealed for Runner {

}