use std::io;

use byteorder::{BigEndian, ReadBytesExt};
use scuffle_bytes_util::{BitReader, range_check};

use crate::ProfileCompatibilityFlags;

/// Profile, tier and level.
///
/// `profile_tier_level(profilePresentFlag, maxNumSubLayersMinus1)`
///
/// - ISO/IEC 23008-2 - 7.3.3
/// - ISO/IEC 23008-2 - 7.4.4
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileTierLevel {
    /// `general_profile_space`, `general_tier_flag`, `general_profile_idc`, `general_profile_compatibility_flag[j]`,
    /// `general_progressive_source_flag`, `general_interlaced_source_flag`, `general_non_packed_constraint_flag`,
    /// `general_frame_only_constraint_flag`, `general_max_12bit_constraint_flag`, `general_max_10bit_constraint_flag`,
    /// `general_max_8bit_constraint_flag`, `general_max_422chroma_constraint_flag`,
    /// `general_max_420chroma_constraint_flag`, `general_max_monochrome_constraint_flag`,
    /// `general_intra_constraint_flag`, `general_one_picture_only_constraint_flag`,
    /// `general_lower_bit_rate_constraint_flag`, `general_max_14bit_constraint_flag`, `general_inbld_flag`
    /// and `general_level_idc`.
    pub general_profile: Profile,
    /// `sub_layer_profile_space[i]`, `sub_layer_tier_flag[i]`,
    /// `sub_layer_profile_idc[i]`,
    /// `sub_layer_profile_compatibility_flag[i][j]`,
    /// `sub_layer_progressive_source_flag[i]`,
    /// `sub_layer_interlaced_source_flag[i]`,
    /// `sub_layer_non_packed_constraint_flag[i]`,
    /// `sub_layer_frame_only_constraint_flag[i]`,
    /// `sub_layer_max_12bit_constraint_flag[i]`,
    /// `sub_layer_max_10bit_constraint_flag[i]`,
    /// `sub_layer_max_8bit_constraint_flag[i]`,
    /// `sub_layer_max_422chroma_constraint_flag[i]`,
    /// `sub_layer_max_420chroma_constraint_flag[i]`,
    /// `sub_layer_max_monochrome_constraint_flag[i]`,
    /// `sub_layer_intra_constraint_flag[i]`,
    /// `sub_layer_one_picture_only_constraint_flag[i]`,
    /// `sub_layer_lower_bit_rate_constraint_flag[i]`,
    /// `sub_layer_max_14bit_constraint_flag[i]`,
    /// `sub_layer_inbld_flag[i]`, and
    /// `sub_layer_level_idc[i]`.
    pub sub_layer_profiles: Vec<Profile>,
}

impl ProfileTierLevel {
    pub(crate) fn parse<R: io::Read>(bit_reader: &mut BitReader<R>, max_num_sub_layers_minus_1: u8) -> io::Result<Self> {
        // When parsing SPSs, the profile_present_flag is always true. (See 7.3.2.2.1)
        // Since this decoder only supports SPS decoding, it is assumed to be true here.

        let mut general_profile = Profile::parse(bit_reader, true)?;
        // inbld_flag is inferred to be 0 when not present for the genral profile
        general_profile.inbld_flag = Some(general_profile.inbld_flag.unwrap_or(false));

        let mut sub_layer_profile_present_flags = Vec::with_capacity(max_num_sub_layers_minus_1 as usize);
        let mut sub_layer_level_present_flags = Vec::with_capacity(max_num_sub_layers_minus_1 as usize);
        for _ in 0..max_num_sub_layers_minus_1 {
            sub_layer_profile_present_flags.push(bit_reader.read_bit()?); // sub_layer_profile_present_flag
            sub_layer_level_present_flags.push(bit_reader.read_bit()?); // sub_layer_level_present_flag
        }

        // reserved_zero_2bits
        if max_num_sub_layers_minus_1 > 0 && max_num_sub_layers_minus_1 < 8 {
            bit_reader.read_bits(2 * (8 - max_num_sub_layers_minus_1))?;
        }

        let mut sub_layer_profiles = vec![None; max_num_sub_layers_minus_1 as usize];
        let mut sub_layer_level_idcs = vec![None; max_num_sub_layers_minus_1 as usize];

        for i in 0..max_num_sub_layers_minus_1 as usize {
            if sub_layer_profile_present_flags[i] {
                // Also reads sub_layer_level_idc when sub_layer_level_present_flag is set.
                sub_layer_profiles[i] = Some(Profile::parse(bit_reader, sub_layer_level_present_flags[i])?);
            } else if sub_layer_level_present_flags[i] {
                sub_layer_level_idcs[i] = Some(bit_reader.read_u8()?);
            }
        }

        let mut last_profile = general_profile.clone();
        let mut sub_layer_profiles: Vec<_> = sub_layer_profiles
            .into_iter()
            .zip(sub_layer_level_idcs)
            .rev()
            .map(|(profile, level_idc)| match profile {
                Some(profile) => {
                    let profile = profile.merge(&last_profile);
                    last_profile = profile.clone();
                    profile
                }
                None => {
                    // Only the level can be present here; the rest is inferred from the next higher sub-layer.
                    last_profile.level_idc = level_idc.or(last_profile.level_idc);
                    last_profile.clone()
                }
            })
            .collect();
        sub_layer_profiles.reverse(); // reverse back to original order

        Ok(ProfileTierLevel {
            general_profile,
            sub_layer_profiles,
        })
    }
}

