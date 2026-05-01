use std::io::Cursor;

use mtklogo::{ColorMode, ContentType, FileInfo, LogoImage};
use mtklogo::utils::{image, image::ImageIO, z_lib};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct LogoSlot {
    index: usize,
    raw_size: usize,
    inflated_size: Option<usize>,
    inflation_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct LogoInspection {
    slot_count: usize,
    header_size: u32,
    block_size: u32,
    slots: Vec<LogoSlot>,
    supported_modes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GuiFile {
    name: String,
    bytes: Vec<u8>,
}

#[tauri::command]
fn inspect_logo(bytes: Vec<u8>) -> Result<LogoInspection, String> {
    let mut reader = Cursor::new(bytes);
    let image = LogoImage::read(&mut reader).map_err(|e| e.to_string())?;

    let slots = image
        .blobs
        .iter()
        .enumerate()
        .map(|(index, blob)| match z_lib::inflate(blob.as_slice()) {
            Ok(inflated) => LogoSlot {
                index,
                raw_size: blob.len(),
                inflated_size: Some(inflated.len()),
                inflation_error: None,
            },
            Err(error) => LogoSlot {
                index,
                raw_size: blob.len(),
                inflated_size: None,
                inflation_error: Some(error.to_string()),
            },
        })
        .collect::<Vec<_>>();

    Ok(LogoInspection {
        slot_count: image.blobs.len(),
        header_size: image.table.header.size,
        block_size: image.table.block_size,
        slots,
        supported_modes: ColorMode::enumerate()
            .iter()
            .map(|mode| mode.to_string())
            .collect(),
    })
}

#[tauri::command]
fn repack_logo(files: Vec<GuiFile>, strip_alpha: bool) -> Result<Vec<u8>, String> {
    let mut blobs = Vec::with_capacity(files.len());

    for file in files {
        let info = FileInfo::from_name(&file.name).map_err(|e| e.to_string())?;
        let blob = match info.content_type {
            ContentType::Z => file.bytes,
            ContentType::PNG(ref color_mode) => {
                let (mut rgba, width, height) =
                    image::png_to_rgba(Cursor::new(file.bytes)).map_err(|e| e.to_string())?;
                if strip_alpha {
                    image::strip_alpha(&mut rgba);
                }
                let device = color_mode
                    .rgba_to_device(&rgba, width, height)
                    .map_err(|e| e.to_string())?;
                z_lib::deflate(&device).map_err(|e| e.to_string())?
            }
        };
        blobs.push((info.id, blob));
    }

    blobs.sort_by(|left, right| left.0.cmp(&right.0));
    let ordered_blobs = blobs.into_iter().map(|(_, blob)| blob).collect();
    let image = LogoImage::new_blobs(ordered_blobs);
    let mut buffer = Vec::new();
    image.write(&mut buffer).map_err(|e| e.to_string())?;
    Ok(buffer)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![inspect_logo, repack_logo])
        .run(tauri::generate_context!())
        .expect("error while running mtklogo GUI");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_logo_reports_slots() {
        let raw = vec![1_u8, 2, 3, 4];
        let compressed = z_lib::deflate(&raw).unwrap();
        let image = LogoImage::new_blobs(vec![compressed]);
        let mut buffer = Vec::new();
        image.write(&mut buffer).unwrap();

        let inspection = inspect_logo(buffer).unwrap();
        assert_eq!(inspection.slot_count, 1);
        assert_eq!(inspection.slots.len(), 1);
        assert_eq!(inspection.slots[0].raw_size, z_lib::deflate(&raw).unwrap().len());
    }

    #[test]
    fn repack_logo_roundtrips_png_slot() {
        let mut png = Vec::new();
        let rgba = [0x12, 0x34, 0x56, 0xFF];
        image::rgba_to_png(&mut png, &rgba, 1, 1).unwrap();

        let output = repack_logo(
            vec![GuiFile {
                name: "logo_000_bgrabe.png".to_string(),
                bytes: png,
            }],
            false,
        )
        .unwrap();

        let mut reader = Cursor::new(output);
        let image = LogoImage::read(&mut reader).unwrap();
        assert_eq!(image.blobs.len(), 1);
    }

    #[test]
    fn repack_logo_sorts_slots_by_filename_index() {
        let output = repack_logo(
            vec![
                GuiFile {
                    name: "logo_010_raw.z".to_string(),
                    bytes: vec![10],
                },
                GuiFile {
                    name: "logo_000_raw.z".to_string(),
                    bytes: vec![0],
                },
            ],
            false,
        )
        .unwrap();

        let mut reader = Cursor::new(output);
        let image = LogoImage::read(&mut reader).unwrap();
        assert_eq!(image.blobs, vec![vec![0], vec![10]]);
    }
}
