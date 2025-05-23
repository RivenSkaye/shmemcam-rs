#[cfg(not(target_os = "windows"))]
compile_error!("This is very clearly a WINDOWS service smh my head. PRs to change this are accepted though!");

pub mod camera;
pub mod util;

pub use camera::*;
pub use util::*;

use std::{ffi::OsStr, sync::mpsc, time::Duration};

#[macro_use]
extern crate windows_service;

use windows_service::{
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType},
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result as WinRes,
};

const SERVICE_NAME: &str = "shmemcam";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

fn main() {
    println!("Hello, world!");
}

fn runner() -> WinRes<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            // Handle stop
            ServiceControl::Stop => {
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }

            // treat the UserEvent as a stop request
            ServiceControl::UserEvent(code) => {
                if code.to_raw() == 130 {
                    shutdown_tx.send(()).unwrap();
                }
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}
