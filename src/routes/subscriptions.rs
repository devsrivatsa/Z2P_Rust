use actix_web::{web, HttpResponse};
use sqlx;
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;
use tracing::Instrument;
use unicode_segmentation::UnicodeSegmentation;
//form data is basically described by me; tailored to my application
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    if !is_valid(&form.name) {
        return HttpResponse::BadRequest().finish();
    }
    match insert_subscriber(&form, &pool).await { //await can return a success or a failure
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => HttpResponse::InternalServerError().finish()
    }
}


#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
pub async fn insert_subscriber(form: &FormData,  pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

pub fn is_valid(s: &str) -> bool {
    let is_empty_or_whitespace = s.trim().is_empty();
    //count all characters in name including chars in graphemes set
    let is_too_long = s.graphemes(true).count() > 256;
    let forbidden_characters = ['/', '(', ')', '"', '<','>','\\','{','}'];
    let contains_forbidden_characters = s.chars().any(|c| forbidden_characters.contains(&c));
    //return false if any of the conditions were violated
    !(is_too_long || is_empty_or_whitespace || contains_forbidden_characters)
}