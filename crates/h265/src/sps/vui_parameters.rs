use std::io;
use std::num::NonZero;

use byteorder::{BigEndian, ReadBytesExt};
use scuffle_bytes_util::{BitReader, range_check};
use scuffle_expgolomb::BitReaderExpGolombExt;

use super::{ConformanceWindow, Profile};
use crate::{AspectRatioIdc, VideoFormat};

mod hrd_parameters;

pub use hrd_parameters::*;

/// VUI parameters.
///
/// `vui_parameters()`
///
/// - ISO/IEC 23008-2 - E.2.1
/// - ISO/IEC 23008-2 - E.3.1
#[derive(Debug, Clone, PartialEq)]
pub struct VuiParameters {
    /// [`AspectRatioInfo`] if `aspect_ratio_info_present_flag` is `true`.
    pub aspect_ratio_info: AspectRatioInfo,
    /// Equal to `true` indicates that the cropped decoded pictures output are suitable
    /// for display using overscan.
    ///
    /// Equal to `false` indicates that the cropped decoded pictures output contain visually important information
    /// in the entire region out to the edges of the conformance cropping window of the picture, such that the
    /// cropped decoded pictures output should not be displayed using overscan. Instead, they should be displayed
    /// using either an exact match between the display area and the conformance cropping window, or using underscan.
    /// As used in this paragraph, the term "overscan" refers to display processes in which some parts near the
    /// borders of the cropped decoded pictures are not visible in the display area. The term "underscan" describes
    /// display processes in which the entire cropped decoded pictures are visible in the display area, but they do
    /// not cover the entire display area. For display processes that neither use overscan nor underscan, the display
    /// area exactly matches the area of the cropped decoded pictures.
    ///
    /// Only present if `overscan_info_present_flag` is `true`.
    pub overscan_appropriate_flag: Option<bool>,
    /// `video_format`, `video_full_range_flag` and `colour_primaries`, if `video_signal_type_present_flag` is `true`.
    ///
    /// See [`VideoSignalType`] for details.
    pub video_signal_type: VideoSignalType,
    /// `chroma_sample_loc_type_top_field` and `chroma_sample_loc_type_bottom_field`, if `chroma_loc_info_present_flag` is `true`.
    ///
    /// See [`ChromaLocInfo`] for details.
    pub chroma_loc_info: Option<ChromaLocInfo>,
    /// Equal to `true` indicates that the value of all decoded chroma samples is
    /// equal to [`1 << (BitDepthC − 1)`](crate::SpsRbsp::bit_depth_c).
    ///
    /// Equal to `false` provides no indication of decoded chroma sample values.
    pub neutral_chroma_indication_flag: bool,
    /// Equal to `true` indicates that the CVS conveys pictures that represent fields, and specifies that
    /// a picture timing SEI message shall be present in every access unit of the current CVS.
    ///
    /// Equal to `false` indicates that the CVS conveys pictures that represent frames and that a picture timing SEI message
    /// may or may not be present in any access unit of the current CVS.
    pub field_seq_flag: bool,
    /// Equal to `true` specifies that picture timing SEI messages are present for every
    /// picture and include the `pic_struct`, `source_scan_type`, and `duplicate_flag` syntax elements.
    ///
    /// Equal to `false` specifies that the `pic_struct` syntax element is not present in
    /// picture timing SEI messages.
    pub frame_field_info_present_flag: bool,
    /// `def_disp_win_left_offset`, `def_disp_win_right_offset`, `def_disp_win_top_offset` and `def_disp_win_bottom_offset`,
    /// if `default_display_window_flag` is `true`.
    ///
    /// See [`DefaultDisplayWindow`] for details.
    pub default_display_window: DefaultDisplayWindow,
    /// `vui_num_units_in_tick`, `vui_time_scale`, `vui_poc_proportional_to_timing_flag` and `vui_num_ticks_poc_diff_one_minus1`,
    /// if `vui_timing_info_present_flag` is `true`.
    ///
    /// See [`VuiTimingInfo`] for details.
    pub vui_timing_info: Option<VuiTimingInfo>,
    /// `tiles_fixed_structure_flag`, `motion_vectors_over_pic_boundaries_flag`, `restricted_ref_pic_lists_flag`,
    /// `min_spatial_segmentation_idc`, `max_bytes_per_pic_denom`, `max_bits_per_min_cu_denom`,
    /// `log2_max_mv_length_horizontal` and `log2_max_mv_length_vertical`, if `bitstream_restriction_flag` is `true`.
    ///
    /// See [`BitStreamRestriction`] for details.
    pub bitstream_restriction: BitStreamRestriction,
}

