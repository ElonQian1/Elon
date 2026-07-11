use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

use super::model::{validate_identifier, FitRunDocument, FitTrial};

const MANIFEST_FILE: &str = "manifest.json";
const MANIFEST_BACKUP_FILE: &str = "manifest.json.bak";
const MANIFEST_TEMP_FILE: &str = "manifest.json.tmp";
const TRIALS_FILE: &str = "trials.jsonl";

#[derive(Debug, Clone, Default)]
pub(crate) struct FitRunStore;

impl FitRunStore {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn save(&self, run: &FitRunDocument) -> Result<()> {
        run.validate_loaded()?;
        let dir = run_dir(&run.project_root, &run.run_id)?;
        fs::create_dir_all(&dir)
            .with_context(|| format!("创建 FitRun 目录失败: {}", dir.display()))?;
        atomic_write_json(&dir, run)
    }

    pub(crate) fn load(&self, project_root: &str, run_id: &str) -> Result<FitRunDocument> {
        let dir = run_dir(project_root, run_id)?;
        let manifest = dir.join(MANIFEST_FILE);
        let backup = dir.join(MANIFEST_BACKUP_FILE);
        let mut run = read_json::<FitRunDocument>(&manifest)
            .or_else(|manifest_error| {
                read_json::<FitRunDocument>(&backup).map_err(|backup_error| {
                    anyhow!(
                        "FitRun manifest 和备份均不可读: manifest={manifest_error:#}; backup={backup_error:#}"
                    )
                })
            })?;
        run.validate_loaded()?;
        self.reconcile_trials(&mut run)?;
        Ok(run)
    }

    pub(crate) fn list_for_project(&self, project_root: &str) -> Result<Vec<FitRunDocument>> {
        let root = fit_runs_root(project_root)?;
        let Ok(entries) = fs::read_dir(&root) else {
            return Ok(Vec::new());
        };
        let mut runs = Vec::new();
        for entry in entries {
            let entry =
                entry.with_context(|| format!("读取 FitRun 列表失败: {}", root.display()))?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let run_id = entry
                .file_name()
                .to_str()
                .ok_or_else(|| anyhow!("FitRun 目录名不是 UTF-8"))?
                .to_string();
            runs.push(
                self.load(project_root, &run_id)
                    .with_context(|| format!("FitRun {run_id} 已损坏，不能在列表中静默跳过"))?,
            );
        }
        runs.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(runs)
    }

    pub(crate) fn read_trials(&self, run: &FitRunDocument) -> Result<Vec<FitTrial>> {
        let path = run_dir(&run.project_root, &run.run_id)?.join(TRIALS_FILE);
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&path)?;
        let mut trials = Vec::new();
        let mut cursor = 0_usize;
        let mut valid_len = 0_usize;
        while cursor < bytes.len() {
            let line_end = bytes[cursor..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map(|offset| cursor + offset + 1)
                .unwrap_or(bytes.len());
            let line = &bytes[cursor..line_end];
            if line.iter().all(u8::is_ascii_whitespace) {
                valid_len = line_end;
                cursor = line_end;
                continue;
            }
            match serde_json::from_slice::<FitTrial>(line) {
                Ok(trial) => {
                    trials.push(trial);
                    valid_len = line_end;
                    cursor = line_end;
                }
                Err(_error) if bytes[line_end..].iter().all(u8::is_ascii_whitespace) => {
                    let file = OpenOptions::new().write(true).open(&path)?;
                    file.set_len(valid_len as u64)?;
                    file.sync_data()?;
                    break;
                }
                Err(error) => {
                    bail!("FitRun trial journal 中段损坏（offset {cursor}）: {error}");
                }
            }
        }
        Ok(trials)
    }

    pub(crate) fn append_trial(&self, run: &FitRunDocument, trial: &FitTrial) -> Result<()> {
        let dir = run_dir(&run.project_root, &run.run_id)?;
        fs::create_dir_all(&dir)?;
        let existing = self.read_trials(run)?;
        let trial = enrich_trial_baselines(run, &existing, trial);
        let path = dir.join(TRIALS_FILE);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("打开 FitRun trial journal 失败: {}", path.display()))?;
        serde_json::to_writer(&mut file, &trial)?;
        file.write_all(b"\n")?;
        file.sync_data()?;
        Ok(())
    }

    pub(crate) fn write_handoff_artifact(
        &self,
        run: &FitRunDocument,
        handoff_id: &str,
        payload: &serde_json::Value,
    ) -> Result<String> {
        validate_identifier(handoff_id, "handoffId")?;
        let dir = run_dir(&run.project_root, &run.run_id)?.join("handoffs");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{handoff_id}.json"));
        let bytes = serde_json::to_vec_pretty(payload)?;
        fs::write(&path, bytes)?;
        Ok(path.display().to_string())
    }

    fn reconcile_trials(&self, run: &mut FitRunDocument) -> Result<()> {
        for trial in self.read_trials(run)? {
            if trial.sequence > run.last_sequence {
                run.apply_checkpoint(trial.checkpoint, trial.sequence);
            }
        }
        Ok(())
    }
}

