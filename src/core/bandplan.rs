// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct BandDefinition {
    pub name: &'static str,
    pub min_freq_hz: u64,
    pub max_freq_hz: u64,
    pub default_freq_hz: u64,
    pub segments: &'static [BandSegment],
}

#[derive(Debug, Clone)]
pub struct BandSegment {
    pub label: &'static str,
    pub start_hz: u64,
    pub end_hz: u64,
    pub mode: SegmentMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SegmentMode {
    Cw,
    Data,
    Ssb,
    Fm,
    Beacon,
}

pub static AMATEUR_BANDS: &[BandDefinition] = &[
    BandDefinition {
        name: "160m",
        min_freq_hz: 1_800_000,
        max_freq_hz: 2_000_000,
        default_freq_hz: 1_840_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 1_800_000,
                end_hz: 1_838_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI",
                start_hz: 1_838_000,
                end_hz: 1_840_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "SSB",
                start_hz: 1_840_000,
                end_hz: 2_000_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "80m",
        min_freq_hz: 3_500_000,
        max_freq_hz: 3_800_000,
        default_freq_hz: 3_710_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 3_500_000,
                end_hz: 3_570_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI/FT8",
                start_hz: 3_570_000,
                end_hz: 3_600_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "SSB",
                start_hz: 3_600_000,
                end_hz: 3_800_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "60m",
        min_freq_hz: 5_351_500,
        max_freq_hz: 5_366_500,
        default_freq_hz: 5_357_000,
        segments: &[
            BandSegment {
                label: "CW/DIGI",
                start_hz: 5_351_500,
                end_hz: 5_354_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "USB",
                start_hz: 5_354_000,
                end_hz: 5_366_500,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "40m",
        min_freq_hz: 7_000_000,
        max_freq_hz: 7_200_000,
        default_freq_hz: 7_150_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 7_000_000,
                end_hz: 7_040_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI/FT8",
                start_hz: 7_040_000,
                end_hz: 7_050_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "SSB",
                start_hz: 7_050_000,
                end_hz: 7_200_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "30m",
        min_freq_hz: 10_100_000,
        max_freq_hz: 10_150_000,
        default_freq_hz: 10_136_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 10_100_000,
                end_hz: 10_130_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI/FT8",
                start_hz: 10_130_000,
                end_hz: 10_150_000,
                mode: SegmentMode::Data,
            },
        ],
    },
    BandDefinition {
        name: "20m",
        min_freq_hz: 14_000_000,
        max_freq_hz: 14_350_000,
        default_freq_hz: 14_195_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 14_000_000,
                end_hz: 14_070_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI/FT8",
                start_hz: 14_070_000,
                end_hz: 14_099_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "BEACON",
                start_hz: 14_099_000,
                end_hz: 14_101_000,
                mode: SegmentMode::Beacon,
            },
            BandSegment {
                label: "SSB",
                start_hz: 14_101_000,
                end_hz: 14_350_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "17m",
        min_freq_hz: 18_068_000,
        max_freq_hz: 18_168_000,
        default_freq_hz: 18_130_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 18_068_000,
                end_hz: 18_095_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI/FT8",
                start_hz: 18_095_000,
                end_hz: 18_111_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "SSB",
                start_hz: 18_111_000,
                end_hz: 18_168_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "15m",
        min_freq_hz: 21_000_000,
        max_freq_hz: 21_450_000,
        default_freq_hz: 21_250_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 21_000_000,
                end_hz: 21_070_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI/FT8",
                start_hz: 21_070_000,
                end_hz: 21_110_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "SSB",
                start_hz: 21_150_000,
                end_hz: 21_450_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "12m",
        min_freq_hz: 24_890_000,
        max_freq_hz: 24_990_000,
        default_freq_hz: 24_950_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 24_890_000,
                end_hz: 24_915_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI/FT8",
                start_hz: 24_915_000,
                end_hz: 24_940_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "SSB",
                start_hz: 24_940_000,
                end_hz: 24_990_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "10m",
        min_freq_hz: 28_000_000,
        max_freq_hz: 29_700_000,
        default_freq_hz: 28_500_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 28_000_000,
                end_hz: 28_070_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "DIGI/FT8",
                start_hz: 28_070_000,
                end_hz: 28_190_000,
                mode: SegmentMode::Data,
            },
            BandSegment {
                label: "SSB",
                start_hz: 28_300_000,
                end_hz: 29_000_000,
                mode: SegmentMode::Ssb,
            },
            BandSegment {
                label: "FM",
                start_hz: 29_510_000,
                end_hz: 29_700_000,
                mode: SegmentMode::Fm,
            },
        ],
    },
    BandDefinition {
        name: "6m",
        min_freq_hz: 50_000_000,
        max_freq_hz: 52_000_000,
        default_freq_hz: 50_150_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 50_000_000,
                end_hz: 50_100_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "SSB/DIGI",
                start_hz: 50_100_000,
                end_hz: 50_500_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "4m",
        min_freq_hz: 70_000_000,
        max_freq_hz: 70_500_000,
        default_freq_hz: 70_200_000,
        segments: &[
            BandSegment {
                label: "CW/SSB",
                start_hz: 70_000_000,
                end_hz: 70_250_000,
                mode: SegmentMode::Ssb,
            },
            BandSegment {
                label: "FM",
                start_hz: 70_250_000,
                end_hz: 70_500_000,
                mode: SegmentMode::Fm,
            },
        ],
    },
    BandDefinition {
        name: "2m",
        min_freq_hz: 144_000_000,
        max_freq_hz: 146_000_000,
        default_freq_hz: 144_300_000,
        segments: &[
            BandSegment {
                label: "CW",
                start_hz: 144_000_000,
                end_hz: 144_150_000,
                mode: SegmentMode::Cw,
            },
            BandSegment {
                label: "SSB/FT8",
                start_hz: 144_150_000,
                end_hz: 144_400_000,
                mode: SegmentMode::Ssb,
            },
            BandSegment {
                label: "FM/Simplex",
                start_hz: 145_200_000,
                end_hz: 145_590_000,
                mode: SegmentMode::Fm,
            },
        ],
    },
    BandDefinition {
        name: "70cm",
        min_freq_hz: 430_000_000,
        max_freq_hz: 440_000_000,
        default_freq_hz: 432_200_000,
        segments: &[
            BandSegment {
                label: "CW/SSB",
                start_hz: 432_000_000,
                end_hz: 432_400_000,
                mode: SegmentMode::Ssb,
            },
            BandSegment {
                label: "FM",
                start_hz: 433_000_000,
                end_hz: 434_000_000,
                mode: SegmentMode::Fm,
            },
        ],
    },
    BandDefinition {
        name: "23cm",
        min_freq_hz: 1_240_000_000,
        max_freq_hz: 1_300_000_000,
        default_freq_hz: 1_296_200_000,
        segments: &[
            BandSegment {
                label: "CW/SSB",
                start_hz: 1_296_000_000,
                end_hz: 1_296_800_000,
                mode: SegmentMode::Ssb,
            },
            BandSegment {
                label: "FM/ATV",
                start_hz: 1_297_000_000,
                end_hz: 1_300_000_000,
                mode: SegmentMode::Fm,
            },
        ],
    },
    BandDefinition {
        name: "13cm",
        min_freq_hz: 2_300_000_000,
        max_freq_hz: 2_450_000_000,
        default_freq_hz: 2_400_050_000, // QO-100 uplink
        segments: &[
            BandSegment {
                label: "CW/SSB",
                start_hz: 2_320_000_000,
                end_hz: 2_320_800_000,
                mode: SegmentMode::Ssb,
            },
            BandSegment {
                label: "QO-100 Uplink",
                start_hz: 2_400_000_000,
                end_hz: 2_400_500_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
    BandDefinition {
        name: "3cm",
        min_freq_hz: 10_000_000_000,
        max_freq_hz: 10_500_000_000,
        default_freq_hz: 10_489_500_000, // QO-100 downlink
        segments: &[
            BandSegment {
                label: "CW/SSB",
                start_hz: 10_368_000_000,
                end_hz: 10_368_800_000,
                mode: SegmentMode::Ssb,
            },
            BandSegment {
                label: "QO-100 Downlink",
                start_hz: 10_489_500_000,
                end_hz: 10_490_000_000,
                mode: SegmentMode::Ssb,
            },
        ],
    },
];

pub fn get_band_by_name(name: &str) -> Option<&'static BandDefinition> {
    AMATEUR_BANDS
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(name))
}

