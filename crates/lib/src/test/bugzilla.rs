use once_cell::sync::Lazy;

use crate::service::bugzilla::Service;

pub const BASE: &str = "http://127.0.0.1:8080/";
pub const USER: &str = "bugbite@bugbite.test";
pub const PASSWORD: &str = "bugbite";

pub static SERVICE: Lazy<Service> = Lazy::new(|| {
    let mut service = Service::new(BASE).unwrap();
    service.config.user = Some(USER.to_string());
    service.config.password = Some(PASSWORD.to_string());
    service
});
