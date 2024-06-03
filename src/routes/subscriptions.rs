use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use actix_web::{web, HttpResponse};
use sqlx::{Postgres, Transaction};
use chrono::Utc;
use sqlx;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};


//form data is basically described by me; tailored to my application
#[derive(serde::Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>, email_client: web::Data<EmailClient>, base_url: web::Data<ApplicationBaseUrl>) -> HttpResponse {
    //try_into works because TryFrom was implemented for new_subscriber which converts form to a New Subscriber type
    let new_subscriber = match form.0.try_into() {
        //we use try_into here as we have implemented try_from
        Ok(form) => form,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };
    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subsciber_id) => subsciber_id,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };
    let subscription_token = generate_subscription_token();
    if store_token(&mut transaction, subscriber_id, &subscription_token).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    //sending a useless email to the new subscriber; ignore email delivery errors for now.
    if send_confirmation_email(&email_client, new_subscriber, &base_url.0, &subscription_token).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Saving new subscriber details in the database", skip(new_subscriber, transaction))]
pub async fn insert_subscriber(transaction: &mut Transaction<'_, Postgres>, new_subscriber: &NewSubscriber) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}
#[tracing::instrument(name = "Send confirmation email to a new subscriber", skip(email_client, new_subscriber, base_url))]
pub async fn send_confirmation_email(email_client: &EmailClient, new_subscriber: NewSubscriber, base_url: &str, subscription_token: &str) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{}/subscriptions/confirm?subscription_token={}", base_url, subscription_token);
    let plain_body = &format!("Welcome to our newsletter! \nVisit {} to confirm your subscription.", confirmation_link);
    let html_body = &format!("Welcome to our newsletter!<br /> Click <a href=\"{}\">here</a> to confirm your subscription.", confirmation_link);
    email_client.send_email(
        new_subscriber.email,
        "Welcome!",
        &html_body,
        &plain_body
    ).await
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(name="Store subsciption token in the database", skip(transaction, subscription_token))]
pub async fn store_token(transaction:&mut Transaction<'_, Postgres>, subscriber_id:Uuid, subscription_token:&str) -> Result<(), sqlx::Error> {
    sqlx::query!(r#"INSERT INTO subscriptions_tokens (subscription_token, subscriber_id) VALUES ($1, $2)"#, subscription_token, subscriber_id)
        .execute(transaction)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}
