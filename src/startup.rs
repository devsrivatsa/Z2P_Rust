use crate::email_client::EmailClient;
use crate::routes::{check_health, subscribe};
use crate::configuration::{DatabaseSettings, Settings};
use sqlx::postgres::PgPoolOptions;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use secrecy::ExposeSecret;
use tracing_actix_web::TracingLogger;


pub struct Application {
    port: u16,
    server: Server
}
impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");

        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout
        );
        let address = format!("{}:{}", configuration.application.host, configuration.application.port);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, email_client)?;
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }

}

pub fn get_connection_pool(configuration: &DatabaseSettings)->PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    /*
    web::Data will wrap the reference of the connection variable in ARC.
    This makes the wrapped reference cloneable.
    The clones will be shared to multiple copies of the app, all will be able to access the same variable.
    */
    let db_pool = web::Data::new(db_pool);
    //move so that we are able to capture the connection variable into the closure
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            //health check
            .route("/health_check", web::get().to(check_health))
            //post requests to add subscriptions
            .route("/subscriptions", web::post().to(subscribe))
            //register the db connection as part of the application state
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
// "127.0.0.1:8000"

