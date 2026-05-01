mod config;
mod infer;

use infer::{infer_profile, ScreenHint, ScreenHintMode};
use mtklogo::utils::{image, image::ImageIO, z_lib};
use mtklogo::{ColorMode, ContentType, FileInfo, LogoImage};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
struct WorkflowResult {
    output_path: String,
    timestamp: String,
    file_count: usize,
}

#[tauri::command]
async fn unpack_logo(input_path: String, screen_resolution: String) -> Result<WorkflowResult, String> {
    tauri::async_runtime::spawn_blocking(move || unpack_logo_impl(input_path, screen_resolution))
        .await
        .map_err(|e| e.to_string())?
}

fn unpack_logo_impl(input_path: String, screen_resolution: String) -> Result<WorkflowResult, String> {
    let input_path = PathBuf::from(input_path);
    let screen_hint = parse_screen_hint(&screen_resolution)?;
    let file = File::open(&input_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let image = LogoImage::read(&mut reader).map_err(|e| e.to_string())?;
    let inflated_sizes: Vec<u32> = image
        .blobs
        .iter()
        .map(|blob| z_lib::inflate(blob.as_slice()).map(|inflated| inflated.len() as u32))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let profile = infer_profile(&inflated_sizes, Some(screen_hint), ScreenHintMode::Prefer)
        .ok_or_else(|| String::from("could not infer a profile from the selected logo"))?
        .to_profile("auto");

    let parent = input_path
        .parent()
        .ok_or_else(|| String::from("selected file has no parent directory"))?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("mtklogo");
    let timestamp = timestamp();
    let output_dir = parent.join(format!("{}_{}", stem, timestamp));
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    for (id, blob) in image.blobs.iter().enumerate() {
        let inflated = z_lib::inflate(blob.as_slice()).map_err(|e| e.to_string())?;
        let format = profile.guess_format(inflated.len() as u32, false).map_err(|e| e.to_string())?;
        let color_mode = ColorMode::by_name(&profile.color_model).map_err(|e| e.to_string())?;
        let info = FileInfo::from_info(id, false, color_mode);
        let output_file = output_dir.join(info.filename());
        let file = File::create(&output_file).map_err(|e| e.to_string())?;
        let writer = BufWriter::new(file);
        if let Err(_error) = color_mode.write_png(writer, &inflated, format.w, format.h) {
            let fallback = output_dir.join(FileInfo::from_info(id, true, color_mode).filename());
            let mut raw = File::create(fallback).map_err(|e| e.to_string())?;
            raw.write_all(blob).map_err(|e| e.to_string())?;
        }
    }

    Ok(WorkflowResult {
        output_path: output_dir.display().to_string(),
        timestamp,
        file_count: image.blobs.len(),
    })
}

#[tauri::command]
async fn repack_logo(source_dir: String, strip_alpha: bool) -> Result<WorkflowResult, String> {
    tauri::async_runtime::spawn_blocking(move || repack_logo_impl(source_dir, strip_alpha))
        .await
        .map_err(|e| e.to_string())?
}

fn repack_logo_impl(source_dir: String, strip_alpha: bool) -> Result<WorkflowResult, String> {
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
        return Err(String::from("no logo files found in the selected directory"));
    }

    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut blobs = Vec::with_capacity(files.len());
    for (_, path, info) in files {
        blobs.push(import_logo(&path, &info, strip_alpha)?);
    }

    let output_file = repack_output_path(&source_dir)?;
    let image = LogoImage::new_blobs(blobs);
    let mut writer = BufWriter::new(File::create(&output_file).map_err(|e| e.to_string())?);
    image.write(&mut writer).map_err(|e| e.to_string())?;

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
            BufReader::new(file).read_to_end(&mut raw).map_err(|e| e.to_string())?;
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
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| String::from("0"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            unpack_logo,
            repack_logo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running mtklogo GUI");
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

        let result = unpack_logo_impl(bin_path.to_string_lossy().into_owned(), "45x56".to_string()).unwrap();
        assert!(Path::new(&result.output_path).is_dir());
        assert_eq!(result.file_count, 1);
    }

    #[test]
    fn repack_sorts_files_by_slot_index() {
        let dir = std::env::temp_dir().join(format!("mtklogo-gui-test-{}", timestamp()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("logo_010_raw.z"), [10_u8]).unwrap();
        fs::write(dir.join("logo_000_raw.z"), [0_u8]).unwrap();

        let output = repack_logo_impl(dir.to_string_lossy().into_owned(), false).unwrap();
        let mut reader = Cursor::new(fs::read(output.output_path).unwrap());
        let image = LogoImage::read(&mut reader).unwrap();
        assert_eq!(image.blobs, vec![vec![0], vec![10]]);
    }
}
