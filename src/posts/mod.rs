use actix_web::{
    App, Error,
    dev::{ServiceFactory, ServiceRequest},
};

pub mod internal;

pub fn register_services<T>(app: App<T>) -> App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>,
{
    app.service(internal::website::deploy::post)
}
