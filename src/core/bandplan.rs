// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        min_freq_hz: 1_810_000,
        max_freq_hz: 2_000_000,
        default_freq_hz: 1_840_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 1_810_000, end_hz: 1_838_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI", start_hz: 1_838_000, end_hz: 1_840_000, mode: SegmentMode::Data },
            BandSegment { label: "SSB", start_hz: 1_840_000, end_hz: 2_000_000, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "80m",
        min_freq_hz: 3_500_000,
        max_freq_hz: 3_800_000,
        default_freq_hz: 3_710_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 3_500_000, end_hz: 3_570_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI/FT8", start_hz: 3_570_000, end_hz: 3_600_000, mode: SegmentMode::Data },
            BandSegment { label: "SSB", start_hz: 3_600_000, end_hz: 3_800_000, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "60m",
        min_freq_hz: 5_351_500,
        max_freq_hz: 5_366_500,
        default_freq_hz: 5_357_000,
        segments: &[
            BandSegment { label: "CW/DIGI", start_hz: 5_351_500, end_hz: 5_354_000, mode: SegmentMode::Data },
            BandSegment { label: "USB", start_hz: 5_354_000, end_hz: 5_366_500, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "40m",
        min_freq_hz: 7_000_000,
        max_freq_hz: 7_200_000,
        default_freq_hz: 7_150_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 7_000_000, end_hz: 7_040_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI/FT8", start_hz: 7_040_000, end_hz: 7_050_000, mode: SegmentMode::Data },
            BandSegment { label: "SSB", start_hz: 7_050_000, end_hz: 7_200_000, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "30m",
        min_freq_hz: 10_100_000,
        max_freq_hz: 10_150_000,
        default_freq_hz: 10_136_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 10_100_000, end_hz: 10_130_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI/FT8", start_hz: 10_130_000, end_hz: 10_150_000, mode: SegmentMode::Data },
        ],
    },
    BandDefinition {
        name: "20m",
        min_freq_hz: 14_000_000,
        max_freq_hz: 14_350_000,
        default_freq_hz: 14_195_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 14_000_000, end_hz: 14_070_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI/FT8", start_hz: 14_070_000, end_hz: 14_099_000, mode: SegmentMode::Data },
            BandSegment { label: "BEACON", start_hz: 14_099_000, end_hz: 14_101_000, mode: SegmentMode::Beacon },
            BandSegment { label: "SSB", start_hz: 14_101_000, end_hz: 14_350_000, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "17m",
        min_freq_hz: 18_068_000,
        max_freq_hz: 18_168_000,
        default_freq_hz: 18_130_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 18_068_000, end_hz: 18_095_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI/FT8", start_hz: 18_095_000, end_hz: 18_111_000, mode: SegmentMode::Data },
            BandSegment { label: "SSB", start_hz: 18_111_000, end_hz: 18_168_000, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "15m",
        min_freq_hz: 21_000_000,
        max_freq_hz: 21_450_000,
        default_freq_hz: 21_250_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 21_000_000, end_hz: 21_070_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI/FT8", start_hz: 21_070_000, end_hz: 21_110_000, mode: SegmentMode::Data },
            BandSegment { label: "SSB", start_hz: 21_150_000, end_hz: 21_450_000, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "12m",
        min_freq_hz: 24_890_000,
        max_freq_hz: 24_990_000,
        default_freq_hz: 24_950_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 24_890_000, end_hz: 24_915_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI/FT8", start_hz: 24_915_000, end_hz: 24_940_000, mode: SegmentMode::Data },
            BandSegment { label: "SSB", start_hz: 24_940_000, end_hz: 24_990_000, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "10m",
        min_freq_hz: 28_000_000,
        max_freq_hz: 29_700_000,
        default_freq_hz: 28_500_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 28_000_000, end_hz: 28_070_000, mode: SegmentMode::Cw },
            BandSegment { label: "DIGI/FT8", start_hz: 28_070_000, end_hz: 28_190_000, mode: SegmentMode::Data },
            BandSegment { label: "SSB", start_hz: 28_300_000, end_hz: 29_000_000, mode: SegmentMode::Ssb },
            BandSegment { label: "FM", start_hz: 29_510_000, end_hz: 29_700_000, mode: SegmentMode::Fm },
        ],
    },
    BandDefinition {
        name: "6m",
        min_freq_hz: 50_000_000,
        max_freq_hz: 52_000_000,
        default_freq_hz: 50_150_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 50_000_000, end_hz: 50_100_000, mode: SegmentMode::Cw },
            BandSegment { label: "SSB/DIGI", start_hz: 50_100_000, end_hz: 50_500_000, mode: SegmentMode::Ssb },
        ],
    },
    BandDefinition {
        name: "4m",
        min_freq_hz: 70_000_000,
        max_freq_hz: 70_500_000,
        default_freq_hz: 70_200_000,
        segments: &[
            BandSegment { label: "CW/SSB", start_hz: 70_000_000, end_hz: 70_250_000, mode: SegmentMode::Ssb },
            BandSegment { label: "FM", start_hz: 70_250_000, end_hz: 70_500_000, mode: SegmentMode::Fm },
        ],
    },
    BandDefinition {
        name: "2m",
        min_freq_hz: 144_000_000,
        max_freq_hz: 146_000_000,
        default_freq_hz: 144_300_000,
        segments: &[
            BandSegment { label: "CW", start_hz: 144_000_000, end_hz: 144_150_000, mode: SegmentMode::Cw },
            BandSegment { label: "SSB/FT8", start_hz: 144_150_000, end_hz: 144_400_000, mode: SegmentMode::Ssb },
            BandSegment { label: "FM/Simplex", start_hz: 145_200_000, end_hz: 145_590_000, mode: SegmentMode::Fm },
        ],
    },
    BandDefinition {
        name: "70cm",
        min_freq_hz: 430_000_000,
        max_freq_hz: 440_000_000,
        default_freq_hz: 432_200_000,
        segments: &[
            BandSegment { label: "CW/SSB", start_hz: 432_000_000, end_hz: 432_400_000, mode: SegmentMode::Ssb },
            BandSegment { label: "FM", start_hz: 433_000_000, end_hz: 434_000_000, mode: SegmentMode::Fm },
        ],
    },
    BandDefinition {
        name: "23cm",
        min_freq_hz: 1_240_000_000,
        max_freq_hz: 1_300_000_000,
        default_freq_hz: 1_296_200_000,
        segments: &[
            BandSegment { label: "CW/SSB", start_hz: 1_296_000_000, end_hz: 1_296_800_000, mode: SegmentMode::Ssb },
            BandSegment { label: "FM/ATV", start_hz: 1_297_000_000, end_hz: 1_300_000_000, mode: SegmentMode::Fm },
        ],
    },
];

pub fn get_band_by_name(name: &str) -> Option<&'static BandDefinition> {
    AMATEUR_BANDS.iter().find(|b| b.name.eq_ignore_ascii_case(name))
}

pub fn get_band_by_freq(freq_hz: u64) -> Option<&'static BandDefinition> {
    AMATEUR_BANDS.iter().find(|b| freq_hz >= b.min_freq_hz && freq_hz <= b.max_freq_hz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_band_lookup() {
        let band20 = get_band_by_freq(14_195_000).unwrap();
        assert_eq!(band20.name, "20m");

        let band40 = get_band_by_name("40m").unwrap();
        assert_eq!(band40.default_freq_hz, 7_150_000);

        let band2m = get_band_by_freq(144_300_000).unwrap();
        assert_eq!(band2m.name, "2m");

        assert!(get_band_by_freq(100_000).is_none());
    }
}

