use std::collections::BTreeMap;
use std::io::{self, Cursor, Read};
use std::ops::ControlFlow;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use windows_projfs::{
    DirectoryEntry, DirectoryInfo, FileInfo, Notification, ProjectedFileSystem,
    ProjectedFileSystemSource,
};

#[derive(Debug, Clone)]
pub struct ProjectedRepoSnapshot {
    directories: Arc<BTreeMap<PathBuf, Vec<DirectoryEntry>>>,
    entries: Arc<BTreeMap<PathBuf, DirectoryEntry>>,
    files: Arc<BTreeMap<PathBuf, Arc<Vec<u8>>>>,
    file_count: usize,
    total_bytes: usize,
}

pub struct MountedProjectedRepo {
    pub projection: ProjectedFileSystem,
    pub file_count: usize,
    pub total_bytes: usize,
}

#[derive(Debug, Clone)]
struct InMemoryProjectionSource {
    snapshot: ProjectedRepoSnapshot,
}

impl ProjectedRepoSnapshot {
    pub fn from_files(files: Vec<(PathBuf, Vec<u8>)>) -> Result<Self, String> {
        let mut directories: BTreeMap<PathBuf, BTreeMap<String, DirectoryEntry>> = BTreeMap::new();
        let mut entries = BTreeMap::new();
        let mut payloads = BTreeMap::new();
        let mut file_count = 0_usize;
        let mut total_bytes = 0_usize;

        directories.entry(PathBuf::new()).or_default();

        for (relative_path, bytes) in files {
            let normalized = normalize_relative_path(&relative_path)?;
            if normalized.as_os_str().is_empty() {
                return Err("Snapshot do ProjFS recebeu arquivo com caminho vazio".to_string());
            }

            let file_name = normalized
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("Nome de arquivo inválido para ProjFS: '{}'", normalized.display()))?
                .to_string();
            let parent = normalized.parent().map(Path::to_path_buf).unwrap_or_default();

            ensure_parent_directories(&mut directories, &mut entries, &normalized);

            let file_info = FileInfo {
                file_name: file_name.clone(),
                file_size: bytes.len() as u64,
                ..Default::default()
            };
            let file_entry = DirectoryEntry::File(file_info);
            directories
                .entry(parent.clone())
                .or_default()
                .insert(file_name, file_entry.clone());
            entries.insert(normalized.clone(), file_entry);
            total_bytes += bytes.len();
            file_count += 1;
            payloads.insert(normalized, Arc::new(bytes));
        }

        let directory_vecs = directories
            .into_iter()
            .map(|(path, values)| (path, values.into_values().collect::<Vec<_>>()))
            .collect::<BTreeMap<_, _>>();

        Ok(Self {
            directories: Arc::new(directory_vecs),
            entries: Arc::new(entries),
            files: Arc::new(payloads),
            file_count,
            total_bytes,
        })
    }

    fn list_directory(&self, path: &Path) -> Vec<DirectoryEntry> {
        let normalized = normalize_lookup_path(path);
        self.directories.get(&normalized).cloned().unwrap_or_default()
    }

    fn get_entry(&self, path: &Path) -> Option<DirectoryEntry> {
        let normalized = normalize_lookup_path(path);
        if normalized.as_os_str().is_empty() {
            return Some(DirectoryEntry::Directory(DirectoryInfo::default()));
        }
        self.entries.get(&normalized).cloned()
    }

    fn open_file_slice(&self, path: &Path, byte_offset: usize, length: usize) -> io::Result<Box<dyn Read>> {
        let normalized = normalize_lookup_path(path);
        let bytes = self
            .files
            .get(&normalized)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Arquivo não existe no snapshot projetado"))?;

        if byte_offset > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Offset além do tamanho do arquivo projetado",
            ));
        }

        let end = bytes.len().min(byte_offset.saturating_add(length));
        Ok(Box::new(Cursor::new(bytes[byte_offset..end].to_vec())))
    }
}

impl ProjectedFileSystemSource for InMemoryProjectionSource {
    fn list_directory(&self, path: &Path) -> Vec<DirectoryEntry> {
        self.snapshot.list_directory(path)
    }

    fn get_directory_entry(&self, path: &Path) -> Option<DirectoryEntry> {
        self.snapshot.get_entry(path)
    }

    fn stream_file_content(
        &self,
        path: &Path,
        byte_offset: usize,
        length: usize,
    ) -> io::Result<Box<dyn Read>> {
        self.snapshot.open_file_slice(path, byte_offset, length)
    }

    fn handle_notification(&self, notification: &Notification) -> ControlFlow<()> {
        if notification.is_cancelable() && !matches!(notification, Notification::FilePreConvertToFull(_)) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

pub fn mount_projected_repo(repo_root: &Path, snapshot: ProjectedRepoSnapshot) -> Result<MountedProjectedRepo, String> {
    std::fs::create_dir_all(repo_root).map_err(|e| {
        format!(
            "Falha ao criar virtualization root do ProjFS '{}': {}",
            repo_root.display(),
            e
        )
    })?;

    let file_count = snapshot.file_count;
    let total_bytes = snapshot.total_bytes;
    let projection = ProjectedFileSystem::new(repo_root, InMemoryProjectionSource { snapshot })
        .map_err(|e| format!("Falha ao iniciar ProjectedFileSystem em '{}': {:?}", repo_root.display(), e))?;

    Ok(MountedProjectedRepo {
        projection,
        file_count,
        total_bytes,
    })
}

fn ensure_parent_directories(
    directories: &mut BTreeMap<PathBuf, BTreeMap<String, DirectoryEntry>>,
    entries: &mut BTreeMap<PathBuf, DirectoryEntry>,
    file_path: &Path,
) {
    let mut current = PathBuf::new();
    let mut parent = PathBuf::new();

    if let Some(dir) = file_path.parent() {
        for component in dir.components() {
            if let Component::Normal(name) = component {
                current.push(name);
                let directory_name = name.to_string_lossy().to_string();
                let entry = DirectoryEntry::Directory(DirectoryInfo {
                    directory_name: directory_name.clone(),
                    ..Default::default()
                });
                directories.entry(parent.clone()).or_default().insert(directory_name, entry.clone());
                directories.entry(current.clone()).or_default();
                entries.insert(current.clone(), entry);
                parent = current.clone();
            }
        }
    }
}

fn normalize_relative_path(path: &Path) -> Result<PathBuf, String> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                return Err(format!(
                    "Caminho incompatível com ProjFS: '{}'",
                    path.display()
                ));
            }
        }
    }
    Ok(normalized)
}

fn normalize_lookup_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if let Component::Normal(part) = component {
            normalized.push(part);
        }
    }
    normalized
}
