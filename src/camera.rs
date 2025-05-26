use std::{
    borrow::Cow,
    cell::SyncUnsafeCell,
    num::{NonZeroU32, NonZeroUsize},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
};

use nokhwa::{
    pixel_format::RgbFormat,
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
    let lock = CAMS_INITIZALIZED.lock().map_err(|_| Cow::from("Could not lock the init mutex, quitting!"))?;
    if lock.load(std::sync::atomic::Ordering::Acquire) {
        return Ok(());
    }
    let basename = match basename {
        None => "CapturedCam".to_owned(),
        Some(name) => name.to_owned(),
    };
    let caminfo = query(ApiBackend::MediaFoundation).map_err(|_| Cow::from("Failed to query cameras!"))?;
    let desired_res = Resolution::new(width.map(|w| w.get()).unwrap_or(1024), height.map(|h| h.get()).unwrap_or(576));
    let mut cameras = Vec::with_capacity(caminfo.len());

    #[cfg(feature = "to_pub")]
    let camfile = crate::util::find_camfile();

    for (index, info) in caminfo.iter().enumerate() {
        let mut nextcam = CallbackCamera::new(
            info.index().clone(),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(CameraFormat::new(
                desired_res,
                nokhwa::utils::FrameFormat::YUYV,
                15,
            ))),
            drop,
        )
        .map_err(|_| Cow::from(format!("Couldn't open camera {index} : {info}!")))?;
        let res = nextcam
            .resolution()
            .map_err(|_| eprintln!("Couldn't acquire a resolution for {index} : {info}!"));
        if res.is_err() {
            continue;
        }
        let res = res.unwrap();
        let mmf = MemoryMappedFile::<RWLock>::new(
            NonZeroUsize::new((3 * res.height_y * res.width_x) as usize)
                .ok_or(Cow::from("Someone managed tp get a resolution of zero ..."))?,
            format!("shmemcam_{basename}_{index}"),
            Namespace::GLOBAL,
        )
        .map_err(|_| eprintln!("Failed opening one of the MMFs!"));
        if mmf.is_err() {
            continue;
        }
        let mmf = mmf.unwrap();
        nextcam
            .set_callback(move |buff| {
                if let Err(wrote) = mmf.write_spin(
                    nokhwa::utils::yuyv422_to_rgb(buff.buffer(), false).unwrap(),
                    None::<fn(&dyn MMFLock, _) -> _>,
                ) {
                    eprintln!("{wrote}")
                }
            })
            .unwrap_or_else(|e| eprintln!("Couldn't set up the MMF callback for {index} : {info}\n{e}"));
        if nextcam.open_stream().is_err() {
            eprintln!("Failed to open camera {index} : {info}!");
            continue;
        }
        cameras.push(Mutex::new(nextcam));
        #[cfg(feature = "to_pub")]
        crate::util::write_camfile(format!("{basename}_{index}"), camfile.as_ref());
    }

    if cameras.is_empty() {
        return Err(Cow::from("Couldn't open any cameras, service going down!"));
    }

    // If anything goes wrong at this point, panic and bail.
    if let Ok(_) = CAMERAS.set(cameras.into()) {
        lock.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).unwrap();
        Ok(())
    } else {
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
