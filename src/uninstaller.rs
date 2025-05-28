#![windows_subsystem = "windows"]

use std::num::NonZeroUsize;

use windows_ext::{ext::QWordExt, minwindef::*};
use winmmf::{MMFLock, MemoryMappedFile, Mmf, Namespace, RWLock};

fn main() -> windows_service::Result<()> {
    use std::{
        io::Write,
        thread::sleep,
        time::{Duration, Instant},
    };

    use windows_service::{
        service::{ServiceAccess, ServiceState},
        service_manager::{ServiceManager, ServiceManagerAccess},
    };
    use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;

    fn hook(boop: &std::panic::PanicHookInfo) {
        use std::io::Write;
        let mut panicfile =
            std::fs::File::create(std::env::current_exe().unwrap().with_file_name("shmemcam.panic.log")).unwrap();
        {
            writeln!(panicfile, "panicking harder than a nigger on steroids! {boop}").unwrap();
        }
    }

    std::panic::set_hook(Box::new(hook));

    let mut logfile =
        std::fs::File::create(std::env::current_exe().unwrap().with_file_name("shmemcam.panic.log")).unwrap();
    let bufflen = 1179648;
    writeln!(logfile, "Set buffer length to {:?}", bufflen.split()).unwrap();
    let mmf = MemoryMappedFile::<RWLock>::new(NonZeroUsize::new(bufflen).unwrap(), "pogchamp", Namespace::GLOBAL);
    writeln!(logfile, "MMF opened?").unwrap();
    if mmf.is_err() {
        writeln!(logfile, "MMF errored out the ass: {}", mmf.unwrap_err()).unwrap();
        panic!("death")
    }
    writeln!(logfile, "MMF opened!").unwrap();
    let mmf = mmf.unwrap();
    writeln!(logfile, "MMF unwrapped").unwrap();
    mmf.write_spin(
        "fgsadjfvdsvbsdlhfsdliufgewlkfglskfglksdvlliusdgsfuglbvlbv".as_bytes(),
        None::<fn(&dyn MMFLock, _) -> _>,
    )
    .unwrap();
    writeln!(logfile, "MMF written to!").unwrap();
    return Ok(());

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service("shmemcam", service_access)?;

    // The service will be marked for deletion as long as this function call succeeds.
    // However, it will not be deleted from the database until it is stopped and all open handles to it are closed.
    service.delete()?;
    // Our handle to it is not closed yet. So we can still query it.
    if service.query_status()?.current_state != ServiceState::Stopped {
        // If the service cannot be stopped, it will be deleted when the system restarts.
        service.stop()?;
    }
    // Explicitly close our open handle to the service. This is automatically called when `service` goes out of scope.
    drop(service);

    // Win32 API does not give us a way to wait for service deletion.
    // To check if the service is deleted from the database, we have to poll it ourselves.
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service("shmemcam", ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                println!("shmemcam is deleted.");
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1));
    }
    println!("shmemcam is marked for deletion.");

    Ok(())
}