pub fn get_band_by_freq(freq_hz: u64) -> Option<&'static BandDefinition> {
    AMATEUR_BANDS
        .iter()
        .find(|b| freq_hz >= b.min_freq_hz && freq_hz <= b.max_freq_hz)
}

/// Zwraca nazwę pasma krótkofalarskiego (np. `"20m"`, `"60m"`, `"3cm"`) dla podanej
/// częstotliwości w kHz lub `None`, jeśli częstotliwość leży poza pasmami amatorskimi.
pub fn freq_khz_to_band(khz: f64) -> Option<&'static str> {
    if !khz.is_finite() || khz <= 0.0 {
        return None;
    }
    let freq_hz = (khz * 1000.0).round() as u64;
    get_band_by_freq(freq_hz).map(|b| b.name)
}

/// Zwraca sugerowaną emisję (np. `"CW"`, `"FT8"`, `"LSB"`, `"USB"`, `"FM"`) dla podanej
/// częstotliwości w Hz na podstawie segmentów IARU Region 1.
pub fn get_suggested_mode(freq_hz: u64) -> Option<&'static str> {
    let band = get_band_by_freq(freq_hz)?;
    band.segments
        .iter()
        .find(|seg| (seg.start_hz..=seg.end_hz).contains(&freq_hz))
        .map(|seg| suggested_mode_for(seg.mode, freq_hz))
}

