use std::{
    num::{NonZeroU32, NonZeroUsize},
    ops::Deref,
    sync::{atomic::AtomicBool, OnceLock},
};

use nokhwa::{
    pixel_format::RgbFormat,
    query,
    utils::{ApiBackend, CameraFormat, RequestedFormat, RequestedFormatType, Resolution},
    CallbackCamera,
};
use winmmf::{MMFLock, MemoryMappedFile, Mmf, RWLock};

pub static CAMERAS: OnceLock<Vec<CallbackCamera>> = OnceLock::new();
static CAMS_INITIZALIZED: AtomicBool = AtomicBool::new(false);

pub fn init_cams(basename: Option<impl Deref<Target = str>>, width: Option<NonZeroU32>, height: Option<NonZeroU32>) {
    if CAMS_INITIZALIZED.load(std::sync::atomic::Ordering::Acquire) {
        return;
    }
    let basename = match basename {
        None => "CapturedCam".to_owned(),
        Some(name) => name.to_owned(),
    };
    let caminfo = query(ApiBackend::MediaFoundation).unwrap();
    let desired_res = Resolution::new(width.map(|w| w.get()).unwrap_or(1024), height.map(|h| h.get()).unwrap_or(576));
    let mut cameras = Vec::with_capacity(caminfo.len());
    for (index, info) in caminfo.iter().enumerate() {
        let mut nextcam = CallbackCamera::new(
            info.index().clone(),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(CameraFormat::new(
                desired_res,
                nokhwa::utils::FrameFormat::RAWRGB,
                4,
            ))),
            drop,
        )
        .unwrap();
        let res = nextcam.resolution().unwrap();
        let mmf = MemoryMappedFile::<RWLock>::new(
            NonZeroUsize::new((res.height_y * res.width_x) as usize).unwrap(),
            format!("{basename}_{index}"),
            winmmf::Namespace::GLOBAL,
        )
        .unwrap();
        nextcam
            .set_callback(move |buff| mmf.write_spin(buff.buffer(), None::<fn(&dyn MMFLock, _) -> _>).unwrap())
            .unwrap();
        nextcam.open_stream().unwrap();
        cameras.push(nextcam);
    }

    if let Ok(()) = CAMERAS.set(cameras) {
        CAMS_INITIZALIZED
            .compare_exchange(false, true, std::sync::atomic::Ordering::AcqRel, std::sync::atomic::Ordering::Relaxed)
            .unwrap();
    } else {
        panic!("Couldn't set the OnceLock")
    }
}
