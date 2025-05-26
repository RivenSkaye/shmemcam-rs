use std::{
    env::{args, current_exe, var},
    ffi::OsString,
};
use windows_service::{
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

fn main() -> windows_service::Result<()> {
    if args().any(|arg| arg.to_lowercase().contains("help")) {
        println!("Welcome to the installer for the shmemcam service!");
        println!("\tThe installation can be configured through env vars prefixed with `SHMEM_`.\n\t\t\x1b[95mI refuse to parse a commandline\x1b[0m.\n");
        print!("%SHMEM_BASENAME% : controls the name prefix for the MMFs.    ");
        println!("(= {})", var("SHMEM_BASENAME").unwrap_or("unset".into()));
        print!("%SHMEM_WIDTH%    : controls the preferred width for images.  ");
        println!("(= {})", var("SHMEM_WIDTH").unwrap_or("unset".into()));
        print!("%SHMEM_HEIGHT%   : controls the preferred height for images. ");
        println!("(= {})", var("SHMEM_HEIGHT").unwrap_or("unset".into()));
        println!("\tIf the requested resolution isn't offered by any or all connected cameras,\n\tthe closest offer is selected instead.");
        return Ok(());
    }
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let mut runtime_args = Vec::with_capacity(3);
    if let Ok(bn) = var("SHMEM_BASENAME") {
        runtime_args.push(format!("--basename={bn}").into())
    }
    if let Ok(pw) = var("SHMEM_WIDTH") {
        runtime_args.push(format!("--width={pw}").into())
    }
    if let Ok(ph) = var("SHMEM_HEIGHT") {
        runtime_args.push(format!("--height={ph}").into())
    }

    // This example installs the service defined in `examples/shmemcam.rs`.
    // In the real world code you would set the executable path to point to your own binary
    // that implements windows service.
    let service_binary_path = current_exe().unwrap().with_file_name("shmemcam.exe");

    let service_info = ServiceInfo {
        name: OsString::from("shmemcam"),
        display_name: OsString::from("Shared Memory Camera service"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::OnDemand,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: runtime_args,
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };
    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description("Windows service that captures all cameras and exposes them over MMFs")?;
    Ok(())
}
