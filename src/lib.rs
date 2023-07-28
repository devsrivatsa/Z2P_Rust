pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;

use std::net::TcpListener;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder };
use actix_web::dev::Server;
