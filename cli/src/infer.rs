use crate::config::{Format, Profile};
use mtklogo::ColorMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenHint {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredFormat {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotInference {
    pub mode: ColorMode,
    pub format: InferredFormat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileInference {
    pub mode: ColorMode,
    pub formats: Vec<InferredFormat>,
}

impl ProfileInference {
    pub fn to_profile(&self, name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            color_model: self.mode.to_string(),
            alias: None,
            formats: self.formats.iter().map(|format| Format {
                w: format.width,
                h: format.height,
                t: None,
            }).collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct ScoredInference {
    inference: SlotInference,
    score: u32,
}

const KNOWN_FORMATS: &[(u32, u32, u32)] = &[
    (14, 16, 68),
    (42, 64, 72),
    (45, 64, 70),
    (45, 56, 71),
    (57, 64, 65),
    (120, 120, 74),
    (192, 120, 76),
    (84, 121, 55),
    (108, 121, 55),
    (163, 29, 65),
    (163, 1, 80),
    (304, 52, 50),
    (304, 1, 70),
    (720, 1280, 85),
    (720, 1440, 90),
    (720, 1600, 90),
    (1200, 1920, 88),
    (1080, 1920, 92),
    (1080, 2160, 94),
    (1080, 2280, 95),
    (1080, 2340, 95),
    (1080, 2400, 100),
    (1080, 2460, 94),
    (1536, 1500, 89),
    (1600, 1440, 96),
    (1920, 1200, 106),
    (1440, 2560, 90),
    (1440, 3040, 93),
    (1440, 3200, 93),
    (2160, 3840, 82),
];

const COMMON_WIDTHS: &[u32] = &[720, 1080, 1200, 1440, 1600, 1920, 2160];

fn preferred_modes() -> Vec<ColorMode> {
    vec![
        ColorMode::Bgra(mtklogo::Endian::Big),
        ColorMode::Bgra(mtklogo::Endian::Little),
        ColorMode::Rgba(mtklogo::Endian::Big),
        ColorMode::Rgba(mtklogo::Endian::Little),
        ColorMode::Rgb565(mtklogo::Endian::Little),
        ColorMode::Rgb565(mtklogo::Endian::Big),
    ]
}

fn mode_bias(mode: &ColorMode) -> u32 {
    match mode {
        ColorMode::Bgra(mtklogo::Endian::Big) => 10,
        ColorMode::Bgra(mtklogo::Endian::Little) => 8,
        ColorMode::Rgba(mtklogo::Endian::Big) => 6,
        ColorMode::Rgba(mtklogo::Endian::Little) => 5,
        ColorMode::Rgb565(mtklogo::Endian::Little) => 2,
        ColorMode::Rgb565(mtklogo::Endian::Big) => 1,
    }
}

fn is_portrait(width: u32, height: u32) -> bool {
    height >= width
}

fn is_landscape(width: u32, height: u32) -> bool {
    width > height
}

fn is_large_format(width: u32, height: u32) -> bool {
    width * height >= 1_000_000
}

fn scored_candidates(inflated_size: u32, screen_hint: Option<ScreenHint>) -> Vec<ScoredInference> {
    let mut guesses = Vec::new();

    for mode in preferred_modes() {
        let bpp = mode.bytes_per_pixel();
        if inflated_size % bpp != 0 {
            continue;
        }

        let pixels = inflated_size / bpp;

        for &(width, height, base_score) in KNOWN_FORMATS {
            if width * height == pixels {
                guesses.push(ScoredInference {
                    inference: SlotInference {
                        mode: mode.clone(),
                        format: InferredFormat { width, height },
                    },
                    score: base_score + mode_bias(&mode) + screen_hint_bias(width, height, screen_hint),
                });
            }
        }

        for &width in COMMON_WIDTHS {
            if pixels % width != 0 {
                continue;
            }

            let height = pixels / width;
            let score = if is_portrait(width, height) && height <= width * 4 {
                40 + mode_bias(&mode)
            } else if is_landscape(width, height) && width <= height * 4 {
                36 + mode_bias(&mode)
            } else {
                10 + mode_bias(&mode)
            } + screen_hint_bias(width, height, screen_hint);

            guesses.push(ScoredInference {
                inference: SlotInference {
                    mode: mode.clone(),
                    format: InferredFormat { width, height },
                },
                score,
            });
        }
    }

    guesses.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.inference.format.height.cmp(&a.inference.format.height))
            .then_with(|| b.inference.format.width.cmp(&a.inference.format.width))
    });
    guesses.dedup_by(|a, b| a.inference == b.inference);
    guesses
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Orientation {
    Portrait,
    Landscape,
}

fn dominant_orientation(all_candidates: &[Vec<ScoredInference>], mode: &ColorMode) -> Option<Orientation> {
    let mut portrait_score = 0;
    let mut landscape_score = 0;

    for candidates in all_candidates {
        for candidate in candidates.iter().filter(|candidate| candidate.inference.mode == *mode) {
            let format = &candidate.inference.format;
            if is_large_format(format.width, format.height) {
                if is_landscape(format.width, format.height) {
                    landscape_score = landscape_score.max(candidate.score);
                } else {
                    portrait_score = portrait_score.max(candidate.score);
                }
            }
        }
    }

    if portrait_score == 0 && landscape_score == 0 {
        None
    } else if landscape_score > portrait_score {
        Some(Orientation::Landscape)
    } else {
        Some(Orientation::Portrait)
    }
}

pub fn infer_profile(inflated_sizes: &[u32], screen_hint: Option<ScreenHint>) -> Option<ProfileInference> {
    let all_candidates: Vec<Vec<ScoredInference>> = inflated_sizes
        .iter()
        .map(|&size| scored_candidates(size, screen_hint))
        .collect();

    let mut best_mode: Option<(ColorMode, u32)> = None;
    for mode in preferred_modes() {
        let score = all_candidates
            .iter()
            .filter_map(|candidates| candidates.iter().find(|candidate| candidate.inference.mode == mode))
            .map(|candidate| candidate.score)
            .sum::<u32>();

        match &best_mode {
            Some((_, best_score)) if score <= *best_score => {}
            _ => best_mode = Some((mode, score)),
        }
    }

    let (mode, score) = best_mode?;
    if score == 0 {
        return None;
    }

    let preferred_orientation = dominant_orientation(&all_candidates, &mode);
    let mut formats = Vec::new();
    for candidates in all_candidates {
        let best_for_mode = candidates.iter()
            .filter(|candidate| candidate.inference.mode == mode)
            .filter(|candidate| {
                let format = &candidate.inference.format;
                match preferred_orientation {
                    Some(Orientation::Landscape) if is_large_format(format.width, format.height) => {
                        is_landscape(format.width, format.height)
                    }
                    Some(Orientation::Portrait) if is_large_format(format.width, format.height) => {
                        is_portrait(format.width, format.height)
                    }
                    _ => true,
                }
            })
            .max_by(|a, b| a.score.cmp(&b.score));

        let fallback = candidates.iter()
            .filter(|candidate| candidate.inference.mode == mode)
            .max_by(|a, b| a.score.cmp(&b.score));

        if let Some(candidate) = best_for_mode.or(fallback) {
            if !formats.contains(&candidate.inference.format) {
                formats.push(candidate.inference.format.clone());
            }
        }
    }

    formats.sort_by(|a, b| (b.width * b.height).cmp(&(a.width * a.height)));
    Some(ProfileInference { mode, formats })
}

pub fn infer_slot(inflated_size: u32, screen_hint: Option<ScreenHint>) -> Vec<SlotInference> {
    scored_candidates(inflated_size, screen_hint)
        .into_iter()
        .map(|candidate| candidate.inference)
        .collect()
}

fn screen_hint_bias(width: u32, height: u32, screen_hint: Option<ScreenHint>) -> u32 {
    match screen_hint {
        Some(ScreenHint { width: screen_width, height: screen_height }) => {
            if (width == screen_width && height == screen_height)
                || (width == screen_height && height == screen_width)
            {
                200
            } else if is_large_format(width, height) && (width == screen_width || height == screen_height) {
                40
            } else if is_large_format(width, height) && (width == screen_height || height == screen_width) {
                25
            } else {
                0
            }
        }
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{infer_profile, infer_slot, InferredFormat, ScreenHint};
    use mtklogo::{ColorMode, Endian};

    #[test]
    fn infer_slot_prefers_1080x2400_for_10368000_bytes() {
        let guesses = infer_slot(10_368_000, None);

        assert_eq!(guesses.first().map(|g| &g.mode), Some(&ColorMode::Bgra(Endian::Big)));
        assert_eq!(
            guesses.first().map(|g| &g.format),
            Some(&InferredFormat { width: 1080, height: 2400 })
        );
    }

    #[test]
    fn infer_slot_prefers_1920x1200_for_tablet_fullscreen_size() {
        let guesses = infer_slot(9_216_000, None);

        assert_eq!(guesses.first().map(|g| &g.mode), Some(&ColorMode::Bgra(Endian::Big)));
        assert_eq!(
            guesses.first().map(|g| &g.format),
            Some(&InferredFormat { width: 1920, height: 1200 })
        );
    }

    #[test]
    fn infer_profile_groups_common_large_and_small_assets() {
        let inferred = infer_profile(&[
            10_368_000, 10_368_000, 10_368_000, 11_520, 10_752, 10_080, 896,
        ], None).expect("profile should be inferred");

        assert_eq!(inferred.mode, ColorMode::Bgra(Endian::Big));
        assert!(inferred.formats.contains(&InferredFormat { width: 1080, height: 2400 }));
        assert!(inferred.formats.contains(&InferredFormat { width: 45, height: 64 }));
        assert!(inferred.formats.contains(&InferredFormat { width: 42, height: 64 }));
        assert!(inferred.formats.contains(&InferredFormat { width: 45, height: 56 }));
        assert!(inferred.formats.contains(&InferredFormat { width: 14, height: 16 }));
    }

    #[test]
    fn infer_slot_prefers_small_symbol_formats_from_real_file_sizes() {
        let guesses_10752 = infer_slot(10_752, None);
        assert_eq!(
            guesses_10752.first().map(|g| &g.format),
            Some(&InferredFormat { width: 42, height: 64 })
        );

        let guesses_10080 = infer_slot(10_080, None);
        assert_eq!(
            guesses_10080.first().map(|g| &g.format),
            Some(&InferredFormat { width: 45, height: 56 })
        );

        let guesses_896 = infer_slot(896, None);
        assert_eq!(
            guesses_896.first().map(|g| &g.format),
            Some(&InferredFormat { width: 14, height: 16 })
        );

        let guesses_92160 = infer_slot(92_160, None);
        assert_eq!(
            guesses_92160.first().map(|g| &g.format),
            Some(&InferredFormat { width: 192, height: 120 })
        );

        let guesses_57600 = infer_slot(57_600, None);
        assert_eq!(
            guesses_57600.first().map(|g| &g.format),
            Some(&InferredFormat { width: 120, height: 120 })
        );
    }

    #[test]
    fn infer_profile_prefers_landscape_tablet_family() {
        let inferred = infer_profile(&[
            9_216_000, 9_216_000, 92_160, 92_160, 57_600, 57_600,
        ], None).expect("profile should be inferred");

        assert_eq!(inferred.mode, ColorMode::Bgra(Endian::Big));
        assert!(inferred.formats.contains(&InferredFormat { width: 1920, height: 1200 }));
        assert!(inferred.formats.contains(&InferredFormat { width: 192, height: 120 }));
        assert!(inferred.formats.contains(&InferredFormat { width: 120, height: 120 }));
    }

    #[test]
    fn inferred_profile_converts_to_unpack_profile() {
        let inferred = infer_profile(&[10_368_000, 11_520], None).expect("profile should be inferred");
        let profile = inferred.to_profile("auto");

        assert_eq!(profile.name, "auto");
        assert_eq!(profile.color_model, "bgrabe");
        assert_eq!(profile.alias, None);
        assert!(profile.formats.iter().any(|format| format.w == 1080 && format.h == 2400 && format.t.is_none()));
        assert!(profile.formats.iter().any(|format| format.w == 45 && format.h == 64 && format.t.is_none()));
    }

    #[test]
    fn screen_hint_prefers_exact_tablet_resolution() {
        let hint = ScreenHint { width: 1920, height: 1200 };
        let guesses = infer_slot(9_216_000, Some(hint));

        assert_eq!(
            guesses.first().map(|g| &g.format),
            Some(&InferredFormat { width: 1920, height: 1200 })
        );
    }

    #[test]
    fn screen_hint_accepts_swapped_orientation() {
        let hint = ScreenHint { width: 1200, height: 1920 };
        let guesses = infer_slot(9_216_000, Some(hint));

        assert_eq!(
            guesses.first().map(|g| &g.format),
            Some(&InferredFormat { width: 1920, height: 1200 })
        );
    }
}
