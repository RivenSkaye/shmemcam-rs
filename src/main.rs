#![feature(sync_unsafe_cell)]
#[cfg(not(target_os = "windows"))]
compile_error!("This is very clearly a WINDOWS service smh my head. PRs to change this are accepted though!");

pub mod camera;
pub mod util;

pub use camera::*;
pub use util::*;

use std::{ffi::OsString, fmt::Debug, io::Write, num::NonZeroU32, ops::Deref, sync::mpsc, time::Duration};

#[macro_use]
extern crate windows_service;

use windows_service::{
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType},
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result as WinRes,
};
const SERVICE_NAME: &str = "shmemcam";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(gen_service_main, service_main);

pub fn run() -> WinRes<()> {
    service_dispatcher::start(SERVICE_NAME, gen_service_main)
}

pub fn service_main(args: Vec<OsString>) {
    let mut logfile = std::fs::File::create(std::env::current_exe().unwrap().with_file_name("shmemcam.log")).unwrap();
    let mut basename = None;
    let mut pref_width = None;
    let mut pref_height = None;
    for arg in args.iter().map(|s| s.to_string_lossy().to_owned()) {
        writeln!(logfile, "Parsing {arg}!").unwrap();
        if arg.contains('=') {
            if arg.starts_with("--basename") {
                basename = arg.split_once('=').map(|p| p.1.to_owned());
            } else if arg.starts_with("--width") {
                pref_width = arg
                    .split_once('=')
                    .map(|p| p.1.parse::<u32>().ok().map(|n| NonZeroU32::new(n)))
                    .flatten()
                    .flatten()
            } else if arg.starts_with("--height") {
                pref_height = arg
                    .split_once('=')
                    .map(|p| p.1.parse::<u32>().ok().map(|n| NonZeroU32::new(n)))
                    .flatten()
                    .flatten()
            }
        }
    }

    if let Err(err) = runner(basename, pref_width, pref_height) {
        writeln!(logfile, "{err}").unwrap();
    } else {
        writeln!(logfile, "Exiting").unwrap();
    }
    _ = logfile.flush();
}

pub fn runner(
    basename: Option<impl Deref<Target = str> + Clone + std::marker::Send + Debug + 'static>,
    w: Option<NonZeroU32>,
    h: Option<NonZeroU32>,
) -> WinRes<()> {
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

            _ => ServiceControlHandlerResult::NoError,
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

    if let Err(res) = camera::init_cams(basename, w, h) {
        writeln!(
            std::fs::File::open(std::env::current_exe().unwrap().with_file_name("shmemcam.log")).unwrap(),
            "EEP! {res}"
        )
        .unwrap();
    }

    loop {
        match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            _ => continue,
        }
    }

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

fn main() -> WinRes<()> {
    let result = run();
    if let Err(ref res) = result {
        eprintln!("NANI!? {res}");
    }
    result
}