/// Profile part of the Profile, tier and level structure.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// Decoders shall ignore the CVS when `general_profile_space` is not equal to 0.
    pub profile_space: u8,
    /// Specifies the tier context for the interpretation of `general_level_idc` as specified in ISO/IEC 23008-2 - Annex A.
    pub tier_flag: bool,
    /// When `general_profile_space` is equal to 0, indicates a profile to which the CVS
    /// conforms as specified in ISO/IEC 23008-2 - Annex A.
    pub profile_idc: u8,
    /// `profile_compatibility_flag[j]` equal to `true`, when `general_profile_space` is equal to 0, indicates
    /// that the CVS conforms to the profile indicated by `general_profile_idc` equal to `j`
    /// as specified in ISO/IEC 23008-2 - Annex A.
    pub profile_compatibility_flag: ProfileCompatibilityFlags,
    /// - If `general_progressive_source_flag` is equal to `true` and
    ///   [`general_interlaced_source_flag`](Profile::interlaced_source_flag) is equal to `false`, the
    ///   source scan type of the pictures in the CVS should be interpreted as progressive only.
    /// - Otherwise, if `general_progressive_source_flag` is equal to `false` and
    ///   [`general_interlaced_source_flag`](Profile::interlaced_source_flag) is equal to `true`, the
    ///   source scan type of the pictures in the CVS should be interpreted as interlaced only.
    /// - Otherwise, if `general_progressive_source_flag` is equal to `false` and
    ///   [`general_interlaced_source_flag`](Profile::interlaced_source_flag) is equal to `false`, the
    ///   source scan type of the pictures in the CVS should be interpreted as unknown or
    ///   unspecified.
    /// - Otherwise (`general_progressive_source_flag` is equal to `true` and
    ///   [`general_interlaced_source_flag`](Profile::interlaced_source_flag) is equal to `true`),
    ///   the source scan type of each picture in the CVS is indicated at the picture level using the syntax
    ///   element `source_scan_type` in a picture timing SEI message.
    pub progressive_source_flag: bool,
    /// See [`progressive_source_flag`](Profile::progressive_source_flag).
    pub interlaced_source_flag: bool,
    /// Equal to `true` specifies that there are no frame packing arrangement
    /// SEI messages, segmented rectangular frame packing arrangement SEI messages, equirectangular
    /// projection SEI messages, or cubemap projection SEI messages present in the CVS.
    ///
    /// Equal to `false` indicates that there may or may not be one or more frame
    /// packing arrangement SEI messages, segmented rectangular frame packing arrangement SEI messages,
    /// equirectangular projection SEI messages, or cubemap projection SEI messages present in the CVS.
    pub non_packed_constraint_flag: bool,
    /// Equal to `true` specifies that `field_seq_flag` is equal to 0.
    ///
    /// Equal to `false` indicates that `field_seq_flag` may or may not be equal to 0.
    pub frame_only_constraint_flag: bool,
    /// Any additional flags that may be present in the profile.
    pub additional_flags: ProfileAdditionalFlags,
    /// Equal to `true` specifies that the INBLD capability as specified in ISO/IEC 23008-2 - Annex F is required for
    /// decoding of the layer to which the `profile_tier_level( )` syntax structure applies.
    ///
    /// Equal to `false` specifies that the INBLD capability as specified in ISO/IEC 23008-2 - Annex F is not required for
    /// decoding of the layer to which the profile_tier_level( ) syntax structure applies.
    pub inbld_flag: Option<bool>,
    /// Indicates a level to which the CVS conforms as specified in ISO/IEC 23008-2 - Annex A.
    ///
    /// Always present for the general profile.
    pub level_idc: Option<u8>,
}

