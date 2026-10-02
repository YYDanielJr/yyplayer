//! Bounded image decoding for user backgrounds. The UI only receives reduced RGBA images.
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, SyncSender, sync_channel},
};

type Pixels = (u32, u32, Vec<u8>);
type Request = (usize, u64, PathBuf, u8);
type Reply = (usize, u64, Result<Pixels, String>);

pub(super) struct Backgrounds {
    commands: SyncSender<Request>,
    replies: Receiver<Reply>,
    worker: Option<std::thread::JoinHandle<()>>,
    pending: [Option<Request>; 2],
    latest: [u64; 2],
    paths: [Option<PathBuf>; 2],
    images: [slint::Image; 2],
    revision: u64,
}

impl Backgrounds {
    pub fn start() -> Self {
        let (commands, rx) = sync_channel::<Request>(2);
        let (tx, replies) = sync_channel::<Reply>(2);
        let worker = std::thread::spawn(move || {
            while let Ok((kind, revision, path, blur)) = rx.recv() {
                if tx.send((kind, revision, decode(&path, blur))).is_err() {
                    break;
                }
            }
        });
        Self {
            commands,
            replies,
            worker: Some(worker),
            pending: [None, None],
            latest: [0, 0],
            paths: [None, None],
            images: Default::default(),
            revision: 0,
        }
    }

    pub fn request(&mut self, kind: usize, path: PathBuf, blur: u8) {
        self.latest[kind] += 1;
        if self.paths[kind].as_ref() != Some(&path) {
            self.images[kind] = slint::Image::default();
            self.paths[kind] = Some(path.clone());
            self.revision += 1;
        }
        self.pending[kind] = Some((kind, self.latest[kind], path, blur));
        self.flush();
    }

    fn flush(&mut self) {
        for slot in &mut self.pending {
            if let Some(request) = slot.take()
                && let Err(std::sync::mpsc::TrySendError::Full(request)) =
                    self.commands.try_send(request)
            {
                *slot = Some(request);
            }
        }
    }

    pub fn poll(&mut self) -> Vec<(usize, Result<(), String>)> {
        let mut notices = Vec::new();
        for (kind, revision, result) in self.replies.try_iter() {
            if revision != self.latest[kind] {
                continue;
            }
            match result {
                Ok((width, height, pixels)) => {
                    self.images[kind] = slint::Image::from_rgba8(slint::SharedPixelBuffer::<
                        slint::Rgba8Pixel,
                    >::clone_from_slice(
                        &pixels, width, height
                    ));
                    self.revision += 1;
                    notices.push((kind, Ok(())));
                }
                Err(error) => notices.push((kind, Err(error))),
            }
        }
        self.flush();
        notices
    }

    pub fn image(&self, kind: usize) -> slint::Image {
        self.images[kind].clone()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn finish(mut self) {
        drop(self.commands);
        drop(self.replies);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn decode(path: &PathBuf, blur: u8) -> Result<Pixels, String> {
    let metadata = std::fs::metadata(path).map_err(|e| format!("背景图片无法读取：{e}"))?;
    if metadata.len() > 32 * 1024 * 1024 {
        return Err("背景图片超过 32 MB".into());
    }
    let (width, height) =
        image::image_dimensions(path).map_err(|e| format!("背景图片格式无效：{e}"))?;
    if width == 0 || height == 0 || (width as u64 * height as u64) > 25_000_000 {
        return Err("背景图片尺寸超出 2500 万像素限制".into());
    }
    let decoded = image::open(path).map_err(|e| format!("背景图片解码失败：{e}"))?;
    let reduced = if width > 1920 || height > 1080 {
        decoded.thumbnail(1920, 1080).into_rgba8()
    } else {
        decoded.into_rgba8()
    };
    let output = if blur == 0 {
        reduced
    } else {
        image::imageops::blur(&reduced, blur as f32)
    };
    Ok((output.width(), output.height(), output.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_background_decode_and_blur() {
        let path =
            std::env::temp_dir().join(format!("yyplayer-background-{}.png", std::process::id()));
        let mut image = image::RgbaImage::from_pixel(32, 32, image::Rgba([0, 0, 0, 255]));
        image.put_pixel(16, 16, image::Rgba([255, 255, 255, 255]));
        image.save(&path).unwrap();
        let (width, height, plain) = decode(&path, 0).unwrap();
        let (_, _, blurred) = decode(&path, 8).unwrap();
        assert_eq!((width, height), (32, 32));
        assert!(plain[(16 * 32 + 16) * 4] > blurred[(16 * 32 + 16) * 4]);
        let mut service = Backgrounds::start();
        service.request(0, path.clone(), 0);
        service.request(0, path.clone(), 8);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut accepted = 0;
        while accepted == 0 && std::time::Instant::now() < deadline {
            accepted += service.poll().len();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(accepted, 1, "only the newest blur reply may reach the UI");
        assert_eq!(service.image(0).size().width, 32);
        service.finish();
        std::fs::remove_file(&path).unwrap();
        assert!(decode(&path, 0).is_err());
    }
}
