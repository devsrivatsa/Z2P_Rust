use z2p::startup::run;
use z2p::configuration::get_configuration;
use std::net::TcpListener;
use sqlx::{ PgPool };

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    //panic if we cannot read configuration
    let configuration = get_configuration().expect("Failed to read configuration");

    let connection_pool = PgPool::connect(
        &configuration.database.connection_string())
        .await
        .expect("Failed to connect to postgres");
    //port 0 will enable the os to scan for any available port and assign a random port
        //let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener= TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}
//maaooo


