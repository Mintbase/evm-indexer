use actix_web::HttpResponse;

pub mod contract;
pub mod token;

#[async_trait::async_trait]
pub trait RequestHandler<M> {
    // TODO - use better return type (impl Responder)
    async fn process_request(&self, messages: &[M]) -> HttpResponse;
}