impl SegmentMode {
    pub const ALL: [Self; 5] = [Self::Cw, Self::Data, Self::Ssb, Self::Fm, Self::Beacon];
}

/// Most segments a single band may have in a user plan.
pub const MAX_SEGMENTS_PER_BAND: usize = 32;
/// Longest segment label, in characters.
pub const MAX_LABEL_CHARS: usize = 24;
/// Largest band plan file accepted on import.
pub const MAX_PLAN_FILE_BYTES: u64 = 256 * 1024;

/// One segment of a user band plan; both edges are inclusive, in Hz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSegment {
    pub label: String,
    pub start_hz: u64,
    pub end_hz: u64,
    pub mode: SegmentMode,
}

/// Segments of one band, named as in [`AMATEUR_BANDS`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanBand {
    pub name: String,
    pub segments: Vec<PlanSegment>,
}

/// How every band is split into mode segments. The band edges themselves are
/// fixed by [`AMATEUR_BANDS`], because ADIF band names depend on them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandPlan {
    pub bands: Vec<PlanBand>,
}

/// Why a band plan was rejected. `index` is the segment position in its band.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BandPlanError {
    UnknownBand(String),
    DuplicateBand(String),
    TooManySegments { band: String },
    EmptyLabel { band: String, index: usize },
    LabelTooLong { band: String, index: usize },
    EmptyRange { band: String, index: usize },
    OutsideBand { band: String, index: usize },
    Overlap { band: String, index: usize },
    TooLarge,
    Format(String),
}

impl std::fmt::Display for BandPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownBand(band) => write!(f, "unknown band {band}"),
            Self::DuplicateBand(band) => write!(f, "band {band} appears twice"),
            Self::TooManySegments { band } => {
                write!(
                    f,
                    "band {band} has more than {MAX_SEGMENTS_PER_BAND} segments"
                )
            }
            Self::EmptyLabel { band, index } => write!(f, "{band} segment {index}: empty label"),
            Self::LabelTooLong { band, index } => {
                write!(
                    f,
                    "{band} segment {index}: label longer than {MAX_LABEL_CHARS}"
                )
            }
            Self::EmptyRange { band, index } => {
                write!(f, "{band} segment {index}: start is not below the end")
            }
            Self::OutsideBand { band, index } => {
                write!(f, "{band} segment {index}: outside the band edges")
            }
            Self::Overlap { band, index } => {
                write!(f, "{band} segment {index}: overlaps the previous segment")
            }
            Self::TooLarge => write!(
                f,
                "band plan file is larger than {MAX_PLAN_FILE_BYTES} bytes"
            ),
            Self::Format(detail) => write!(f, "band plan file is not valid: {detail}"),
        }
    }
}