impl VuiParameters {
    // TODO: Find a solution for this
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn parse<R: io::Read>(
        bit_reader: &mut BitReader<R>,
        sps_max_sub_layers_minus1: u8,
        chroma_format_idc: u8,
        general_profile: &Profile,
        conformance_window: &ConformanceWindow,
        sub_width_c: u8,
        pic_width_in_luma_samples: NonZero<u64>,
        sub_height_c: u8,
        pic_height_in_luma_samples: NonZero<u64>,
    ) -> io::Result<Self> {
        let mut aspect_ratio_info = AspectRatioInfo::Predefined(AspectRatioIdc::Unspecified);
        let mut overscan_appropriate_flag = None;
        let mut video_signal_type = None;
        let mut chroma_loc_info = None;
        let mut default_display_window = None;
        let mut vui_timing_info = None;

        let aspect_ratio_info_present_flag = bit_reader.read_bit()?;
        if aspect_ratio_info_present_flag {
            let aspect_ratio_idc = bit_reader.read_u8()?;
            if aspect_ratio_idc == AspectRatioIdc::ExtendedSar.0 {
                let sar_width = bit_reader.read_u16::<BigEndian>()?;
                let sar_height = bit_reader.read_u16::<BigEndian>()?;
                aspect_ratio_info = AspectRatioInfo::ExtendedSar { sar_width, sar_height };
            } else {
                aspect_ratio_info = AspectRatioInfo::Predefined(aspect_ratio_idc.into());
            }
        }

        let overscan_info_present_flag = bit_reader.read_bit()?;
        if overscan_info_present_flag {
            overscan_appropriate_flag = Some(bit_reader.read_bit()?);
        }

        let video_signal_type_present_flag = bit_reader.read_bit()?;
        if video_signal_type_present_flag {
            let video_format = VideoFormat::from(bit_reader.read_bits(3)? as u8);
            let video_full_range_flag = bit_reader.read_bit()?;
            let colour_description_present_flag = bit_reader.read_bit()?;

            if colour_description_present_flag {
                let colour_primaries = bit_reader.read_u8()?;
                let transfer_characteristics = bit_reader.read_u8()?;
                let matrix_coeffs = bit_reader.read_u8()?;

                video_signal_type = Some(VideoSignalType {
                    video_format,
                    video_full_range_flag,
                    colour_primaries,
                    transfer_characteristics,
                    matrix_coeffs,
                });
            } else {
                video_signal_type = Some(VideoSignalType {
                    video_format,
                    video_full_range_flag,
                    ..Default::default()
                });
            }
        }

        let chroma_loc_info_present_flag = bit_reader.read_bit()?;

        if chroma_format_idc != 1 && chroma_loc_info_present_flag {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "chroma_loc_info_present_flag must be 0 if chroma_format_idc != 1",
            ));
        }

        if chroma_loc_info_present_flag {
            let chroma_sample_loc_type_top_field = bit_reader.read_exp_golomb()?;
            let chroma_sample_loc_type_bottom_field = bit_reader.read_exp_golomb()?;

            chroma_loc_info = Some(ChromaLocInfo {
                top_field: chroma_sample_loc_type_top_field,
                bottom_field: chroma_sample_loc_type_bottom_field,
            });
        }

        let neutral_chroma_indication_flag = bit_reader.read_bit()?;
        let field_seq_flag = bit_reader.read_bit()?;

        if general_profile.frame_only_constraint_flag && field_seq_flag {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "field_seq_flag must be 0 if general_frame_only_constraint_flag is 1",
            ));
        }

        let frame_field_info_present_flag = bit_reader.read_bit()?;

        if !frame_field_info_present_flag
            && (field_seq_flag || (general_profile.progressive_source_flag && general_profile.interlaced_source_flag))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame_field_info_present_flag must be 1 if field_seq_flag is 1 or general_progressive_source_flag and general_interlaced_source_flag are both 1",
            ));
        }

        let default_display_window_flag = bit_reader.read_bit()?;
        if default_display_window_flag {
            let def_disp_win_left_offset = bit_reader.read_exp_golomb()?;
            let def_disp_win_right_offset = bit_reader.read_exp_golomb()?;
            let def_disp_win_top_offset = bit_reader.read_exp_golomb()?;
            let def_disp_win_bottom_offset = bit_reader.read_exp_golomb()?;
            let left_offset = conformance_window.conf_win_left_offset + def_disp_win_left_offset;
            let right_offset = conformance_window.conf_win_right_offset + def_disp_win_right_offset;
            let top_offset = conformance_window.conf_win_top_offset + def_disp_win_top_offset;
            let bottom_offset = conformance_window.conf_win_bottom_offset + def_disp_win_bottom_offset;

            if sub_width_c as u64 * (left_offset + right_offset) >= pic_width_in_luma_samples.get() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "sub_width_c * (left_offset + right_offset) must be less than pic_width_in_luma_samples",
                ));
            }

            if sub_height_c as u64 * (top_offset + bottom_offset) >= pic_height_in_luma_samples.get() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "sub_height_c * (top_offset + bottom_offset) must be less than pic_height_in_luma_samples",
                ));
            }

            default_display_window = Some(DefaultDisplayWindow {
                def_disp_win_left_offset,
                def_disp_win_right_offset,
                def_disp_win_top_offset,
                def_disp_win_bottom_offset,
            });
        }

        let vui_timing_info_present_flag = bit_reader.read_bit()?;
        if vui_timing_info_present_flag {
            let vui_num_units_in_tick = NonZero::new(bit_reader.read_u32::<BigEndian>()?).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "vui_num_units_in_tick must greater than zero",
            ))?;
            let vui_time_scale = NonZero::new(bit_reader.read_u32::<BigEndian>()?)
                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "vui_time_scale must not be zero"))?;

            let mut num_ticks_poc_diff_one_minus1 = None;
            let vui_poc_proportional_to_timing_flag = bit_reader.read_bit()?;
            if vui_poc_proportional_to_timing_flag {
                let vui_num_ticks_poc_diff_one_minus1 = bit_reader.read_exp_golomb()?;
                range_check!(vui_num_ticks_poc_diff_one_minus1, 0, 2u64.pow(32) - 2)?;
                num_ticks_poc_diff_one_minus1 = Some(vui_num_ticks_poc_diff_one_minus1 as u32);
            }

            let mut vui_hrd_parameters = None;
            let vui_hrd_parameters_present_flag = bit_reader.read_bit()?;
            if vui_hrd_parameters_present_flag {
                vui_hrd_parameters = Some(HrdParameters::parse(bit_reader, true, sps_max_sub_layers_minus1)?);
            }

            vui_timing_info = Some(VuiTimingInfo {
                num_units_in_tick: vui_num_units_in_tick,
                time_scale: vui_time_scale,
                poc_proportional_to_timing_flag: vui_poc_proportional_to_timing_flag,
                num_ticks_poc_diff_one_minus1,
                hrd_parameters: vui_hrd_parameters,
            });
        }

        let mut bitstream_restriction = BitStreamRestriction::default();
        let bitstream_restriction_flag = bit_reader.read_bit()?;
        if bitstream_restriction_flag {
            bitstream_restriction.tiles_fixed_structure_flag = bit_reader.read_bit()?;
            bitstream_restriction.motion_vectors_over_pic_boundaries_flag = bit_reader.read_bit()?;
            bitstream_restriction.restricted_ref_pic_lists_flag = Some(bit_reader.read_bit()?);

            let min_spatial_segmentation_idc = bit_reader.read_exp_golomb()?;
            range_check!(min_spatial_segmentation_idc, 0, 4095)?;
            bitstream_restriction.min_spatial_segmentation_idc = min_spatial_segmentation_idc as u16;

            let max_bytes_per_pic_denom = bit_reader.read_exp_golomb()?;
            range_check!(max_bytes_per_pic_denom, 0, 16)?;
            bitstream_restriction.max_bytes_per_pic_denom = max_bytes_per_pic_denom as u8;

            let max_bits_per_min_cu_denom = bit_reader.read_exp_golomb()?;
            range_check!(max_bits_per_min_cu_denom, 0, 16)?;
            bitstream_restriction.max_bits_per_min_cu_denom = max_bits_per_min_cu_denom as u8;

            let log2_max_mv_length_horizontal = bit_reader.read_exp_golomb()?;
            range_check!(log2_max_mv_length_horizontal, 0, 15)?;
            bitstream_restriction.log2_max_mv_length_horizontal = log2_max_mv_length_horizontal as u8;

            let log2_max_mv_length_vertical = bit_reader.read_exp_golomb()?;
            range_check!(log2_max_mv_length_vertical, 0, 15)?;
            bitstream_restriction.log2_max_mv_length_vertical = log2_max_mv_length_vertical as u8;
        }

        Ok(Self {
            aspect_ratio_info,
            overscan_appropriate_flag,
            video_signal_type: video_signal_type.unwrap_or_default(),
            chroma_loc_info,
            neutral_chroma_indication_flag,
            field_seq_flag,
            frame_field_info_present_flag,
            default_display_window: default_display_window.unwrap_or_default(),
            vui_timing_info,
            bitstream_restriction,
        })
    }
}