impl Profile {
    fn parse<R: io::Read>(bit_reader: &mut BitReader<R>, level_present: bool) -> io::Result<Self> {
        let profile_space = bit_reader.read_bits(2)? as u8;
        let tier_flag = bit_reader.read_bit()?;
        let profile_idc = bit_reader.read_bits(5)? as u8;

        let profile_compatibility_flag = ProfileCompatibilityFlags::from_bits_retain(bit_reader.read_u32::<BigEndian>()?);

        let check_profile_idcs = |profiles: ProfileCompatibilityFlags| {
            profiles.contains(ProfileCompatibilityFlags::from_bits_retain(1 << profile_idc))
                || profile_compatibility_flag.intersects(profiles)
        };

        let progressive_source_flag = bit_reader.read_bit()?;
        let interlaced_source_flag = bit_reader.read_bit()?;
        let non_packed_constraint_flag = bit_reader.read_bit()?;
        let frame_only_constraint_flag = bit_reader.read_bit()?;

        let additional_flags = if check_profile_idcs(
            ProfileCompatibilityFlags::FormatRangeExtensionsProfile
                | ProfileCompatibilityFlags::HighThroughputProfile
                | ProfileCompatibilityFlags::Profile6
                | ProfileCompatibilityFlags::Profile7
                | ProfileCompatibilityFlags::Profile8
                | ProfileCompatibilityFlags::ScreenContentCodingExtensionsProfile
                | ProfileCompatibilityFlags::Profile10
                | ProfileCompatibilityFlags::HighThroughputScreenContentCodingExtensionsProfile,
        ) {
            let max_12bit_constraint_flag = bit_reader.read_bit()?;
            let max_10bit_constraint_flag = bit_reader.read_bit()?;
            let max_8bit_constraint_flag = bit_reader.read_bit()?;
            let max_422chroma_constraint_flag = bit_reader.read_bit()?;
            let max_420chroma_constraint_flag = bit_reader.read_bit()?;
            let max_monochrome_constraint_flag = bit_reader.read_bit()?;
            let intra_constraint_flag = bit_reader.read_bit()?;
            let one_picture_only_constraint_flag = bit_reader.read_bit()?;
            let lower_bit_rate_constraint_flag = bit_reader.read_bit()?;

            let max_14bit_constraint_flag = if check_profile_idcs(
                ProfileCompatibilityFlags::HighThroughputProfile
                    | ProfileCompatibilityFlags::ScreenContentCodingExtensionsProfile
                    | ProfileCompatibilityFlags::Profile10
                    | ProfileCompatibilityFlags::HighThroughputScreenContentCodingExtensionsProfile,
            ) {
                let max_14bit_constraint_flag = bit_reader.read_bit()?;
                bit_reader.read_bits(33)?;
                Some(max_14bit_constraint_flag)
            } else {
                bit_reader.read_bits(34)?;
                None
            };

            ProfileAdditionalFlags::Full {
                max_12bit_constraint_flag,
                max_10bit_constraint_flag,
                max_8bit_constraint_flag,
                max_422chroma_constraint_flag,
                max_420chroma_constraint_flag,
                max_monochrome_constraint_flag,
                intra_constraint_flag,
                one_picture_only_constraint_flag,
                lower_bit_rate_constraint_flag,
                max_14bit_constraint_flag,
            }
        } else if check_profile_idcs(ProfileCompatibilityFlags::Main10Profile) {
            bit_reader.read_bits(7)?; // reserved_zero_7bits
            let one_picture_only_constraint_flag = bit_reader.read_bit()?;
            bit_reader.read_bits(35)?; // reserved_zero_35bits
            ProfileAdditionalFlags::Main10Profile {
                one_picture_only_constraint_flag,
            }
        } else {
            bit_reader.read_bits(43)?; // reserved_zero_43bits
            ProfileAdditionalFlags::None
        };

        let inbld_flag = if check_profile_idcs(
            ProfileCompatibilityFlags::MainProfile
                | ProfileCompatibilityFlags::Main10Profile
                | ProfileCompatibilityFlags::MainStillPictureProfile
                | ProfileCompatibilityFlags::FormatRangeExtensionsProfile
                | ProfileCompatibilityFlags::HighThroughputProfile
                | ProfileCompatibilityFlags::ScreenContentCodingExtensionsProfile
                | ProfileCompatibilityFlags::HighThroughputScreenContentCodingExtensionsProfile,
        ) {
            Some(bit_reader.read_bit()?)
        } else {
            bit_reader.read_bit()?; // reserved_zero_bit
            None
        };

        let mut level_idc_value = None;
        if level_present {
            let level_idc = bit_reader.read_bits(8)? as u8;
            range_check!(level_idc, 0, 254)?;
            level_idc_value = Some(level_idc);
        }

        Ok(Profile {
            profile_space,
            tier_flag,
            profile_idc,
            profile_compatibility_flag,
            progressive_source_flag,
            interlaced_source_flag,
            non_packed_constraint_flag,
            frame_only_constraint_flag,
            additional_flags,
            inbld_flag,
            level_idc: level_idc_value,
        })
    }

