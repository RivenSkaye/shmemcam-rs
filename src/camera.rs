use std::{
    borrow::Cow,
    cell::SyncUnsafeCell,
    io::Write,
    num::{NonZeroU32, NonZeroUsize},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
};

use nokhwa::{
    pixel_format::YuyvFormat,
    query,
    utils::{ApiBackend, CameraFormat, RequestedFormat, RequestedFormatType, Resolution},
    CallbackCamera,
};

use winmmf::{MMFLock, MemoryMappedFile, Mmf, Namespace, RWLock};

pub static CAMERAS: OnceLock<SyncUnsafeCell<Vec<Mutex<CallbackCamera>>>> = OnceLock::new();
static CAMS_INITIZALIZED: Mutex<AtomicBool> = Mutex::new(AtomicBool::new(false));

pub fn init_cams(
    basename: Option<impl Deref<Target = str>>,
    width: Option<NonZeroU32>,
    height: Option<NonZeroU32>,
) -> Result<(), Cow<'static, str>> {
    let mut logfile =
        std::fs::File::create(std::env::current_exe().unwrap().with_file_name("shmemcam.cam.log")).unwrap();
    let lock = CAMS_INITIZALIZED.lock().map_err(|_| {
        writeln!(logfile, "Could not lock the init mutex, quitting!").unwrap();
        Cow::from("Could not lock the init mutex, quitting!")
    })?;
    if lock.load(std::sync::atomic::Ordering::Acquire) {
        writeln!(logfile, "already initalized cams").unwrap();
        return Ok(());
    }
    let basename = match basename {
        None => "CapturedCam".to_owned(),
        Some(name) => name.to_owned(),
    };
    writeln!(logfile, "Basename '{basename}'").unwrap();
    let caminfo = query(ApiBackend::MediaFoundation).map_err(|_| Cow::from("Failed to query cameras!"))?;
    writeln!(logfile, "cams queried").unwrap();
    let desired_res = Resolution::new(width.map(|w| w.get()).unwrap_or(1024), height.map(|h| h.get()).unwrap_or(576));
    let mut cameras = Vec::with_capacity(caminfo.len());

    #[cfg(feature = "to_pub")]
    let camfile = crate::util::find_camfile();

    for (index, info) in caminfo.iter().enumerate() {
        writeln!(logfile, "Cam {index} providing {info}").unwrap();
        let nextcam = CallbackCamera::new(
            info.index().clone(),
            RequestedFormat::new::<YuyvFormat>(RequestedFormatType::Closest(CameraFormat::new(
                desired_res,
                nokhwa::utils::FrameFormat::YUYV,
                15,
            ))),
            |_| {
                let mut panicfile =
                    std::fs::File::create(std::env::current_exe().unwrap().with_file_name("shmemcam.frame.log"))
                        .unwrap();
                writeln!(panicfile, "Getting frames pre-mmf!").unwrap();
            },
        )
        .map_err(|e| {
            writeln!(logfile, "Couldn't open camera {index} : {}!\n\t>{e}", info.misc()).unwrap();
            e
        })
        .map_err(|_| Cow::from(format!("Couldn't open camera {index} : {info}!")));
        if nextcam.is_err() {
            continue;
        }
        let mut nextcam = nextcam.unwrap();
        writeln!(logfile, "Got camera {index}").unwrap();
        let res = nextcam.resolution();
        if res.is_err() {
            writeln!(logfile, "Couldn't get a resolution on camera {index}").unwrap();
            continue;
        }
        let res = res.unwrap();
        let bufflen = nextcam.poll_frame().unwrap().buffer().len();
        writeln!(logfile, "Buffer size: {bufflen} bytes").unwrap();
        writeln!(logfile, "Camera {index} @ {res}").unwrap();
        fn hook(boop: &std::panic::PanicHookInfo) {
            use std::io::Write;
            let mut panicfile =
                std::fs::File::create(std::env::current_exe().unwrap().with_file_name("shmemcam.panic.log")).unwrap();
            {
                writeln!(panicfile, "panicking harder than a nigger on steroids! {boop}").unwrap();
            }
        }

        std::panic::set_hook(Box::new(hook));
        let mmf = MemoryMappedFile::<RWLock>::new(NonZeroUsize::new(bufflen).unwrap(), "pogchamp", Namespace::GLOBAL);
        writeln!(logfile, "MMF opened?").unwrap();
        if mmf.is_err() {
            writeln!(logfile, "MMF errored out the ass: {}", mmf.unwrap_err()).unwrap();
            continue;
        }
        writeln!(logfile, "MMF opened!").unwrap();
        let mmf = mmf.unwrap();
        writeln!(logfile, "MMF unwrapped").unwrap();
        nextcam
            .set_callback(move |buff| {
                let mut framelog = std::fs::File::create(
                    std::env::current_exe().unwrap().with_file_name(format!("shmemcam.frames-{index}.log")),
                )
                .unwrap();
                if let Err(wrote) = mmf.write_spin(
                    nokhwa::utils::yuyv422_to_rgb(buff.buffer(), false).unwrap(),
                    None::<fn(&dyn MMFLock, _) -> _>,
                ) {
                    writeln!(framelog, "couldn't write frame: {wrote}").unwrap();
                } else {
                    writeln!(framelog, "successfully wrote frame to mmf").unwrap();
                }
            })
            .map_err(|e| writeln!(logfile, "{e}"))
            .unwrap_or(());
        if let Err(e) = nextcam.open_stream() {
            writeln!(logfile, "Couldn't open stream for camera {index}: {e}").unwrap();
            continue;
        }
        cameras.push(Mutex::new(nextcam));
        #[cfg(feature = "to_pub")]
        crate::util::write_camfile(format!("{basename}_{index}"), camfile.as_ref());
        writeln!(logfile, "Set up camera {index}").unwrap();
    }

    if cameras.is_empty() {
        writeln!(logfile, "No cams, no bitches").unwrap();
        //return Err(Cow::from("Couldn't open any cameras, service going down!"));
        return Ok(());
    }

    // If anything goes wrong at this point, panic and bail.
    if let Ok(_) = CAMERAS.set(cameras.into()) {
        writeln!(logfile, "Saving cams globally").unwrap();
        lock.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).unwrap();
        writeln!(logfile, "Set lock state").unwrap();
        Ok(())
    } else {
        writeln!(logfile, "Failed to store them, chief").unwrap();
        Err(Cow::from("Couldn't set the OnceLock"))
    }
}

pub fn close_cams() -> Result<(), Cow<'static, str>> {
    let lock = CAMS_INITIZALIZED.lock().map_err(|_| Cow::from("Couldn't acquire the initializer lock!"))?;
    if !lock.load(std::sync::atomic::Ordering::Acquire) {
        return Ok(());
    }
    let cams = unsafe {
        CAMERAS
            .get()
            .ok_or(Cow::from("Could not acquire the list of registered cameras!"))?
            .get()
            .as_mut()
    }
    .unwrap();
    for (index, cam) in cams.iter().enumerate() {
        let mut cam = cam.lock().unwrap();
        cam.stop_stream()
            .map_err(|e| Cow::from(format!("Couldn't close camera {index} : {}\n{e}", cam.info())))?;
    }
    lock.store(false, Ordering::Release);
    cams.clear();
    Ok(())
}