/// Specifies the value of the sample aspect ratio of the luma samples.
#[derive(Debug, Clone, PartialEq)]
pub enum AspectRatioInfo {
    /// Any value other than [`AspectRatioIdc::ExtendedSar`].
    Predefined(AspectRatioIdc),
    /// [`AspectRatioIdc::ExtendedSar`].
    ExtendedSar {
        /// Indicates the horizontal size of the sample aspect ratio (in arbitrary units).
        sar_width: u16,
        /// Indicates the vertical size of the sample aspect ratio (in the same arbitrary units as `sar_width`).
        sar_height: u16,
    },
}

/// Directly part of [`VuiParameters`].
#[derive(Debug, Clone, PartialEq)]
pub struct VideoSignalType {
    /// Indicates the representation of the pictures as specified in ISO/IEC 23008-2 - Table E.2, before being coded
    /// in accordance with this document.
    ///
    /// The values 6 and 7 for video_format are reserved for future use by ITU-T | ISO/IEC and
    /// shall not be present in bitstreams conforming to this version of this document.
    /// Decoders shall interpret the values 6 and 7 for video_format as equivalent to the value [`VideoFormat::Unspecified`].
    pub video_format: VideoFormat,
    /// Indicates the black level and range of the luma and chroma signals as derived from
    /// `E'Y`, `E'PB`, and `E'PR` or `E'R`, `E'G`, and `E'B` real-valued component signals.
    pub video_full_range_flag: bool,
    /// Indicates the chromaticity coordinates of the source primaries as specified in
    /// ISO/IEC 23008-2 - Table E.3 in terms of the CIE 1931 definition of x and y as specified in ISO 11664-1.
    pub colour_primaries: u8,
    /// As specified in ISO/IEC 23008-2 - Table E.4, either indicates the reference opto-electronic transfer
    /// characteristic function of the source picture as a function of a source input linear optical intensity `Lc` with
    /// a nominal real-valued range of 0 to 1 or indicates the inverse of the reference electro-optical transfer
    /// characteristic function as a function of an output linear optical intensity `Lo` with a nominal real-valued
    /// range of 0 to 1.
    ///
    /// For more details, see ISO/IEC 23008-2 - E.3.1.
    pub transfer_characteristics: u8,
    /// Describes the matrix coefficients used in deriving luma and chroma signals from the green,
    /// blue, and red, or Y, Z, and X primaries, as specified in ISO/IEC 23008-2 - Table E.5.
    ///
    /// Reported as coded. The constraints that ISO/IEC 23008-2 - E.3.1 places on the values 0 and 8
    /// (which depend on the chroma format and bit depths) are not enforced by this parser.
    pub matrix_coeffs: u8,
}

