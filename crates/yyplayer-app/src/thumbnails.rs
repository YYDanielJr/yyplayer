//! Visible-row thumbnail requests, bounded CPU cache and worker queues.
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
};
type Pixels = (u32, u32, Vec<u8>);
type Reply = (u64, PathBuf, Option<Pixels>);
pub(super) struct Thumbnails {
    commands: SyncSender<(u64, PathBuf)>,
    replies: Receiver<Reply>,
    worker: Option<std::thread::JoinHandle<()>>,
    generation: Arc<AtomicU64>,
    epoch: u64,
    cache: HashMap<PathBuf, Option<slint::Image>>,
    order: VecDeque<PathBuf>,
    pending: HashSet<PathBuf>,
}

impl Thumbnails {
    pub fn start() -> Self {
        let (commands, rx) = sync_channel::<(u64, PathBuf)>(8);
        let (tx, replies) = sync_channel(8);
        let generation = Arc::new(AtomicU64::new(0));
        let cancel = generation.clone();
        let worker = std::thread::spawn(move || {
            lofty::config::apply_global_options(
                lofty::config::GlobalOptions::new().allocation_limit(16 * 1024 * 1024),
            );
            while let Ok((epoch, path)) = rx.recv() {
                if cancel.load(Ordering::Relaxed) != epoch {
                    continue;
                }
                let pixels = super::music_assets::read_thumbnail(&path);
                if cancel.load(Ordering::Relaxed) == epoch
                    && tx.send((epoch, path, pixels)).is_err()
                {
                    break;
                }
            }
        });
        Self {
            commands,
            replies,
            worker: Some(worker),
            generation,
            epoch: 0,
            cache: HashMap::new(),
            order: VecDeque::new(),
            pending: HashSet::new(),
        }
    }
    pub fn invalidate(&mut self) {
        self.epoch += 1;
        self.generation.store(self.epoch, Ordering::Relaxed);
        self.cache.clear();
        self.order.clear();
        self.pending.clear();
    }
    pub fn request(&mut self, path: &Path) {
        if self.cache.contains_key(path) {
            self.order.retain(|p| p != path);
            self.order.push_back(path.into());
            return;
        }
        if !self.pending.contains(path)
            && self.pending.len() < 8
            && self.commands.try_send((self.epoch, path.into())).is_ok()
        {
            self.pending.insert(path.into());
        }
    }
    pub fn image(&self, path: &Path) -> slint::Image {
        self.cache
            .get(path)
            .and_then(|i| i.clone())
            .unwrap_or_default()
    }
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        for (epoch, path, pixels) in self.replies.try_iter() {
            if epoch != self.epoch {
                continue;
            }
            self.pending.remove(&path);
            let image = pixels.map(|(w, h, bytes)| {
                slint::Image::from_rgba8(
                    slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&bytes, w, h),
                )
            });
            self.cache.insert(path.clone(), image);
            self.order.push_back(path);
            changed = true;
            while self.order.len() > 256 {
                if let Some(old) = self.order.pop_front() {
                    self.cache.remove(&old);
                }
            }
        }
        changed
    }
    pub fn finish(mut self) {
        self.generation.store(u64::MAX, Ordering::Relaxed);
        drop(self.commands);
        drop(self.replies);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_art_cache_is_bounded_and_stale_replies_are_discarded() {
        let mut service = Thumbnails::start();
        let directory = std::env::temp_dir().join(format!("yy-missing-art-{}", std::process::id()));
        let wait = |service: &mut Thumbnails| {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !service.pending.is_empty() {
                service.poll();
                assert!(
                    std::time::Instant::now() < deadline,
                    "thumbnail worker stalled"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        };
        for i in 0..270 {
            service.request(&directory.join(format!("{i}.missing")));
            wait(&mut service);
        }
        assert_eq!(service.cache.len(), 256);
        assert!(!service.cache.contains_key(&directory.join("0.missing")));
        let cached = directory.join("269.missing");
        service.request(&cached);
        assert!(
            service.pending.is_empty(),
            "missing artwork must also be cached"
        );
        for i in 0..20 {
            service.request(&directory.join(format!("stale-{i}.missing")));
        }
        assert_eq!(service.pending.len(), 8);
        service.invalidate();
        let fresh = directory.join("fresh.missing");
        service.request(&fresh);
        wait(&mut service);
        assert_eq!(service.cache.len(), 1);
        assert!(service.cache.contains_key(&fresh));
        service.finish();
    }
}
