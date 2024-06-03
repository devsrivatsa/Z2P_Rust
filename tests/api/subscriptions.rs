use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn subscribe_returns_a_400_on_for_invalid_fields() {
    //arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email")
    ];
    for (body,desc) in test_cases {
        //act
        let response = app.post_subscriptions(body.to_string());
        //assert
        assert_eq!(
            400,
            response.await.status().as_u16(),
            "The API did not return a 400 BAD REQUEST when the payload was {}.",
            desc
        )
    }
}
#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    //arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    //act
    let response = app.post_subscriptions(body.to_string()).await;

    //assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    //arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    //act
    let response = app.post_subscriptions(body.to_string()).await;

    //assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscriptions");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=le20%guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (invalid_body, error_msg) in test_cases {
        let response = app.post_subscriptions(invalid_body.into());
        assert_eq!(
            400,
            response.await.status().as_u16(),
            "The api did not fail with 400 bad request when the payload was {}",
            error_msg
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    //arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    //act
    app.post_subscriptions(body.into()).await;
    //assert
    //mock asserts on drop
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    //arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200)) //not setting expectation as this test is focused on behavior
        .mount(&app.email_server)
        .await;

    //act
    app.post_subscriptions((body.into())).await;

    //assert
    //get the first intercepted request
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);
    // the 2 links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}