fn enrich_trial_baselines(
    run: &FitRunDocument,
    existing: &[FitTrial],
    trial: &FitTrial,
) -> FitTrial {
    let mut enriched = trial.clone();
    let mut baselines = earliest_before_values(existing);
    if let Ok(values) = crate::node_agent_android_live::ui_ir::persisted_node_property_values(
        &run.project_root,
        &run.session_id,
        &run.pair.runtime_node_id,
        &run.pair.definition_id,
        run.pair.instance_key.as_deref(),
    ) {
        for (property, value) in values {
            baselines.entry(property).or_insert(value);
        }
    }
    geometry_baselines(run, &mut baselines);
    let Some(candidate) = enriched.candidate.as_mut() else {
        return enriched;
    };
    for operation in &mut candidate.operations {
        let Some(object) = operation.as_object_mut() else {
            continue;
        };
        if object.contains_key("beforeValue") {
            continue;
        }
        let Some(property) = object.get("property").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if let Some(value) = baselines.get(property) {
            object.insert("beforeValue".to_string(), value.clone());
        }
    }
    enriched
}

fn earliest_before_values(trials: &[FitTrial]) -> BTreeMap<String, serde_json::Value> {
    let mut result = BTreeMap::new();
    for operation in trials
        .iter()
        .filter_map(|trial| trial.candidate.as_ref())
        .flat_map(|candidate| &candidate.operations)
    {
        let Some(property) = operation
            .get("property")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        if let Some(value) = operation.get("beforeValue") {
            result
                .entry(property.to_string())
                .or_insert_with(|| value.clone());
        }
    }
    result
}

fn geometry_baselines(run: &FitRunDocument, values: &mut BTreeMap<String, serde_json::Value>) {
    let density = f64::from(run.environment.density.unwrap_or(1.0).max(0.01));
    let width = f64::from(run.pair.current_rect.right - run.pair.current_rect.left) / density;
    let height = f64::from(run.pair.current_rect.bottom - run.pair.current_rect.top) / density;
    for (property, value, value_type) in [
        ("width", width, "dp"),
        ("height", height, "dp"),
        ("translationX", 0.0, "dp"),
        ("translationY", 0.0, "dp"),
        ("opacity", 1.0, "float"),
    ] {
        values
            .entry(property.to_string())
            .or_insert_with(|| serde_json::json!({ "type": value_type, "value": value }));
    }
}

fn fit_runs_root(project_root: &str) -> Result<PathBuf> {
    let root = PathBuf::from(project_root)
        .canonicalize()
        .context("FitRun 项目目录不存在")?;
    Ok(root.join(".elon").join("ui-tuner").join("fit-runs"))
}

fn run_dir(project_root: &str, run_id: &str) -> Result<PathBuf> {
    validate_identifier(run_id, "runId")?;
    Ok(fit_runs_root(project_root)?.join(run_id))
}

fn atomic_write_json(dir: &Path, value: &FitRunDocument) -> Result<()> {
    let manifest = dir.join(MANIFEST_FILE);
    let backup = dir.join(MANIFEST_BACKUP_FILE);
    let temporary = dir.join(MANIFEST_TEMP_FILE);
    let bytes = serde_json::to_vec_pretty(value)?;
    {
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if manifest.exists() {
        fs::rename(&manifest, &backup).with_context(|| {
            format!(
                "备份 FitRun manifest 失败: {} -> {}",
                manifest.display(),
                backup.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(&temporary, &manifest) {
        if backup.exists() && !manifest.exists() {
            let _ = fs::rename(&backup, &manifest);
        }
        bail!("替换 FitRun manifest 失败: {error}");
    }
    Ok(())
}

fn read_json<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("读取失败: {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("JSON 无效: {}", path.display()))
}
