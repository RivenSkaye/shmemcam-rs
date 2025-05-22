#[cfg(not(target_os = "windows"))]
compile_error!("This is very clearly a WINDOWS service smh my head");

pub mod camera;
pub mod util;

pub use camera::*;
pub use util::*;

use std::ffi::OsStr;

#[macro_use]
extern crate windows_service;

use windows_service::{
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType},
    service_control_handler, service_dispatcher, Result as winres,
};

const SERVICE_NAME: &str = "shmemcam";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

fn main() {
    println!("Hello, world!");
}
