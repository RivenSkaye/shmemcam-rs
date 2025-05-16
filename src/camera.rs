use std::{cell::OnceCell, num::NonZeroUsize, ops::Deref};

use nokhwa::{
    pixel_format::RgbFormat,
    query,
    utils::{
        ApiBackend, CameraFormat, CameraInfo, RequestedFormat, RequestedFormatType, Resolution,
    },
    Buffer, CallbackCamera,
};
use winmmf::{MemoryMappedFile, Mmf};

pub static CAMS_AND_MMFS: OnceCell<
    Vec<(CallbackCamera, MemoryMappedFile<winmmf::states::RWLock>)>,
> = OnceCell::new();

pub fn init_cams(basename: impl Deref<Target = str>) {
    let caminfo = query(ApiBackend::Auto).unwrap();
    let desired_res = Resolution::new(1024, 576);
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
        let mmf = MemoryMappedFile::new(
            NonZeroUsize::new((res.height_y * res.width_x) as usize).unwrap(),
            format!("{basename}_{index}"),
            winmmf::Namespace::GLOBAL,
        )
        .unwrap();
        nextcam
            .set_callback(|buff| mmf.write_spin(buff.buffer(), None).unwrap())
            .unwrap();
        cameras.push((nextcam, mmf));
    }
    //CAMS.set().unwrap();
}
