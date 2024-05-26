use z2p::configuration::get_configuration;
use z2p::telemetry::{get_subscriber, init_subscriber};
use z2p::startup::Application;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    //1. set telemetry
    // env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let subscriber = get_subscriber("z2p".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    //2. read configuration
    //panic if we cannot read configuration
    let configuration = get_configuration().expect("Failed to read configuration");

    //5. call run from startup
    let application = Application::build(configuration).await?;

    Ok(())
}
