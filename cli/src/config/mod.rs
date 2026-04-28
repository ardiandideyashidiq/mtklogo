use serde::{Deserialize, Serialize};
use serde_yaml;
use std::env;
use std::fs::File;
use std::io::{Error as IOError, ErrorKind, Result};
use std::path::{Path, PathBuf};
use mtklogo::ColorMode;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub version: String,
    pub profiles: Vec<Profile>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Profile {
    pub name: String,
    pub color_model: String,
    pub alias: Option<Vec<String>>,
    pub formats: Vec<Format>,
}

impl Profile {
    pub fn with_color_model(self, color_model: String) -> Profile {
        return Profile { name: self.name, color_model, alias: self.alias, formats: self.formats };
    }
    pub fn guess_format(&self, size: u32, flip: bool) -> Result<Format> {
        let mtk_color_model = ColorMode::by_name(&self.color_model)?;
        let bpp = mtk_color_model.bytes_per_pixel();
        let pixels = size / bpp;
        let o = self.formats.iter()
            .find(|f| f.w * f.h == pixels)
            .cloned();
        match o {
            Some(f) => {
                if flip {
                    Ok(f.flip())
                } else {
                    Ok(f)
                }
            }
            None => Err(IOError::new(ErrorKind::InvalidData,
                                     format!(
                                         "size '{}' does not correspond to any dimension in profile '{}'", size, self.name)))
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Format {
    pub w: u32,
    pub h: u32,
    pub t: Option<String>,
}

impl Format {
    pub fn flip(&self) -> Format {
        let flipped_title = match &self.t {
            Some(s) => Some(format!("flip({})", s)),
            None => None
        };
        Format {
            w: self.h,
            h: self.w,
            t: flipped_title,
        }
    }
}


impl Config {
    const GLOBAL_CONFIG: &'static str = "/etc/mtklogo.yaml";
    const RELATIVE_CONFIG: &'static str = "mtklogo.yaml";

    /// Resolves configuration path, in this order:
    /// - `"mtklogo.yaml"` in $HOME/.config
    /// - `"mtklogo.yaml"` in /etc
    /// - `"mtklogo.yaml"` in program's installation directory
    fn candidate_paths(home: Option<&Path>, current_exe: &Path) -> Vec<PathBuf> {
        let mut candidates = Vec::with_capacity(3);

        if let Some(home) = home {
            candidates.push(home.join(".config").join(Self::RELATIVE_CONFIG));
        }

        candidates.push(PathBuf::from(Self::GLOBAL_CONFIG));

        if let Some(parent) = current_exe.parent() {
            candidates.push(parent.join(Self::RELATIVE_CONFIG));
        }

        candidates
    }

    fn config_path() -> Result<(PathBuf, File)> {
        let home = env::var_os("HOME").map(PathBuf::from);
        let current_exe = env::current_exe()?;

        for candidate in Self::candidate_paths(home.as_deref(), current_exe.as_path()) {
            if let Ok(file) = File::open(candidate.as_path()) {
                return Ok((candidate, file));
            }
        }

        Err(IOError::new(
            ErrorKind::NotFound,
            "`mtklogo.yaml` configuration not found, please provide one.",
        ))
    }

    fn wrap_read(path: &Path, file: File) -> Result<Config> {
        let config = serde_yaml::from_reader(file);
        config.map_err(
            |e| IOError::new(ErrorKind::InvalidData,
                             format!(
                                 "could not read config {} -> '{}'", path.display(), e)))
    }

    pub fn from_file(path: &Path) -> Result<Config> {
        let file = File::open(path)?;
        Self::wrap_read(path, file)
    }

    pub fn load() -> Result<Config> {
        Config::config_path().and_then(|(path, file)| Self::wrap_read(path.as_path(), file))
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use std::path::{Path, PathBuf};

    #[test]
    fn config_candidates_follow_home_then_etc_then_executable_order() {
        let home = Path::new("/tmp/home");
        let exe = Path::new("/tmp/bin/mtklogo");

        let candidates = Config::candidate_paths(Some(home), exe);

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/tmp/home/.config/mtklogo.yaml"),
                PathBuf::from("/etc/mtklogo.yaml"),
                PathBuf::from("/tmp/bin/mtklogo.yaml"),
            ]
        );
    }
}
