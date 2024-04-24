use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use z2p::configuration::{get_configuration, DatabaseSettings};
use z2p::email_client::EmailClient;
use z2p::startup::run;
use z2p::telemetry::{get_subscriber, init_subscriber};

//ensure that the tracing stack is initialized only once using once_cell
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    let mut subscriber;
    if std::env::var("TEST_LOG").is_ok() {
        subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        // subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::Sink);
        subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let subscriber = get_subscriber("test".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    //we are creating a new logical db everytime and then rolling it back
    let mut configuration = get_configuration().expect("Failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string(); //modify the db name to random string

    let connection_pool = configure_database(&configuration.database).await;

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address in test");
    let email_client = EmailClient::new(configuration.email_client.base_url, sender_email, configuration.email_client.authorization_token);

    let server = run(listener, connection_pool.clone(), email_client).expect("Failed to bind address");

    let _ = tokio::spawn(server); //task to spawn an async function. in this case - the server
    TestApp {
        address,
        db_pool: connection_pool,
    }
    //yet to add code to rollback
}
//we want this so that we are able to create dummy databases to run tests
pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    //establish connection
    let mut connection =
        PgConnection::connect(&config.connection_string_without_db().expose_secret())
            .await
            .expect("Failed to create database"); //create connection string without dummy database name
                                                  //creating db
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");
    //migrate db
    let connection_pool = PgPool::connect(&config.connection_string().expose_secret())
        .await
        .expect("Failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool //return this connection pool inside the spawn app function
}

#[tokio::test]
async fn health_check_works() {
    //spawn app and get address
    let app = spawn_app().await; //this will spawn the server, and return the bound address and port

    //send a request via request client to check the health of the server
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health_check", &app.address)) //constructs a RequestBuilder object with the specified url
        .send() //sends the request and returns a future
        .await //polls for response
        .expect("Failed to execute request!");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
#[tokio::test]
async fn subscribe_returns_a400_on_for_invalid_fields() {
    //arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email")
    ];
    for (body,desc) in test_cases {
        //act
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");
        //assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 BAD REQUEST when the payload was {}.",
            desc
        )
    }
}
#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    //check if valid form data is read through post request
    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscriptions");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}
#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le20%guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (invalid_body, error_msg) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The api did not fail with 400 bad request when the payload was {}",
            error_msg
        );
    }
}
//maaooo
