use crate::routes::{check_health, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use std::net::TcpListener;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;



pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    /*
    web::Data will wrap the reference of the connection variable in ARC.
    This makes the wrapped reference cloneable.
    The clones will be shared to multiple copies of the app, all will be able to access the same variable.
    */
    let db_pool = web::Data::new(db_pool);
    //move so that we are able to capture the connection variable into the closure
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            //health check
            .route("/health_check", web::get().to(check_health))
            //post requests to add subscriptions
            .route("/subscriptions", web::post().to(subscribe))
            //register the db connection as part of the application state
            .app_data(db_pool.clone())
    }).listen(listener)?
        .run();
    Ok(server)
}
// "127.0.0.1:8000"