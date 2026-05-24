use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Arc, Mutex};

use eframe::egui;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const FETCH_SIZE_PX: u32 = 64;
const AVATAR_WORKER_COUNT: usize = 4;
const AVATAR_QUEUE_CAPACITY: usize = 32;

#[derive(Default)]
pub struct AvatarCache {
    entries: HashMap<String, CacheEntry>,
    pending_jobs: VecDeque<FetchJob>,
    job_tx: Option<mpsc::SyncSender<FetchJob>>,
    result_rx: Option<mpsc::Receiver<AvatarResult>>,
}

enum CacheEntry {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

struct AvatarResult {
    url: String,
    image: Result<egui::ColorImage, String>,
}

struct FetchJob {
    url: String,
    repaint_ctx: egui::Context,
}

impl AvatarCache {
    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(result_rx) = &self.result_rx else {
            return;
        };

        while let Ok(result) = result_rx.try_recv() {
            let entry = match result.image {
                Ok(image) => CacheEntry::Ready(ctx.load_texture(
                    format!("author-avatar:{}", result.url),
                    image,
                    egui::TextureOptions::LINEAR,
                )),
                Err(_) => CacheEntry::Failed,
            };
            self.entries.insert(result.url, entry);
        }
        self.flush_pending_jobs();
    }

    pub(super) fn texture(
        &mut self,
        ctx: &egui::Context,
        url: &str,
    ) -> Option<egui::TextureHandle> {
        if !self.entries.contains_key(url) {
            self.start_fetch(ctx.clone(), url.to_owned());
        }
        self.flush_pending_jobs();

        match self.entries.get(url) {
            Some(CacheEntry::Ready(texture)) => Some(texture.clone()),
            Some(CacheEntry::Loading | CacheEntry::Failed) | None => None,
        }
    }

    fn start_fetch(&mut self, ctx: egui::Context, url: String) {
        self.entries.insert(url.clone(), CacheEntry::Loading);
        self.enqueue_job(FetchJob {
            url,
            repaint_ctx: ctx,
        });
    }

    fn enqueue_job(&mut self, job: FetchJob) {
        let send_result = self.job_sender().try_send(job);
        if let Err(error) = send_result {
            match error {
                mpsc::TrySendError::Full(job) | mpsc::TrySendError::Disconnected(job) => {
                    self.pending_jobs.push_back(job);
                }
            }
        }
    }

    fn flush_pending_jobs(&mut self) {
        while let Some(job) = self.pending_jobs.pop_front() {
            let send_result = self.job_sender().try_send(job);
            if let Err(error) = send_result {
                match error {
                    mpsc::TrySendError::Full(job) | mpsc::TrySendError::Disconnected(job) => {
                        self.pending_jobs.push_front(job);
                        break;
                    }
                }
            }
        }
    }

    fn job_sender(&mut self) -> &mpsc::SyncSender<FetchJob> {
        if self.job_tx.is_none() || self.result_rx.is_none() {
            let (job_tx, job_rx) = mpsc::sync_channel::<FetchJob>(AVATAR_QUEUE_CAPACITY);
            let shared_job_rx = Arc::new(Mutex::new(job_rx));
            let (result_tx, result_rx) = mpsc::channel();

            for _ in 0..AVATAR_WORKER_COUNT {
                let worker_job_rx = Arc::clone(&shared_job_rx);
                let worker_result_tx = result_tx.clone();
                std::thread::spawn(move || loop {
                    let job = {
                        let receiver = worker_job_rx
                            .lock()
                            .expect("avatar job receiver lock poisoned");
                        match receiver.recv() {
                            Ok(job) => job,
                            Err(_) => break,
                        }
                    };

                    let image = download_avatar(&job.url);
                    let _ = worker_result_tx.send(AvatarResult {
                        url: job.url,
                        image,
                    });
                    job.repaint_ctx.request_repaint();
                });
            }

            self.job_tx = Some(job_tx);
            self.result_rx = Some(result_rx);
        }

        self.job_tx.as_ref().expect("avatar job sender initialized")
    }
}

fn download_avatar(url: &str) -> Result<egui::ColorImage, String> {
    let mut response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "avatar request failed with status {}",
            status.as_u16()
        ));
    }
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|error| error.to_string())?;
    decode_avatar(&bytes)
}

#[cfg(test)]
pub(super) fn decode_avatar(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    decode_avatar_impl(bytes)
}

#[cfg(not(test))]
fn decode_avatar(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    decode_avatar_impl(bytes)
}

fn decode_avatar_impl(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let image = image::load_from_memory(bytes).map_err(|error| error.to_string())?;
    let rgba = image.thumbnail(FETCH_SIZE_PX, FETCH_SIZE_PX).to_rgba8();
    let [width, height] = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [width, height],
        &pixels,
    ))
}