impl Default for VideoSignalType {
    fn default() -> Self {
        Self {
            video_format: VideoFormat::Unspecified,
            video_full_range_flag: false,
            colour_primaries: 2,
            transfer_characteristics: 2,
            matrix_coeffs: 2,
        }
    }
}

/// Directly part of [`VuiParameters`].
///
/// Specifies the location of chroma samples as follows:
/// - If [`chroma_format_idc`](crate::SpsRbsp::chroma_format_idc) is equal to 1 (4:2:0 chroma format),
///   [`chroma_sample_loc_type_top_field`](ChromaLocInfo::top_field) and
///   [`chroma_sample_loc_type_bottom_field`](ChromaLocInfo::bottom_field) specify the location of chroma samples
///   for the top field and the bottom field, respectively, as shown in ISO/IEC 23008-2 - Figure E.1.
/// - Otherwise ([`chroma_format_idc`](crate::SpsRbsp::chroma_format_idc) is not equal to 1), the values of the syntax elements
///   [`chroma_sample_loc_type_top_field`](ChromaLocInfo::top_field) and
///   [`chroma_sample_loc_type_bottom_field`](ChromaLocInfo::bottom_field) shall be ignored.
///   When [`chroma_format_idc`](crate::SpsRbsp::chroma_format_idc) is equal to 2 (4:2:2 chroma format) or 3 (4:4:4 chroma format),
///   the location of chroma samples is specified in ISO/IEC 23008-2 - 6.2.
///   When [`chroma_format_idc`](crate::SpsRbsp::chroma_format_idc) is equal to 0, there is no chroma sample array.
#[derive(Debug, Clone, PartialEq)]
pub struct ChromaLocInfo {
    /// `chroma_sample_loc_type_top_field`
    pub top_field: u64,
    /// `chroma_sample_loc_type_bottom_field`
    pub bottom_field: u64,
}

