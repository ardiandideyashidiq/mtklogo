mod config;
mod infer;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use infer::{ScreenHint, ScreenHintMode, infer_profile};
use mtklogo::utils::{image, image::ImageIO, z_lib};
use mtklogo::{ColorMode, ContentType, FileInfo, LogoImage};
use rayon::prelude::*;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize)]
struct WorkflowResult {
    output_path: String,
    timestamp: String,
    file_count: usize,
}

#[derive(Clone, Default)]
struct AppState {
    active_cancel: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

impl AppState {
    fn begin(&self) -> Result<Arc<AtomicBool>, String> {
        let mut active = self
            .active_cancel
            .lock()
            .map_err(|_| String::from("workflow state is unavailable"))?;
        if active.is_some() {
            return Err(String::from("another workflow is already running"));
        }

        let cancel = Arc::new(AtomicBool::new(false));
        *active = Some(cancel.clone());
        Ok(cancel)
    }

    fn cancel_current(&self) -> Result<bool, String> {
        let active = self
            .active_cancel
            .lock()
            .map_err(|_| String::from("workflow state is unavailable"))?;
        if let Some(flag) = active.as_ref() {
            flag.store(true, Ordering::SeqCst);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn finish(&self) {
        if let Ok(mut active) = self.active_cancel.lock() {
            *active = None;
        }
    }
}

struct ActiveWorkflowGuard {
    state: AppState,
}

impl ActiveWorkflowGuard {
    fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl Drop for ActiveWorkflowGuard {
    fn drop(&mut self) {
        self.state.finish();
    }
}

#[derive(Debug, Clone, Serialize)]
struct WorkflowProgress {
    workflow: &'static str,
    phase: &'static str,
    current: usize,
    total: usize,
    message: String,
    item: Option<String>,
}

#[derive(Debug)]
struct InflatedBlob {
    id: usize,
    inflated: Vec<u8>,
    blob: Vec<u8>,
}

#[tauri::command]
async fn unpack_logo(
    app: AppHandle,
    state: State<'_, AppState>,
    input_path: String,
    screen_resolution: String,
) -> Result<WorkflowResult, String> {
    let cancel = state.begin()?;
    let state = state.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _guard = ActiveWorkflowGuard::new(state);
        unpack_logo_impl(
            Some(&app),
            Some(cancel.as_ref()),
            input_path,
            screen_resolution,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

fn unpack_logo_impl(
    app: Option<&AppHandle>,
    cancel: Option<&AtomicBool>,
    input_path: String,
    screen_resolution: String,
) -> Result<WorkflowResult, String> {
    let app = app.cloned();
    let input_path = PathBuf::from(input_path);
    let screen_hint = parse_screen_hint(&screen_resolution)?;
    let file = File::open(&input_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let image = LogoImage::read(&mut reader).map_err(|e| e.to_string())?;
    let blob_count = image.blobs.len();
    let total_steps = blob_count.saturating_mul(2).saturating_add(2);
    let progress = Arc::new(AtomicUsize::new(0));

    emit_progress(
        app.as_ref(),
        WorkflowProgress {
            workflow: "unpack",
            phase: "preparing",
            current: 0,
            total: total_steps,
            message: String::from("Inspecting logo archive"),
            item: None,
        },
    );

    let inflated_blobs: Vec<InflatedBlob> = image
        .blobs
        .into_par_iter()
        .enumerate()
        .map(|(id, blob)| {
            check_cancel(cancel)?;
            let inflated = z_lib::inflate(blob.as_slice()).map_err(|e| e.to_string())?;
            let current = progress.fetch_add(1, Ordering::SeqCst) + 1;
            emit_progress(
                app.as_ref(),
                WorkflowProgress {
                    workflow: "unpack",
                    phase: "inflating",
                    current,
                    total: total_steps,
                    message: String::from("Inspecting compressed slots"),
                    item: Some(format!("slot {:03}", id)),
                },
            );
            Ok(InflatedBlob { id, inflated, blob })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let inflated_sizes: Vec<u32> = inflated_blobs
        .iter()
        .map(|item| item.inflated.len() as u32)
        .collect();

    let profile = infer_profile(&inflated_sizes, Some(screen_hint), ScreenHintMode::Prefer)
        .ok_or_else(|| String::from("could not infer a profile from the selected logo"))?
        .to_profile("auto");
    let color_mode = ColorMode::by_name(&profile.color_model)
        .map_err(|e| e.to_string())?
        .clone();

    let parent = input_path
        .parent()
        .ok_or_else(|| String::from("selected file has no parent directory"))?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("mtklogo");
    let timestamp = timestamp();
    let output_dir = parent.join(format!("{}_{}", stem, timestamp));
    let temp_output_dir = parent.join(format!(
        "{}.partial",
        output_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("mtklogo")
    ));
    remove_path_if_exists(&temp_output_dir)?;
    fs::create_dir_all(&temp_output_dir).map_err(|e| e.to_string())?;
    let mut temp_output_guard = TempArtifact::new(temp_output_dir.clone());

    inflated_blobs
        .into_par_iter()
        .map(|item| {
            check_cancel(cancel)?;
            let format = profile
                .guess_format(item.inflated.len() as u32, false)
                .map_err(|e| e.to_string())?;
            let info = FileInfo::from_info(item.id, false, &color_mode);
            let output_file = temp_output_dir.join(info.filename());
            let file = File::create(&output_file).map_err(|e| e.to_string())?;
            let writer = BufWriter::new(file);
            if let Err(_error) = color_mode.write_png(writer, &item.inflated, format.w, format.h) {
                let fallback = temp_output_dir
                    .join(FileInfo::from_info(item.id, true, &color_mode).filename());
                let mut raw = File::create(fallback).map_err(|e| e.to_string())?;
                raw.write_all(&item.blob).map_err(|e| e.to_string())?;
            }

            let current = progress.fetch_add(1, Ordering::SeqCst) + 1;
            emit_progress(
                app.as_ref(),
                WorkflowProgress {
                    workflow: "unpack",
                    phase: "writing",
                    current,
                    total: total_steps,
                    message: String::from("Writing extracted files"),
                    item: Some(format!("slot {:03}", item.id)),
                },
            );
            Ok(())
        })
        .collect::<Result<Vec<_>, String>>()?;

    check_cancel(cancel)?;
    emit_progress(
        app.as_ref(),
        WorkflowProgress {
            workflow: "unpack",
            phase: "finalizing",
            current: total_steps - 1,
            total: total_steps,
            message: String::from("Finalizing output folder"),
            item: None,
        },
    );

    remove_path_if_exists(&output_dir)?;
    fs::rename(&temp_output_dir, &output_dir).map_err(|e| e.to_string())?;
    temp_output_guard.commit();

    emit_progress(
        app.as_ref(),
        WorkflowProgress {
            workflow: "unpack",
            phase: "done",
            current: total_steps,
            total: total_steps,
            message: String::from("Extraction complete"),
            item: None,
        },
    );

    Ok(WorkflowResult {
        output_path: output_dir.display().to_string(),
        timestamp,
        file_count: blob_count,
    })
}

#[tauri::command]
async fn repack_logo(
    app: AppHandle,
    state: State<'_, AppState>,
    source_dir: String,
    strip_alpha: bool,
) -> Result<WorkflowResult, String> {
    let cancel = state.begin()?;
    let state = state.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _guard = ActiveWorkflowGuard::new(state);
        repack_logo_impl(Some(&app), Some(cancel.as_ref()), source_dir, strip_alpha)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn repack_logo_impl(
    app: Option<&AppHandle>,
    cancel: Option<&AtomicBool>,
    source_dir: String,
    strip_alpha: bool,
) -> Result<WorkflowResult, String> {
    let app = app.cloned();
    let source_dir = PathBuf::from(source_dir);
    if !source_dir.is_dir() {
        return Err(String::from("source directory does not exist"));
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&source_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("file '{}' is not a possible path.", path.display()))?;
        let info = FileInfo::from_name(name).map_err(|e| e.to_string())?;
        files.push((info.id, path, info));
    }

    if files.is_empty() {
        return Err(String::from(
            "no logo files found in the selected directory",
        ));
    }

    files.sort_by(|left, right| left.0.cmp(&right.0));
    let total_steps = files.len().saturating_add(2);
    let progress = Arc::new(AtomicUsize::new(0));

    emit_progress(
        app.as_ref(),
        WorkflowProgress {
            workflow: "repack",
            phase: "preparing",
            current: 0,
            total: total_steps,
            message: String::from("Reading selected files"),
            item: None,
        },
    );

    let blobs: Vec<Vec<u8>> = files
        .into_par_iter()
        .map(|(_, path, info)| {
            check_cancel(cancel)?;
            let blob = import_logo(&path, &info, strip_alpha)?;
            let current = progress.fetch_add(1, Ordering::SeqCst) + 1;
            emit_progress(
                app.as_ref(),
                WorkflowProgress {
                    workflow: "repack",
                    phase: "processing",
                    current,
                    total: total_steps,
                    message: String::from("Packing logo files"),
                    item: path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.to_string()),
                },
            );
            Ok(blob)
        })
        .collect::<Result<Vec<_>, String>>()?;

    let output_file = repack_output_path(&source_dir)?;
    let temp_output_file = output_file.with_extension("bin.partial");
    remove_path_if_exists(&temp_output_file)?;
    let mut temp_output_guard = TempArtifact::new(temp_output_file.clone());
    let image = LogoImage::new_blobs(blobs);
    let mut writer = BufWriter::new(File::create(&temp_output_file).map_err(|e| e.to_string())?);
    image.write(&mut writer).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;

    check_cancel(cancel)?;
    emit_progress(
        app.as_ref(),
        WorkflowProgress {
            workflow: "repack",
            phase: "finalizing",
            current: total_steps - 1,
            total: total_steps,
            message: String::from("Writing output .bin"),
            item: None,
        },
    );

    remove_path_if_exists(&output_file)?;
    fs::rename(&temp_output_file, &output_file).map_err(|e| e.to_string())?;
    temp_output_guard.commit();

    emit_progress(
        app.as_ref(),
        WorkflowProgress {
            workflow: "repack",
            phase: "done",
            current: total_steps,
            total: total_steps,
            message: String::from("Repack complete"),
            item: None,
        },
    );

    Ok(WorkflowResult {
        output_path: output_file.display().to_string(),
        timestamp: timestamp(),
        file_count: image.blobs.len(),
    })
}

fn import_logo(path: &Path, info: &FileInfo, strip_alpha: bool) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    match info.content_type {
        ContentType::Z => {
            let mut raw = Vec::new();
            BufReader::new(file)
                .read_to_end(&mut raw)
                .map_err(|e| e.to_string())?;
            Ok(raw)
        }
        ContentType::PNG(ref color_mode) => {
            let (mut rgba, w, h) = image::png_to_rgba(file).map_err(|e| e.to_string())?;
            if strip_alpha {
                image::strip_alpha(&mut rgba);
            }
            let device = color_mode
                .rgba_to_device(&rgba, w, h)
                .map_err(|e| e.to_string())?;
            z_lib::deflate(&device).map_err(|e| e.to_string())
        }
    }
}

fn repack_output_path(source_dir: &Path) -> Result<PathBuf, String> {
    let parent = source_dir
        .parent()
        .ok_or_else(|| String::from("selected directory has no parent"))?;
    let name = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| String::from("selected directory name is not valid unicode"))?;
    Ok(parent.join(format!("{}.bin", name)))
}

fn parse_screen_hint(screen: &str) -> Result<ScreenHint, String> {
    let tokens: Vec<&str> = screen.split('x').map(|token| token.trim()).collect();
    if tokens.len() != 2 || tokens.iter().any(|token| token.is_empty()) {
        return Err(String::from("screen must be WIDTHxHEIGHT"));
    }

    let width = tokens[0]
        .parse::<u32>()
        .map_err(|_| String::from("screen width must be an integer"))?;
    let height = tokens[1]
        .parse::<u32>()
        .map_err(|_| String::from("screen height must be an integer"))?;
    Ok(ScreenHint { width, height })
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| String::from("0"))
}

fn check_cancel(cancel: Option<&AtomicBool>) -> Result<(), String> {
    if cancel
        .map(|flag| flag.load(Ordering::SeqCst))
        .unwrap_or(false)
    {
        Err(String::from("operation cancelled"))
    } else {
        Ok(())
    }
}

fn remove_path_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}

fn remove_path_quietly(path: &Path) {
    if !path.exists() {
        return;
    }

    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

struct TempArtifact {
    path: PathBuf,
    committed: bool,
}

impl TempArtifact {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for TempArtifact {
    fn drop(&mut self) {
        if !self.committed {
            remove_path_quietly(&self.path);
        }
    }
}

fn emit_progress(app: Option<&AppHandle>, progress: WorkflowProgress) {
    if let Some(app) = app {
        let _ = app.emit("workflow-progress", progress);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            unpack_logo,
            repack_logo,
            cancel_current_workflow,
        ])
        .run(tauri::generate_context!())
        .expect("error while running mtklogo GUI");
}

#[tauri::command]
fn cancel_current_workflow(state: State<'_, AppState>) -> Result<(), String> {
    if state.cancel_current()? {
        Ok(())
    } else {
        Err(String::from("no workflow is running"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtklogo::LogoImage;
    use std::io::Cursor;

    #[test]
    fn timestamped_output_path_ends_with_bin() {
        let path = repack_output_path(Path::new("/tmp/logo_123")).unwrap();
        assert!(path.ends_with("logo_123.bin"));
    }

    #[test]
    fn unpack_impl_creates_output_directory() {
        let dir = std::env::temp_dir().join(format!("mtklogo-gui-test-{}", timestamp()));
        fs::create_dir_all(&dir).unwrap();

        let raw = vec![0x12_u8; 45 * 56 * 4];
        let compressed = z_lib::deflate(&raw).unwrap();
        let image = LogoImage::new_blobs(vec![compressed]);
        let bin_path = dir.join("logo.bin");
        let mut writer = BufWriter::new(File::create(&bin_path).unwrap());
        image.write(&mut writer).unwrap();
        writer.flush().unwrap();

        let result = unpack_logo_impl(
            None,
            None,
            bin_path.to_string_lossy().into_owned(),
            "45x56".to_string(),
        )
        .unwrap();
        assert!(Path::new(&result.output_path).is_dir());
        assert_eq!(result.file_count, 1);
    }

    #[test]
    fn repack_sorts_files_by_slot_index() {
        let dir = std::env::temp_dir().join(format!("mtklogo-gui-test-{}", timestamp()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("logo_010_raw.z"), [10_u8]).unwrap();
        fs::write(dir.join("logo_000_raw.z"), [0_u8]).unwrap();

        let output =
            repack_logo_impl(None, None, dir.to_string_lossy().into_owned(), false).unwrap();
        let mut reader = Cursor::new(fs::read(output.output_path).unwrap());
        let image = LogoImage::read(&mut reader).unwrap();
        assert_eq!(image.blobs, vec![vec![0], vec![10]]);
    }
}
