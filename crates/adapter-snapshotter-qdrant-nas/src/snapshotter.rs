//! `QdrantNasSnapshotter` — uses Qdrant's native snapshot API, pushes the
//! resulting file to a NAS path. v1 uses a local directory as the "NAS"
//! target; production mounts the NAS or copies via HTTPS/SCP — same shape,
//! different transport.

use librarian_domain::{AdapterIdentity, ConfigHash, SnapshotId, Snapshotter, StageVersion};
use reqwest::blocking::{multipart, Client};
use std::path::PathBuf;
use std::time::Duration;

use crate::error::SnapshotError;

pub struct QdrantNasSnapshotter {
    client: Client,
    qdrant_url: String,
    collection: String,
    nas_path: PathBuf,
}

impl QdrantNasSnapshotter {
    pub fn new(
        qdrant_url: impl Into<String>,
        collection: impl Into<String>,
        nas_path: impl Into<PathBuf>,
    ) -> Result<Self, SnapshotError> {
        let nas_path = nas_path.into();
        std::fs::create_dir_all(&nas_path).map_err(SnapshotError::Io)?;
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .map_err(|e| SnapshotError::Http(e.to_string()))?,
            qdrant_url: qdrant_url.into().trim_end_matches('/').to_string(),
            collection: collection.into(),
            nas_path,
        })
    }

    fn nas_file(&self, id: &SnapshotId) -> PathBuf {
        self.nas_path.join(&id.0)
    }

    fn http<R, F: FnOnce() -> reqwest::Result<R>>(label: &str, f: F) -> Result<R, SnapshotError> {
        f().map_err(|e| SnapshotError::Http(format!("{label}: {e}")))
    }
}

impl AdapterIdentity for QdrantNasSnapshotter {
    fn name(&self) -> &str { "qdrant-nas-snapshotter" }
    fn version(&self) -> StageVersion { StageVersion("0.1.0".into()) }
    fn config_hash(&self) -> ConfigHash {
        ConfigHash(format!("c={};nas={}", self.collection, self.nas_path.display()))
    }
}

impl Snapshotter for QdrantNasSnapshotter {
    type Error = SnapshotError;

    fn snapshot(&self) -> Result<SnapshotId, Self::Error> {
        // 1. Ask Qdrant to create a snapshot.
        let create_url = format!(
            "{}/collections/{}/snapshots", self.qdrant_url, self.collection,
        );
        let create_resp = Self::http("create", || self.client.post(&create_url).send())?;
        if !create_resp.status().is_success() {
            return Err(SnapshotError::Qdrant(format!(
                "create snapshot http {}", create_resp.status()
            )));
        }
        let create_body: serde_json::Value = Self::http("create-body", || create_resp.json())?;
        let name = create_body["result"]["name"].as_str()
            .ok_or_else(|| SnapshotError::Qdrant("missing snapshot name in response".into()))?
            .to_string();
        let id = SnapshotId(format!("{}__{}", self.collection, name));

        // 2. Download the snapshot file from Qdrant and write to NAS.
        let download_url = format!(
            "{}/collections/{}/snapshots/{}", self.qdrant_url, self.collection, name,
        );
        let bytes = Self::http("download", || self.client.get(&download_url).send())?
            .error_for_status().map_err(|e| SnapshotError::Qdrant(e.to_string()))?;
        let body = Self::http("download-body", || bytes.bytes())?;
        std::fs::write(self.nas_file(&id), &body).map_err(SnapshotError::Io)?;

        // 3. Free the snapshot from Qdrant — NAS is the durable copy.
        let delete_url = format!(
            "{}/collections/{}/snapshots/{}", self.qdrant_url, self.collection, name,
        );
        let _ = self.client.delete(&delete_url).send();

        Ok(id)
    }

    fn restore(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        let path = self.nas_file(id);
        if !path.exists() { return Err(SnapshotError::NotFound(id.0.clone())); }
        let upload_url = format!(
            "{}/collections/{}/snapshots/upload?priority=snapshot",
            self.qdrant_url, self.collection,
        );
        let form = multipart::Form::new()
            .file("snapshot", &path)
            .map_err(SnapshotError::Io)?;
        let resp = Self::http("upload", || self.client.post(&upload_url).multipart(form).send())?;
        if !resp.status().is_success() {
            let s = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(SnapshotError::Qdrant(format!("restore http {s}: {body}")));
        }
        Ok(())
    }

    fn list(&self) -> Result<Vec<SnapshotId>, Self::Error> {
        let mut out = Vec::new();
        let prefix = format!("{}__", self.collection);
        for entry in std::fs::read_dir(&self.nas_path).map_err(SnapshotError::Io)? {
            let entry = entry.map_err(SnapshotError::Io)?;
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&prefix) {
                    out.push(SnapshotId(name.to_string()));
                }
            }
        }
        Ok(out)
    }

    /// Keep newest `keep_last` by mtime; delete the rest from the NAS.
    fn prune(&self, keep_last: usize) -> Result<(), Self::Error> {
        let mut snaps: Vec<(PathBuf, std::time::SystemTime)> = self
            .list()?
            .into_iter()
            .map(|id| {
                let p = self.nas_file(&id);
                let mtime = std::fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                (p, mtime)
            })
            .collect();
        snaps.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
        for (p, _) in snaps.into_iter().skip(keep_last) {
            let _ = std::fs::remove_file(&p);
        }
        Ok(())
    }
}
