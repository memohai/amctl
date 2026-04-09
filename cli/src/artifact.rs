use crate::memory::{ArtifactRecord, MemoryStore};
use crate::output::CommandError;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ArtifactManager<'a> {
    pub store: Option<&'a MemoryStore>,
    pub session: &'a str,
    pub invocation_id: &'a str,
    pub artifact_dir: &'a Path,
    pub screen_file: Option<&'a Path>,
    pub screenshot_file: Option<&'a Path>,
    pub page_dir: &'a Path,
}

pub struct SaveRequest<'a> {
    pub kind: &'a str,
    pub category: &'a str,
    pub op: &'a str,
    pub mime_type: &'a str,
    pub extension: &'a str,
    pub bytes: &'a [u8],
    pub explicit_file: Option<&'a Path>,
    pub default_file: Option<&'a Path>,
    pub default_dir: &'a Path,
}

impl<'a> ArtifactManager<'a> {
    pub fn save(&self, request: SaveRequest<'_>) -> Result<serde_json::Value, CommandError> {
        let path = if let Some(explicit) = request.explicit_file {
            explicit.to_path_buf()
        } else if let Some(default_file) = request.default_file {
            default_file.to_path_buf()
        } else {
            request
                .default_dir
                .join(self.generated_name(request.kind, request.extension))
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CommandError::internal(format!(
                    "create artifact directory failed ({}): {e}",
                    parent.display()
                ))
            })?;
        }

        fs::write(&path, request.bytes).map_err(|e| {
            CommandError::internal(format!("write artifact failed ({}): {e}", path.display()))
        })?;

        let size_bytes = request.bytes.len() as i64;
        let content_hash = sha256_hex(request.bytes);
        let artifact_id = if let Some(store) = self.store {
            Some(
                store
                    .insert_artifact(&ArtifactRecord {
                        session: self.session,
                        trace_id: self.invocation_id,
                        category: request.category,
                        op: request.op,
                        kind: request.kind,
                        mime_type: request.mime_type,
                        file_path: &path,
                        size_bytes,
                        content_hash: &content_hash,
                    })
                    .map_err(|e| CommandError::internal(format!("record artifact failed: {e}")))?,
            )
        } else {
            None
        };

        Ok(json!({
            "artifactId": artifact_id,
            "kind": request.kind,
            "mimeType": request.mime_type,
            "savedFile": path.display().to_string(),
            "sizeBytes": size_bytes,
            "contentHash": content_hash,
        }))
    }

    fn generated_name(&self, kind: &str, extension: &str) -> String {
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let session = sanitize(self.session);
        let trace = sanitize(self.invocation_id);
        format!("{ts}-{session}-{trace}-{kind}.{extension}")
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn json_bytes(value: &serde_json::Value) -> Result<Vec<u8>, CommandError> {
    serde_json::to_vec_pretty(value)
        .map_err(|e| CommandError::internal(format!("serialize artifact json failed: {e}")))
}

pub fn subdir(base: &Path, name: &str) -> PathBuf {
    base.join(name)
}
