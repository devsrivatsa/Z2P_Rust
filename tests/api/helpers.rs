use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use z2p::configuration::{get_configuration, DatabaseSettings};
use z2p::email_client::EmailClient;
use z2p::startup::{get_connection_pool, Application};
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
impl TestApp {
    pub async fn post_subscriptions(&self, body:String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request")
    }
}
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let subscriber = get_subscriber("test".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    //we are creating a new logical db everytime and then rolling it back
    //randomize configuration to ensure test isolation
    let mut configuration = {
        let mut c = get_configuration().expect("Failed to read configuration");
        c.database.database_name = Uuid::new_v4().to_string(); //modify the db name to random string
        c.application.port = 0;
        c
    };

    let application = Application::build(configuration.clone()).await.expect("Failed to bind address");
    let address = format!("http:..127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped()); //task to spawn an async function. in this case - the server
    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
    }
    //yet to add code to rollback
}
//we want this so that we are able to create dummy databases to run tests
async fn configure_database(config: &DatabaseSettings) -> PgPool {
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