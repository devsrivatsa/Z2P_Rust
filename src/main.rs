use z2p::startup::run;
use z2p::configuration::get_configuration;
use z2p::telemetry::{ get_subscriber, init_subscriber };
use std::net::TcpListener;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use secrecy::ExposeSecret;



#[tokio::main]
async fn main() -> Result<(), std::io::Error> {

    //1. set telemetry
    // env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let subscriber = get_subscriber("z2p".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    //2. read configuration
    //panic if we cannot read configuration
    let configuration = get_configuration().expect("Failed to read configuration");

    //3. connect with database
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(
        &configuration.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to postgres");

    //4. set port for listening
    //port 0 will enable the os to scan for any available port and assign a random port
    //let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let address = format!("{}:{}", configuration.application.host ,configuration.application.port);
    let listener= TcpListener::bind(address)?;

    //5. call run from startup
    run(listener, connection_pool)?.await?;

    Ok(())
}



