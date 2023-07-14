use actix_web::{web, HttpResponse};
use sqlx;
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;

//form data is basically described by me; tailored to my application
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String
}

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),form.email,form.name,Utc::now())
        .execute(pool.get_ref()) //get_ref returns an immutable reference to self. i.e PgPool object
        .await
    { //await can return a success or a failure
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            println!("Failed to execute the query due to the error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}