use actix_web::{web, HttpResponse};

pub async fn check_health() -> HttpResponse {
    HttpResponse::Ok().finish()
}