/// Directly part of [`VuiParameters`].
///
/// Specifies the samples of the pictures in the CVS that are within the default display window,
/// in terms of a rectangular region specified in picture coordinates for display.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DefaultDisplayWindow {
    /// `def_disp_win_left_offset`
    pub def_disp_win_left_offset: u64,
    /// `def_disp_win_right_offset`
    pub def_disp_win_right_offset: u64,
    /// `def_disp_win_top_offset`
    pub def_disp_win_top_offset: u64,
    /// `def_disp_win_bottom_offset`
    pub def_disp_win_bottom_offset: u64,
}

impl DefaultDisplayWindow {
    /// `leftOffset = conf_win_left_offset + def_disp_win_left_offset` (E-68)
    ///
    /// ISO/IEC 23008-2 - E.3.1
    pub fn left_offset(&self, conformance_window: &ConformanceWindow) -> u64 {
        conformance_window.conf_win_left_offset + self.def_disp_win_left_offset
    }

    /// `rightOffset = conf_win_right_offset + def_disp_win_right_offset` (E-69)
    ///
    /// ISO/IEC 23008-2 - E.3.1
    pub fn right_offset(&self, conformance_window: &ConformanceWindow) -> u64 {
        conformance_window.conf_win_right_offset + self.def_disp_win_right_offset
    }

