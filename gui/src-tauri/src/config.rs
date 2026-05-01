use mtklogo::ColorMode;
use serde::{Deserialize, Serialize};
use std::io::{Error as IOError, ErrorKind, Result};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Profile {
    pub name: String,
    pub color_model: String,
    pub alias: Option<Vec<String>>,
    pub formats: Vec<Format>,
}

impl Profile {
    pub fn guess_format(&self, size: u32, flip: bool) -> Result<Format> {
        let mtk_color_model = ColorMode::by_name(&self.color_model)?;
        let bpp = mtk_color_model.bytes_per_pixel();
        let pixels = size / bpp;
        let format = self
            .formats
            .iter()
            .find(|f| f.w * f.h == pixels)
            .cloned();

        match format {
            Some(found) => {
                if flip {
                    Ok(found.flip())
                } else {
                    Ok(found)
                }
            }
            None => Err(IOError::new(
                ErrorKind::InvalidData,
                format!(
                    "size '{}' does not correspond to any dimension in profile '{}'",
                    size, self.name
                ),
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Format {
    pub w: u32,
    pub h: u32,
    pub t: Option<String>,
}

impl Format {
    pub fn flip(&self) -> Format {
        let flipped_title = self.t.as_ref().map(|s| format!("flip({})", s));
        Format {
            w: self.h,
            h: self.w,
            t: flipped_title,
        }
    }
}
