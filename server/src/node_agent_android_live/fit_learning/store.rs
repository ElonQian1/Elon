use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::case_builder::sanitize_case_for_storage;
use super::types::{
    FitCase, FitCaseDocument, FitPriorDocument, FIT_CASE_SCHEMA_VERSION, FIT_PRIOR_SCHEMA_VERSION,
};

const CASES_FILE: &str = "fit-cases.v1.json";
const PRIORS_FILE: &str = "fit-priors.v1.json";

fn store_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone)]
pub(crate) struct FitLearningStore {
    standards_dir: PathBuf,
}

impl FitLearningStore {
    pub(crate) fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let root = project_root.as_ref();
        if root.as_os_str().is_empty() {
            bail!("fit learning 需要项目目录");
        }
        Ok(Self {
            standards_dir: root.join(".elon").join("ui-standards"),
        })
    }

    pub(crate) fn cases_path(&self) -> PathBuf {
        self.standards_dir.join(CASES_FILE)
    }

    pub(crate) fn priors_path(&self) -> PathBuf {
        self.standards_dir.join(PRIORS_FILE)
    }

    pub(crate) fn record_case(&self, case: FitCase) -> Result<FitCaseDocument> {
        let _guard = store_lock()
            .lock()
            .map_err(|_| anyhow!("fit learning 存储锁已损坏"))?;
        let case = sanitize_case_for_storage(case);
        let mut document = self.load_cases_unlocked()?;
        if let Some(existing) = document
            .cases
            .iter_mut()
            .find(|existing| existing.case_id == case.case_id)
        {
            *existing = case;
        } else {
            document.cases.push(case);
        }
        document
            .cases
            .sort_by(|left, right| left.case_id.cmp(&right.case_id));
        document.updated_at = Utc::now().to_rfc3339();
        atomic_write_json(&self.cases_path(), &document)?;
        Ok(document)
    }

    pub(crate) fn load_cases(&self) -> Result<FitCaseDocument> {
        let _guard = store_lock()
            .lock()
            .map_err(|_| anyhow!("fit learning 存储锁已损坏"))?;
        self.load_cases_unlocked()
    }

    pub(crate) fn save_priors(&self, document: &FitPriorDocument) -> Result<()> {
        if document.schema_version != FIT_PRIOR_SCHEMA_VERSION {
            bail!("不支持的 fit prior schemaVersion");
        }
        let _guard = store_lock()
            .lock()
            .map_err(|_| anyhow!("fit learning 存储锁已损坏"))?;
        atomic_write_json(&self.priors_path(), document)
    }

    pub(crate) fn load_priors(&self) -> Result<FitPriorDocument> {
        let _guard = store_lock()
            .lock()
            .map_err(|_| anyhow!("fit learning 存储锁已损坏"))?;
        let path = self.priors_path();
        if !path.exists() && !backup_path(&path)?.exists() {
            return Ok(FitPriorDocument::default());
        }
        let document: FitPriorDocument = read_json_with_backup(&path)?;
        if document.schema_version != FIT_PRIOR_SCHEMA_VERSION {
            bail!("不支持的 fit prior schemaVersion");
        }
        Ok(document)
    }

    fn load_cases_unlocked(&self) -> Result<FitCaseDocument> {
        let path = self.cases_path();
        if !path.exists() && !backup_path(&path)?.exists() {
            return Ok(FitCaseDocument::default());
        }
        let document: FitCaseDocument = read_json_with_backup(&path)?;
        if document.schema_version != FIT_CASE_SCHEMA_VERSION {
            bail!("不支持的 fit case schemaVersion");
        }
        Ok(document)
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = File::open(path).with_context(|| format!("无法读取 {}", path.display()))?;
    serde_json::from_reader(BufReader::new(file))
        .with_context(|| format!("无法解析 {}", path.display()))
}

fn read_json_with_backup<T: DeserializeOwned + Serialize>(path: &Path) -> Result<T> {
    match read_json(path) {
        Ok(document) => Ok(document),
        Err(primary_error) => {
            let backup = backup_path(path)?;
            let document = read_json(&backup).with_context(|| {
                format!(
                    "主文件和固定备份均不可读: primary={primary_error:#}; backup={}",
                    backup.display()
                )
            })?;
            // 固定备份恢复后立即修复主文件，避免下一次写入把损坏文件
            // 覆盖掉最后一份可用备份。
            let bytes = serde_json::to_vec_pretty(&document)?;
            fs::write(path, bytes)
                .with_context(|| format!("从固定备份恢复 {} 失败", path.display()))?;
            Ok(document)
        }
    }
}

fn atomic_write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("fit learning 路径缺少父目录"))?;
    fs::create_dir_all(parent).with_context(|| format!("无法创建 {}", parent.display()))?;
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let temp = parent.join(format!(".fit-learning-{nonce}.tmp"));
    let backup = backup_path(path)?;
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .with_context(|| format!("无法创建 {}", temp.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);

    if !path.exists() {
        return fs::rename(&temp, path).with_context(|| format!("无法原子写入 {}", path.display()));
    }
    if backup.exists() {
        fs::remove_file(&backup)
            .with_context(|| format!("无法替换固定备份 {}", backup.display()))?;
    }
    fs::rename(path, &backup).with_context(|| format!("无法备份 {}", path.display()))?;
    match fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::rename(&backup, path);
            let _ = fs::remove_file(&temp);
            Err(error).with_context(|| format!("无法原子替换 {}", path.display()))
        }
    }
}

fn backup_path(path: &Path) -> Result<PathBuf> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("fit learning 路径缺少文件名"))?;
    Ok(path.with_file_name(format!("{file_name}.bak")))
}