    /// `topOffset = conf_win_top_offset + def_disp_win_top_offset` (E-70)
    ///
    /// ISO/IEC 23008-2 - E.3.1
    pub fn top_offset(&self, conformance_window: &ConformanceWindow) -> u64 {
        conformance_window.conf_win_top_offset + self.def_disp_win_top_offset
    }

    /// `bottomOffset = conf_win_bottom_offset + def_disp_win_bottom_offset` (E-71)
    ///
    /// ISO/IEC 23008-2 - E.3.1
    pub fn bottom_offset(&self, conformance_window: &ConformanceWindow) -> u64 {
        conformance_window.conf_win_bottom_offset + self.def_disp_win_bottom_offset
    }
}

/// Directly part of [`VuiParameters`].
#[derive(Debug, Clone, PartialEq)]
pub struct VuiTimingInfo {
    /// This value is the number of time units of a clock operating at the frequency `vui_time_scale`
    /// Hz that corresponds to one increment (called a clock tick) of a clock tick counter.
    ///
    /// This value is greater than 0.
    ///
    /// A clock tick, in units of seconds, is equal to the quotient of `vui_num_units_in_tick` divided by `vui_time_scale`.
    /// For example, when the picture rate of a video signal is 25 Hz, `vui_time_scale`
    /// may be equal to `27 000 000` and `vui_num_units_in_tick` may be equal to 1 080 000, and consequently a
    /// clock tick may be equal to `0.04` seconds.
    pub num_units_in_tick: NonZero<u32>,
    /// This value is the number of time units that pass in one second.
    ///
    /// For example, a time coordinate system that measures time using a `27 MHz` clock has a `vui_time_scale` of `27 000 000`.
    ///
    /// The value of `vui_time_scale` is greater than 0.
    pub time_scale: NonZero<u32>,
    /// equal to 1 indicates that the picture order count value for each
    /// picture in the CVS that is not the first picture in the CVS, in decoding order, is proportional to the output
    /// time of the picture relative to the output time of the first picture in the CVS.
    /// vui_poc_proportional_to_timing_flag equal to 0 indicates that the picture order count value for each
    /// picture in the CVS that is not the first picture in the CVS, in decoding order, may or may not be
    /// proportional to the output time of the picture relative to the output time of the first picture in the CVS.
    pub poc_proportional_to_timing_flag: bool,
    /// This value plus 1 specifies the number of clock ticks corresponding to a
    /// difference of picture order count values equal to 1.
    ///
    /// The value is in range \[0, 2^32 − 2\].
    pub num_ticks_poc_diff_one_minus1: Option<u32>,
    /// If `vui_hrd_parameters_present_flag` is equal to 1, this value specifies the HRD parameters.
    pub hrd_parameters: Option<HrdParameters>,
}