    fn merge(self, defaults: &Self) -> Self {
        Self {
            additional_flags: self.additional_flags.merge(&defaults.additional_flags),
            inbld_flag: self.inbld_flag.or(defaults.inbld_flag),
            level_idc: self.level_idc.or(defaults.level_idc),
            ..self
        }
    }
}

/// Additional profile flags that can be present in the [profile](Profile).
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileAdditionalFlags {
    /// All additional flags are present.
    Full {
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        max_12bit_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        max_10bit_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        max_8bit_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        max_422chroma_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        max_420chroma_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        max_monochrome_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        intra_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        one_picture_only_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        lower_bit_rate_constraint_flag: bool,
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        max_14bit_constraint_flag: Option<bool>,
    },
    /// Only the `one_picture_only_constraint_flag` is present because `profile_idc` is 2 or `general_profile_compatibility_flag[2]` is `true`.
    Main10Profile {
        /// Semantics specified in ISO/IEC 23008-2 - Annex A.
        one_picture_only_constraint_flag: bool,
    },
    /// No additional flags are present.
    None,
}

impl ProfileAdditionalFlags {
    fn merge(self, defaults: &Self) -> Self {
        match (&self, defaults) {
            (Self::Full { .. }, _) => self,
            (
                Self::Main10Profile {
                    one_picture_only_constraint_flag,
                },
                Self::Full {
                    max_12bit_constraint_flag,
                    max_10bit_constraint_flag,
                    max_8bit_constraint_flag,
                    max_422chroma_constraint_flag,
                    max_420chroma_constraint_flag,
                    max_monochrome_constraint_flag,
                    intra_constraint_flag,
                    lower_bit_rate_constraint_flag,
                    max_14bit_constraint_flag,
                    ..
                },
            ) => Self::Full {
                max_12bit_constraint_flag: *max_12bit_constraint_flag,
                max_10bit_constraint_flag: *max_10bit_constraint_flag,
                max_8bit_constraint_flag: *max_8bit_constraint_flag,
                max_422chroma_constraint_flag: *max_422chroma_constraint_flag,
                max_420chroma_constraint_flag: *max_420chroma_constraint_flag,
                max_monochrome_constraint_flag: *max_monochrome_constraint_flag,
                intra_constraint_flag: *intra_constraint_flag,
                one_picture_only_constraint_flag: *one_picture_only_constraint_flag,
                lower_bit_rate_constraint_flag: *lower_bit_rate_constraint_flag,
                max_14bit_constraint_flag: *max_14bit_constraint_flag,
            },
            (Self::Main10Profile { .. }, _) => self,
            (Self::None, _) => defaults.clone(),
        }
    }
}