impl std::error::Error for BandPlanError {}

impl BandPlanError {
    /// Band and segment the problem points at, if it points at one.
    pub fn location(&self) -> Option<(&str, Option<usize>)> {
        match self {
            Self::UnknownBand(band)
            | Self::DuplicateBand(band)
            | Self::TooManySegments { band } => Some((band, None)),
            Self::EmptyLabel { band, index }
            | Self::LabelTooLong { band, index }
            | Self::EmptyRange { band, index }
            | Self::OutsideBand { band, index }
            | Self::Overlap { band, index } => Some((band, Some(*index))),
            Self::TooLarge | Self::Format(_) => None,
        }
    }
}

static IARU_REGION1: LazyLock<BandPlan> = LazyLock::new(|| BandPlan {
    bands: AMATEUR_BANDS
        .iter()
        .map(|band| PlanBand {
            name: band.name.to_string(),
            segments: band
                .segments
                .iter()
                .map(|seg| PlanSegment {
                    label: seg.label.to_string(),
                    start_hz: seg.start_hz,
                    end_hz: seg.end_hz,
                    mode: seg.mode,
                })
                .collect(),
        })
        .collect(),
});

/// Mode a segment suggests at `freq_hz`; phone below 10 MHz is LSB except on 60 m.
fn suggested_mode_for(mode: SegmentMode, freq_hz: u64) -> &'static str {
    match mode {
        SegmentMode::Cw | SegmentMode::Beacon => "CW",
        SegmentMode::Data => "FT8",
        SegmentMode::Fm => "FM",
        SegmentMode::Ssb => {
            if freq_hz < 10_000_000 && !(5_351_500..=5_366_500).contains(&freq_hz) {
                "LSB"
            } else {
                "USB"
            }
        }
    }
}