/// Directly part of [`VuiParameters`].
#[derive(Debug, Clone, PartialEq)]
pub struct BitStreamRestriction {
    /// Equal to `true` indicates that each PPS that is active in the CVS has the same value
    /// of the syntax elements `num_tile_columns_minus1`, `num_tile_rows_minus1`, `uniform_spacing_flag`,
    /// `column_width_minus1[i]`, `row_height_minus1[i]` and `loop_filter_across_tiles_enabled_flag`, when
    /// present.
    ///
    /// Equal to `false` indicates that tiles syntax elements in different PPSs may or
    /// may not have the same value.
    pub tiles_fixed_structure_flag: bool,
    /// Equal to `false` indicates that no sample outside the picture
    /// boundaries and no sample at a fractional sample position for which the sample value is derived using one
    /// or more samples outside the picture boundaries is used for inter prediction of any sample.
    ///
    /// Equal to `true` indicates that one or more samples outside the
    /// picture boundaries may be used in inter prediction.
    pub motion_vectors_over_pic_boundaries_flag: bool,
    /// Equal to `Some(true)` indicates that all P and B slices (when present) that belong to the
    /// same picture have an identical reference picture list 0, and that all B slices (when present) that belong to
    /// the same picture have an identical reference picture list 1.
    pub restricted_ref_pic_lists_flag: Option<bool>,
    /// When not equal to 0, establishes a bound on the maximum possible size
    /// of distinct coded spatial segmentation regions in the pictures of the CVS.
    ///
    /// The value is in range \[0, 4095\].
    ///
    /// Defines [`minSpatialSegmentationTimes4`](BitStreamRestriction::min_spatial_segmentation_times4).
    pub min_spatial_segmentation_idc: u16,
    /// Indicates a number of bytes not exceeded by the sum of the sizes of the VCL
    /// NAL units associated with any coded picture in the CVS.
    ///
    /// The number of bytes that represent a picture in the NAL unit stream is specified for this purpose as the
    /// total number of bytes of VCL NAL unit data (i.e. the total of the `NumBytesInNalUnit` variables for the VCL
    /// NAL units) for the picture.
    ///
    /// The value is in range \[0, 16\].
    pub max_bytes_per_pic_denom: u8,
    /// Indicates an upper bound for the number of coded bits of `coding_unit()`
    /// data for any coding block in any picture of the CVS.
    ///
    /// The value is in range \[0, 16\].
    pub max_bits_per_min_cu_denom: u8,
    /// Indicates the maximum absolute
    /// value of a decoded horizontal and vertical motion vector component, respectively, in quarter luma sample
    /// units, for all pictures in the CVS. A value of n asserts that no value of a motion vector component is outside
    /// the range of \[`−2n`, `2n − 1`\], in units of quarter luma sample displacement, where `n` refers to the
    /// value of [`log2_max_mv_length_horizontal`](BitStreamRestriction::log2_max_mv_length_horizontal) and
    /// [`log2_max_mv_length_vertical`](BitStreamRestriction::log2_max_mv_length_vertical) for the horizontal and
    /// vertical component of the MV, respectively.
    ///
    /// The value is in range \[0, 15\].
    pub log2_max_mv_length_horizontal: u8,
    /// Same as [`log2_max_mv_length_horizontal`](BitStreamRestriction::log2_max_mv_length_horizontal).
    pub log2_max_mv_length_vertical: u8,
}

impl Default for BitStreamRestriction {
    fn default() -> Self {
        Self {
            tiles_fixed_structure_flag: false,
            motion_vectors_over_pic_boundaries_flag: true,
            restricted_ref_pic_lists_flag: None,
            min_spatial_segmentation_idc: 0,
            max_bytes_per_pic_denom: 2,
            max_bits_per_min_cu_denom: 1,
            log2_max_mv_length_horizontal: 15,
            log2_max_mv_length_vertical: 15,
        }
    }
}

impl BitStreamRestriction {
    /// `minSpatialSegmentationTimes4 = min_spatial_segmentation_idc + 4` (E-72)
    ///
    /// ISO/IEC 23008-2 - E.3.1
    pub fn min_spatial_segmentation_times4(&self) -> u16 {
        self.min_spatial_segmentation_idc + 4
    }
}

#[cfg(test)]
#[cfg_attr(all(test, coverage_nightly), coverage(off))]
mod tests {
    use std::io::Write;
    use std::num::NonZero;

    use byteorder::{BigEndian, WriteBytesExt};
    use scuffle_bytes_util::{BitReader, BitWriter};
    use scuffle_expgolomb::BitWriterExpGolombExt;

    use crate::sps::vui_parameters::{BitStreamRestriction, DefaultDisplayWindow};
    use crate::{AspectRatioIdc, ConformanceWindow, Profile, ProfileCompatibilityFlags, VideoFormat, VuiParameters};

