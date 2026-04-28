use crate::infer::{infer_profile, infer_slot, InferredFormat, ScreenHint};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufReader, Result};
use std::path::PathBuf;
use super::{cmd, data1, emphasize1, warn};
use super::mtklogo::LogoImage;
use super::mtklogo::utils::z_lib;

pub fn run_infer_profile(path: PathBuf, slots: Option<Vec<usize>>, screen_hint: Option<ScreenHint>) -> Result<()> {
    println!("{} file {}.", cmd("infer-profile"), emphasize1(path.display()));

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let image = LogoImage::read(&mut reader)?;

    let mut inflated_sizes = Vec::new();
    let mut unresolved = BTreeSet::new();

    for (id, blob) in image.blobs.iter().enumerate() {
        let should_include = match slots {
            None => true,
            Some(ref selected) => selected.contains(&id),
        };

        if !should_include {
            continue;
        }

        let inflated = z_lib::inflate(blob as &[u8])?;
        let inflated_size = inflated.len() as u32;
        inflated_sizes.push(inflated_size);

        if infer_slot(inflated_size, screen_hint).is_empty() {
            unresolved.insert(inflated_size);
        }
    }

    if let Some(inference) = infer_profile(&inflated_sizes, screen_hint) {
        println!("name: inferred_logo");
        if let Some(ScreenHint { width, height }) = screen_hint {
            println!("screen_hint: {}x{}", width, height);
        }
        println!("color_model: {}", inference.mode);
        println!("formats:");
        for InferredFormat { width, height } in inference.formats {
            println!("- {{ w: {}, h: {} }}", width, height);
        }
    } else {
        println!("{} no confident profile could be inferred.", warn("warning"));
    }

    if !unresolved.is_empty() {
        println!("{} unresolved inflated sizes:", warn("warning"));
        for size in unresolved {
            println!("- {} bytes", data1(size));
        }
    }

    Ok(())
}