impl BandPlan {
    /// The built-in IARU Region 1 plan.
    pub fn iaru_region1() -> &'static Self {
        &IARU_REGION1
    }

    pub fn band(&self, name: &str) -> Option<&PlanBand> {
        self.bands.iter().find(|b| b.name == name)
    }

    pub fn band_mut(&mut self, name: &str) -> Option<&mut PlanBand> {
        self.bands.iter_mut().find(|b| b.name == name)
    }

    /// First segment containing `freq_hz`.
    pub fn segment_at(&self, freq_hz: u64) -> Option<&PlanSegment> {
        let band = get_band_by_freq(freq_hz)?;
        self.band(band.name)?
            .segments
            .iter()
            .find(|seg| (seg.start_hz..=seg.end_hz).contains(&freq_hz))
    }

    /// Suggested mode (`CW`, `FT8`, `LSB`, `USB`, `FM`) at `freq_hz`.
    pub fn suggested_mode(&self, freq_hz: u64) -> Option<&'static str> {
        self.segment_at(freq_hz)
            .map(|seg| suggested_mode_for(seg.mode, freq_hz))
    }

    /// Restores one band to the IARU Region 1 segments.
    pub fn reset_band(&mut self, name: &str) {
        if let (Some(target), Some(source)) = (self.band_mut(name), Self::iaru_region1().band(name))
        {
            target.segments.clone_from(&source.segments);
        }
    }

    /// Puts bands in the order of [`AMATEUR_BANDS`], adds missing bands with the
    /// IARU Region 1 segments, trims labels and sorts segments by frequency.
    pub fn normalize(&mut self) {
        for band in &Self::iaru_region1().bands {
            if self.band(&band.name).is_none() {
                self.bands.push(band.clone());
            }
        }
        let position = |name: &str| {
            AMATEUR_BANDS
                .iter()
                .position(|b| b.name == name)
                .unwrap_or(usize::MAX)
        };
        self.bands.sort_by_key(|b| position(&b.name));
        for band in &mut self.bands {
            for seg in &mut band.segments {
                let trimmed = seg.label.trim();
                if trimmed.len() != seg.label.len() {
                    seg.label = trimmed.to_string();
                }
            }
            band.segments.sort_by_key(|seg| (seg.start_hz, seg.end_hz));
        }
    }

    /// Checks band names, segment labels and ranges. Segments must be sorted and
    /// may touch but not overlap.
    pub fn validate(&self) -> Result<(), BandPlanError> {
        let mut seen: Vec<&str> = Vec::with_capacity(self.bands.len());
        for band in &self.bands {
            let Some(def) = get_band_by_name(&band.name).filter(|d| d.name == band.name) else {
                return Err(BandPlanError::UnknownBand(band.name.clone()));
            };
            if seen.contains(&def.name) {
                return Err(BandPlanError::DuplicateBand(band.name.clone()));
            }
            seen.push(def.name);
            if band.segments.len() > MAX_SEGMENTS_PER_BAND {
                return Err(BandPlanError::TooManySegments {
                    band: band.name.clone(),
                });
            }
            let mut previous_end = None;
            for (index, seg) in band.segments.iter().enumerate() {
                let at = || (band.name.clone(), index);
                let label = seg.label.trim();
                if label.is_empty() {
                    let (band, index) = at();
                    return Err(BandPlanError::EmptyLabel { band, index });
                }
                if label.chars().count() > MAX_LABEL_CHARS || label.chars().any(char::is_control) {
                    let (band, index) = at();
                    return Err(BandPlanError::LabelTooLong { band, index });
                }
                if seg.start_hz >= seg.end_hz {
                    let (band, index) = at();
                    return Err(BandPlanError::EmptyRange { band, index });
                }
                if seg.start_hz < def.min_freq_hz || seg.end_hz > def.max_freq_hz {
                    let (band, index) = at();
                    return Err(BandPlanError::OutsideBand { band, index });
                }
                if previous_end.is_some_and(|end| seg.start_hz < end) {
                    let (band, index) = at();
                    return Err(BandPlanError::Overlap { band, index });
                }
                previous_end = Some(seg.end_hz);
            }
        }
        Ok(())
    }

    /// Reads a plan saved by [`BandPlan::to_json`]; the result is normalized and valid.
    pub fn from_json(text: &str) -> Result<Self, BandPlanError> {
        if text.len() as u64 > MAX_PLAN_FILE_BYTES {
            return Err(BandPlanError::TooLarge);
        }
        let mut plan: Self =
            serde_json::from_str(text).map_err(|e| BandPlanError::Format(e.to_string()))?;
        plan.normalize();
        plan.validate()?;
        Ok(plan)
    }

    pub fn to_json(&self) -> Result<String, BandPlanError> {
        serde_json::to_string_pretty(self).map_err(|e| BandPlanError::Format(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_20m(segments: Vec<PlanSegment>) -> BandPlan {
        let mut plan = BandPlan::iaru_region1().clone();
        plan.band_mut("20m").unwrap().segments = segments;
        plan
    }

    fn seg(label: &str, start_hz: u64, end_hz: u64, mode: SegmentMode) -> PlanSegment {
        PlanSegment {
            label: label.to_string(),
            start_hz,
            end_hz,
            mode,
        }
    }

    #[test]
    fn built_in_plan_matches_the_static_table_and_is_valid() {
        let plan = BandPlan::iaru_region1();
        assert_eq!(plan.bands.len(), AMATEUR_BANDS.len());
        assert_eq!(plan.validate(), Ok(()));
        for freq in [
            1_830_000,
            3_573_000,
            7_150_000,
            14_074_000,
            14_250_000,
            145_500_000,
        ] {
            assert_eq!(plan.suggested_mode(freq), get_suggested_mode(freq));
        }
        assert_eq!(plan.suggested_mode(14_250_000), Some("USB"));
        assert_eq!(plan.suggested_mode(5_357_000), Some("USB"));
        assert_eq!(plan.suggested_mode(16_000_000), None);
    }

    #[test]
    fn user_segments_change_the_suggested_mode() {
        let plan = plan_20m(vec![
            seg("CW", 14_000_000, 14_100_000, SegmentMode::Cw),
            seg("FT8", 14_100_000, 14_120_000, SegmentMode::Data),
        ]);
        assert_eq!(plan.validate(), Ok(()));
        assert_eq!(plan.suggested_mode(14_090_000), Some("CW"));
        assert_eq!(plan.suggested_mode(14_110_000), Some("FT8"));
        assert_eq!(plan.suggested_mode(14_250_000), None);
    }

    #[test]
    fn validation_points_at_the_broken_segment() {
        let at = |segments, expected: BandPlanError| {
            let err = plan_20m(segments).validate().unwrap_err();
            assert_eq!(err, expected);
            assert_eq!(err.location(), Some(("20m", Some(1))));
        };
        let band = || "20m".to_string();
        at(
            vec![
                seg("CW", 14_000_000, 14_070_000, SegmentMode::Cw),
                seg(" ", 14_070_000, 14_100_000, SegmentMode::Data),
            ],
            BandPlanError::EmptyLabel {
                band: band(),
                index: 1,
            },
        );
        at(
            vec![
                seg("CW", 14_000_000, 14_070_000, SegmentMode::Cw),
                seg("DIGI", 14_090_000, 14_090_000, SegmentMode::Data),
            ],
            BandPlanError::EmptyRange {
                band: band(),
                index: 1,
            },
        );
        at(
            vec![
                seg("CW", 14_000_000, 14_070_000, SegmentMode::Cw),
                seg("SSB", 14_100_000, 14_400_000, SegmentMode::Ssb),
            ],
            BandPlanError::OutsideBand {
                band: band(),
                index: 1,
            },
        );
        at(
            vec![
                seg("CW", 14_000_000, 14_070_000, SegmentMode::Cw),
                seg("DIGI", 14_060_000, 14_100_000, SegmentMode::Data),
            ],
            BandPlanError::Overlap {
                band: band(),
                index: 1,
            },
        );
        at(
            vec![
                seg("CW", 14_000_000, 14_070_000, SegmentMode::Cw),
                seg(
                    &"X".repeat(MAX_LABEL_CHARS + 1),
                    14_070_000,
                    14_100_000,
                    SegmentMode::Data,
                ),
            ],
            BandPlanError::LabelTooLong {
                band: band(),
                index: 1,
            },
        );
        let many = (0..=MAX_SEGMENTS_PER_BAND as u64)
            .map(|i| {
                seg(
                    "CW",
                    14_000_000 + i * 1000,
                    14_000_500 + i * 1000,
                    SegmentMode::Cw,
                )
            })
            .collect();
        assert_eq!(
            plan_20m(many).validate(),
            Err(BandPlanError::TooManySegments { band: band() })
        );
    }

    #[test]
    fn band_names_must_be_known_and_unique() {
        let mut plan = BandPlan::iaru_region1().clone();
        plan.bands.push(PlanBand {
            name: "20m".to_string(),
            segments: vec![],
        });
        assert_eq!(
            plan.validate(),
            Err(BandPlanError::DuplicateBand("20m".to_string()))
        );
        let unknown = BandPlan {
            bands: vec![PlanBand {
                name: "11m".to_string(),
                segments: vec![],
            }],
        };
        assert_eq!(
            unknown.validate(),
            Err(BandPlanError::UnknownBand("11m".to_string()))
        );
        // Wielkość liter musi się zgadzać, bo nazwa trafia do ADIF.
        let upper = BandPlan {
            bands: vec![PlanBand {
                name: "20M".to_string(),
                segments: vec![],
            }],
        };
        assert!(upper.validate().is_err());
    }

    #[test]
    fn json_round_trip_normalizes_a_partial_plan() {
        let text = r#"{"bands":[{"name":"20m","segments":[
            {"label":" SSB ","start_hz":14100000,"end_hz":14350000,"mode":"ssb"},
            {"label":"CW","start_hz":14000000,"end_hz":14100000,"mode":"cw"}]}]}"#;
        let plan = BandPlan::from_json(text).unwrap();
        assert_eq!(plan.bands.len(), AMATEUR_BANDS.len());
        assert_eq!(plan.bands[5].name, "20m");
        let labels: Vec<&str> = plan
            .band("20m")
            .unwrap()
            .segments
            .iter()
            .map(|s| s.label.as_str())
            .collect();
        assert_eq!(labels, ["CW", "SSB"]);
        assert_eq!(plan.band("40m"), BandPlan::iaru_region1().band("40m"));
        assert_eq!(BandPlan::from_json(&plan.to_json().unwrap()), Ok(plan));
    }

    #[test]
    fn json_import_rejects_garbage_and_oversized_files() {
        assert!(matches!(
            BandPlan::from_json("not json"),
            Err(BandPlanError::Format(_))
        ));
        assert!(matches!(
            BandPlan::from_json(
                r#"{"bands":[{"name":"20m","segments":[{"label":"X","start_hz":1,"end_hz":2,"mode":"psk"}]}]}"#
            ),
            Err(BandPlanError::Format(_))
        ));
        let huge = " ".repeat(MAX_PLAN_FILE_BYTES as usize + 1);
        assert_eq!(BandPlan::from_json(&huge), Err(BandPlanError::TooLarge));
        assert!(matches!(
            BandPlan::from_json(
                r#"{"bands":[{"name":"20m","segments":[{"label":"X","start_hz":7000000,"end_hz":7100000,"mode":"cw"}]}]}"#
            ),
            Err(BandPlanError::OutsideBand { .. })
        ));
    }

    #[test]
    fn reset_band_restores_only_that_band() {
        let mut plan = plan_20m(vec![seg("CW", 14_000_000, 14_350_000, SegmentMode::Cw)]);
        plan.band_mut("40m").unwrap().segments.clear();
        plan.reset_band("20m");
        assert_eq!(plan.band("20m"), BandPlan::iaru_region1().band("20m"));
        assert_eq!(plan.band("40m").unwrap().segments, []);
    }

    #[test]
    fn test_band_lookup() {
        let band20 = get_band_by_freq(14_195_000).unwrap();
        assert_eq!(band20.name, "20m");

        let band40 = get_band_by_name("40m").unwrap();
        assert_eq!(band40.default_freq_hz, 7_150_000);

        let band2m = get_band_by_freq(144_300_000).unwrap();
        assert_eq!(band2m.name, "2m");

        let band13cm = get_band_by_freq(2_400_050_000).unwrap();
        assert_eq!(band13cm.name, "13cm");

        let band3cm = get_band_by_freq(10_489_500_000).unwrap();
        assert_eq!(band3cm.name, "3cm");

        assert!(get_band_by_freq(100_000).is_none());
    }

    #[test]
    fn test_band_boundaries_are_inclusive() {
        // Dolna i górna granica pasma 20m muszą być włączone
        assert_eq!(get_band_by_freq(14_000_000).unwrap().name, "20m");
        assert_eq!(get_band_by_freq(14_350_000).unwrap().name, "20m");

        // Tuż poniżej / powyżej granic pasm — luka między pasmami (brak pasma)
        assert!(get_band_by_freq(14_350_001).is_none());
        assert!(get_band_by_freq(1_799_999).is_none());
    }

    #[test]
    fn test_band_gaps_return_other_band() {
        // 2.0..3.5 MHz to luka (brak pasma) -> None
        assert!(get_band_by_freq(2_500_000).is_none());
        // 14.35..18.068 MHz luka
        assert!(get_band_by_freq(16_000_000).is_none());
        // Pasmo 60m jest wąskie (5.3515..5.3665)
        assert_eq!(get_band_by_freq(5_357_000).unwrap().name, "60m");
        assert!(get_band_by_freq(5_400_000).is_none());
    }

    #[test]
    fn test_band_name_lookup_is_case_insensitive() {
        assert!(get_band_by_name("20M").is_some());
        assert!(get_band_by_name("2m").is_some());
        assert!(get_band_by_name("").is_none());
        assert!(get_band_by_name("unknown").is_none());
    }

    #[test]
    fn test_zero_freq_is_out_of_band() {
        assert!(get_band_by_freq(0).is_none());
    }
}