    #[test]
    fn vui_parameters() {
        let mut data = Vec::new();
        let mut writer = BitWriter::new(&mut data);

        writer.write_bit(true).unwrap(); // aspect_ratio_info_present_flag
        writer.write_u8(AspectRatioIdc::ExtendedSar.0).unwrap(); // aspect_ratio_idc
        writer.write_u16::<BigEndian>(1).unwrap(); // sar_width
        writer.write_u16::<BigEndian>(1).unwrap(); // sar_height

        writer.write_bit(true).unwrap(); // overscan_info_present_flag
        writer.write_bit(true).unwrap(); // overscan_appropriate_flag

        writer.write_bit(false).unwrap(); // video_signal_type_present_flag
        writer.write_bit(false).unwrap(); // chroma_loc_info_present_flag
        writer.write_bit(false).unwrap(); // neutral_chroma_indication_flag
        writer.write_bit(false).unwrap(); // field_seq_flag
        writer.write_bit(false).unwrap(); // frame_field_info_present_flag

        writer.write_bit(true).unwrap(); // default_display_window_flag
        writer.write_exp_golomb(0).unwrap(); // def_disp_win_left_offset
        writer.write_exp_golomb(10).unwrap(); // def_disp_win_right_offset
        writer.write_exp_golomb(0).unwrap(); // def_disp_win_top_offset
        writer.write_exp_golomb(10).unwrap(); // def_disp_win_bottom_offset

        writer.write_bit(false).unwrap(); // vui_timing_info_present_flag
        writer.write_bit(false).unwrap(); // bitstream_restriction_flag

        writer.write_bits(0, 5).unwrap(); // fill the byte
        writer.flush().unwrap();

        let conf_window = ConformanceWindow {
            conf_win_left_offset: 2,
            conf_win_right_offset: 2,
            conf_win_top_offset: 2,
            conf_win_bottom_offset: 2,
        };

        let vui_parameters = VuiParameters::parse(
            &mut BitReader::new(data.as_slice()),
            0,
            1,
            &Profile {
                profile_space: 0,
                tier_flag: false,
                profile_idc: 0,
                profile_compatibility_flag: ProfileCompatibilityFlags::empty(),
                progressive_source_flag: false,
                interlaced_source_flag: false,
                non_packed_constraint_flag: false,
                frame_only_constraint_flag: false,
                additional_flags: crate::ProfileAdditionalFlags::None,
                inbld_flag: None,
                level_idc: Some(0),
            },
            &conf_window,
            1,
            NonZero::new(1920).unwrap(),
            1,
            NonZero::new(1080).unwrap(),
        )
        .unwrap();

        assert_eq!(
            vui_parameters.aspect_ratio_info,
            super::AspectRatioInfo::ExtendedSar {
                sar_width: 1,
                sar_height: 1
            }
        );
        assert_eq!(vui_parameters.overscan_appropriate_flag, Some(true));
        assert_eq!(vui_parameters.video_signal_type.video_format, VideoFormat::Unspecified);
        assert!(!vui_parameters.video_signal_type.video_full_range_flag);
        assert_eq!(vui_parameters.video_signal_type.colour_primaries, 2);
        assert_eq!(vui_parameters.video_signal_type.transfer_characteristics, 2);
        assert_eq!(vui_parameters.video_signal_type.matrix_coeffs, 2);
        assert_eq!(vui_parameters.chroma_loc_info, None);
        assert!(!vui_parameters.neutral_chroma_indication_flag);
        assert!(!vui_parameters.field_seq_flag);
        assert!(!vui_parameters.frame_field_info_present_flag);
        assert_eq!(
            vui_parameters.default_display_window,
            DefaultDisplayWindow {
                def_disp_win_left_offset: 0,
                def_disp_win_right_offset: 10,
                def_disp_win_top_offset: 0,
                def_disp_win_bottom_offset: 10,
            }
        );
        assert_eq!(vui_parameters.default_display_window.left_offset(&conf_window), 2);
        assert_eq!(vui_parameters.default_display_window.right_offset(&conf_window), 12);
        assert_eq!(vui_parameters.default_display_window.top_offset(&conf_window), 2);
        assert_eq!(vui_parameters.default_display_window.bottom_offset(&conf_window), 12);
        assert_eq!(vui_parameters.vui_timing_info, None);
        assert_eq!(vui_parameters.bitstream_restriction, BitStreamRestriction::default());
        assert_eq!(vui_parameters.bitstream_restriction.min_spatial_segmentation_times4(), 4);
    }
}
