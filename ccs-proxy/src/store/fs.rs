use super::{RequestSummary, SessionMeta, Store, StoreError};
use crate::capture::CaptureRecord;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tokio::fs;

pub struct FsStore {
    root: PathBuf,
    write_failures: Mutex<u32>,
}

impl FsStore {
    pub fn open(root: PathBuf) -> Result<Self, StoreError> {
        std::fs::create_dir_all(root.join("sessions"))?;
        std::fs::create_dir_all(root.join("logs"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) =
                std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            {
                tracing::warn!(?root, ?e, "failed to set root data dir permissions to 0700");
            }
        }
        Ok(Self {
            root,
            write_failures: Mutex::new(0),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn consecutive_write_failures(&self) -> u32 {
        *self.write_failures.lock().unwrap()
    }

    fn session_dir(&self, sid: &str) -> PathBuf {
        self.root.join("sessions").join(sid)
    }

    fn meta_path(&self, sid: &str) -> PathBuf {
        self.session_dir(sid).join("meta.json")
    }

    fn record_path(&self, sid: &str, seq: u64) -> PathBuf {
        self.session_dir(sid).join(format!("{seq:04}.json"))
    }

    async fn atomic_write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        let staging_path = path.with_extension("tmp");
        fs::write(&staging_path, bytes).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(
                &staging_path,
                std::fs::Permissions::from_mode(0o600),
            )
            .await;
        }
        fs::rename(&staging_path, path).await?;
        Ok(())
    }

    fn note_write_failure(&self) -> u32 {
        let mut guard = self.write_failures.lock().unwrap();
        *guard = guard.saturating_add(1);
        *guard
    }

    fn note_write_success(&self) {
        let mut guard = self.write_failures.lock().unwrap();
        *guard = 0;
    }

    async fn bump_request_count(&self, session_id: &str, seq: u64) {
        let meta_path = self.meta_path(session_id);
        let Ok(meta_bytes) = fs::read(&meta_path).await else {
            return;
        };
        let Ok(mut meta) = serde_json::from_slice::<SessionMeta>(&meta_bytes) else {
            return;
        };
        meta.request_count = meta.request_count.max(seq);
        let Ok(out) = serde_json::to_vec_pretty(&meta) else {
            return;
        };
        let _ = self.atomic_write(&meta_path, &out).await;
    }
}

#[async_trait]
impl Store for FsStore {
    async fn init_session(&self, meta: SessionMeta) -> Result<(), StoreError> {
        let dir = self.session_dir(&meta.session_id);
        fs::create_dir_all(&dir).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).await;
        }
        let bytes = serde_json::to_vec_pretty(&meta)?;
        match self
            .atomic_write(&self.meta_path(&meta.session_id), &bytes)
            .await
        {
            Ok(()) => {
                self.note_write_success();
                Ok(())
            }
            Err(err) => {
                self.note_write_failure();
                Err(err)
            }
        }
    }

    async fn finalize_session(&self, session_id: &str) -> Result<(), StoreError> {
        let path = self.meta_path(session_id);
        let Ok(bytes) = fs::read(&path).await else {
            return Ok(());
        };
        let mut meta: SessionMeta = serde_json::from_slice(&bytes)?;
        meta.ended_at = Some(chrono::Utc::now());
        let out = serde_json::to_vec_pretty(&meta)?;
        self.atomic_write(&path, &out).await?;
        Ok(())
    }

    async fn append(&self, rec: CaptureRecord) -> Result<(), StoreError> {
        let dir = self.session_dir(&rec.session_id);
        fs::create_dir_all(&dir).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).await;
        }
        let path = self.record_path(&rec.session_id, rec.seq);
        let bytes = serde_json::to_vec_pretty(&rec)?;
        match self.atomic_write(&path, &bytes).await {
            Ok(()) => {
                self.note_write_success();
                self.bump_request_count(&rec.session_id, rec.seq).await;
                Ok(())
            }
            Err(err) => {
                self.note_write_failure();
                Err(err)
            }
        }
    }

    async fn list_sessions(&self) -> Result<Vec<SessionMeta>, StoreError> {
        let dir = self.root.join("sessions");
        let mut out = Vec::new();
        let Ok(mut rd) = fs::read_dir(&dir).await else {
            return Ok(out);
        };
        while let Some(entry) = rd.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let meta_path = entry.path().join("meta.json");
            let Ok(bytes) = fs::read(&meta_path).await else {
                continue;
            };
            match serde_json::from_slice::<SessionMeta>(&bytes) {
                Ok(meta) => out.push(meta),
                Err(err) => {
                    tracing::warn!(path = ?meta_path, error = ?err, "skipping corrupt meta.json");
                }
            }
        }
        out.sort_by_key(|meta| std::cmp::Reverse(meta.started_at));
        Ok(out)
    }

    async fn list_requests(
        &self,
        session_id: &str,
    ) -> Result<Vec<RequestSummary>, StoreError> {
        let dir = self.session_dir(session_id);
        let mut out = Vec::new();
        let Ok(mut rd) = fs::read_dir(&dir).await else {
            return Ok(out);
        };
        while let Some(entry) = rd.next_entry().await? {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "meta.json" || !name_str.ends_with(".json") {
                continue;
            }
            let Ok(bytes) = fs::read(entry.path()).await else {
                continue;
            };
            let Ok(rec) = serde_json::from_slice::<CaptureRecord>(&bytes) else {
                tracing::warn!(path = ?entry.path(), "skipping unparseable record");
                continue;
            };
            out.push(summary_of(&rec));
        }
        out.sort_by_key(|summary| summary.seq);
        Ok(out)
    }

    async fn get_request(
        &self,
        session_id: &str,
        seq: u64,
    ) -> Result<Option<CaptureRecord>, StoreError> {
        let path = self.record_path(session_id, seq);
        match fs::read(&path).await {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}

fn summary_of(rec: &CaptureRecord) -> RequestSummary {
    RequestSummary {
        seq: rec.seq,
        session_id: rec.session_id.clone(),
        request_id: rec.request_id.clone(),
        started_at: rec.started_at,
        duration_ms: rec.duration_ms,
        model: rec.model.clone(),
        status: rec.response.as_ref().map(|resp| resp.status),
        input_tokens: rec.usage.as_ref().map(|usage| usage.input_tokens),
        output_tokens: rec.usage.as_ref().map(|usage| usage.output_tokens),
        has_error: rec.error.is_some(),
    }
}
