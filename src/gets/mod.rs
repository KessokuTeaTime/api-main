use actix_web::{
    App, Error,
    dev::{ServiceFactory, ServiceRequest},
};

pub mod health;

pub fn register_services<T>(app: App<T>) -> App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>,
{
    app.service(health::get)
}
