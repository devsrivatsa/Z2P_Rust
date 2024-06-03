use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use z2p::configuration::{get_configuration, DatabaseSettings};
use z2p::email_client::EmailClient;
use z2p::startup::{get_connection_pool, Application};
use z2p::telemetry::{get_subscriber, init_subscriber};
use wiremock::MockServer;

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
        subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    };
});

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16
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

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        //extract the link from one of the request fields
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            //if there are more than 1 links then we should raise an assertion error
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

            //let's make sure we dont call random apis on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };
        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks {
            html,
            plain_text
        }
    }
}
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let subscriber = get_subscriber("test".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let email_server = MockServer::start().await;
    //we are creating a new logical db everytime and then rolling it back
    //randomize configuration to ensure test isolation
    let mut configuration = {
        let mut c = get_configuration().expect("Failed to read configuration");
        c.database.database_name = Uuid::new_v4().to_string(); //modify the db name to random string
        c.application.port = 0;
        c.email_client.base_url = email_server.uri(); //use mockserver as uri
        c
    };

    let application = Application::build(configuration.clone()).await.expect("Failed to bind address");
    let application_port = application.port();
    let address = format!("http://localhost:{}", application_port);
    let _ = tokio::spawn(application.run_until_stopped()); //task to spawn an async function. in this case - the server
    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port: application_port
    }
    //yet to add code to rollback
}
//we want this so that we are able to create dummy databases to run tests
async fn configure_database(config: &DatabaseSettings) -> PgPool {
    //establish connection
    let mut connection =
        PgConnection::connect_with(&config.without_db())
            .await
            .expect("Failed to create database"); //create connection string without dummy database name
    //creating db
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");
    //migrate db --------------------------------------------------------------------this is causing error----------------------
    let connection_pool = PgPool::connect_with(config.without_db().clone())
        .await
        .expect("Failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool //return this connection pool inside the spawn app function
}