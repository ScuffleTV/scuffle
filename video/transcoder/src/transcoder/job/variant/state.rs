use std::collections::HashMap;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct PlaylistState {
    mutations: HashMap<String, String>,
    current_segment_idx: u32,
    current_fragment_idx: u32,
    discontinuity_sequence: u32,
    sequence_number: u32,
    tracks: Vec<Track>,
    playlist: String,
    longest_segment: f64,
}

impl Default for PlaylistState {
    fn default() -> Self {
        Self {
            mutations: HashMap::from_iter(vec![
                ("current_segment_idx".to_string(), "0".to_string()),
                ("current_fragment_idx".to_string(), "0".to_string()),
                ("discontinuity_sequence".to_string(), "0".to_string()),
                ("sequence_number".to_string(), "0".to_string()),
                ("longest_segment".to_string(), "0.0".to_string()),
                ("track_count".to_string(), "0".to_string()),
                ("playlist".to_string(), String::new()),
            ]),
            current_segment_idx: 0,
            current_fragment_idx: 0,
            discontinuity_sequence: 0,
            sequence_number: 0,
            tracks: Vec::new(),
            longest_segment: 0.0,
            playlist: String::new(),
        }
    }
}

impl PlaylistState {
    pub fn set_current_segment_idx(&mut self, value: u32) {
        if value != self.current_fragment_idx {
            self.mutations
                .insert("current_segment_idx".to_string(), value.to_string());
            self.current_segment_idx = value;
        }
    }

    pub fn set_current_fragment_idx(&mut self, value: u32) {
        if value != self.current_fragment_idx {
            self.mutations
                .insert("current_fragment_idx".to_string(), value.to_string());
            self.current_fragment_idx = value;
        }
    }

    pub fn set_discontinuity_sequence(&mut self, value: u32) {
        if value != self.discontinuity_sequence {
            self.mutations
                .insert("discontinuity_sequence".to_string(), value.to_string());
            self.discontinuity_sequence = value;
        }
    }

    pub fn set_sequence_number(&mut self, value: u32) {
        if value != self.sequence_number {
            self.mutations
                .insert("sequence_number".to_string(), value.to_string());
            self.sequence_number = value;
        }
    }

    pub fn insert_track(&mut self, track: Track) {
        self.mutations.insert(
            format!("track_{}_duration", self.tracks.len() + 1),
            track.duration.to_string(),
        );
        self.mutations.insert(
            format!("track_{}_timescale", self.tracks.len() + 1),
            track.timescale.to_string(),
        );
        self.mutations
            .insert("track_count".into(), (self.tracks.len() + 1).to_string());

        self.tracks.push(track);
    }

    pub fn set_longest_segment(&mut self, value: f64) {
        if value != self.longest_segment {
            self.mutations
                .insert("longest_segment".to_string(), value.to_string());
            self.longest_segment = value;
        }
    }

    pub fn set_track_duration(&mut self, track_idx: usize, value: u32) {
        if let Some(track) = self.tracks.get_mut(track_idx) {
            if value != track.duration {
                self.mutations.insert(
                    format!("track_{}_duration", track_idx + 1),
                    value.to_string(),
                );
                track.duration = value;
            }
        }
    }

    pub fn set_playlist(&mut self, value: String) {
        if value != self.playlist {
            self.mutations.insert("playlist".to_string(), value.clone());
            self.playlist = value;
        }
    }

    #[inline(always)]
    pub fn current_segment_idx(&self) -> u32 {
        self.current_segment_idx
    }

    #[inline(always)]
    pub fn current_fragment_idx(&self) -> u32 {
        self.current_fragment_idx
    }

    #[inline(always)]
    pub fn discontinuity_sequence(&self) -> u32 {
        self.discontinuity_sequence
    }

    #[inline(always)]
    pub fn sequence_number(&self) -> u32 {
        self.sequence_number
    }

    #[inline(always)]
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    #[inline(always)]
    pub fn track_duration(&self, track_idx: usize) -> Option<u32> {
        self.tracks.get(track_idx).map(|t| t.duration)
    }

    #[inline(always)]
    pub fn track_timescale(&self, track_idx: usize) -> Option<u32> {
        self.tracks.get(track_idx).map(|t| t.timescale)
    }

    #[inline(always)]
    pub fn longest_segment(&self) -> f64 {
        self.longest_segment
    }

    pub fn extract_mutations(&mut self) -> HashMap<String, String> {
        std::mem::take(&mut self.mutations)
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    pub duration: u32,
    pub timescale: u32,
}

impl From<HashMap<String, String>> for PlaylistState {
    fn from(value: HashMap<String, String>) -> Self {
        let mut mutations = HashMap::new();

        let current_segment_idx = value
            .get("current_segment_idx")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or_else(|| {
                mutations.insert("current_segment_idx".to_string(), "0".to_string());
                0
            });

        let current_fragment_idx = value
            .get("current_fragment_idx")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or_else(|| {
                mutations.insert("current_fragment_idx".to_string(), "0".to_string());
                0
            });

        let discontinuity_sequence = value
            .get("discontinuity_sequence")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or_else(|| {
                mutations.insert("discontinuity_sequence".to_string(), "0".to_string());
                0
            });

        let track_count = value
            .get("track_count")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or_else(|| {
                mutations.insert("track_count".to_string(), "0".to_string());
                0
            });

        let sequence_number = value
            .get("sequence_number")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or_else(|| {
                mutations.insert("sequence_number".to_string(), "0".to_string());
                0
            });

        let playlist = value
            .get("playlist")
            .map(|v| v.to_string())
            .unwrap_or_else(|| {
                mutations.insert("playlist".to_string(), "".to_string());
                "".to_string()
            });

        let mut tracks = Vec::with_capacity(track_count);

        for i in 0..track_count {
            let duration = value
                .get(&format!("track_{}_duration", i + 1))
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or_else(|| {
                    mutations.insert(format!("track_{}_duration", i + 1), "0".to_string());
                    0
                });

            let timescale = value
                .get(&format!("track_{}_timescale", i + 1))
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or_else(|| {
                    mutations.insert(format!("track_{}_timescale", i + 1), "0".to_string());
                    0
                });

            tracks.push(Track {
                duration,
                timescale,
            });
        }

        let longest_segment = value
            .get("longest_segment")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or_default();

        Self {
            mutations,
            current_segment_idx,
            current_fragment_idx,
            discontinuity_sequence,
            tracks,
            longest_segment,
            sequence_number,
            playlist,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SegmentState {
    mutations: HashMap<String, String>,
    ready: bool,
    discontinuity: bool,
    timestamp: DateTime<Utc>,
    fragments: Vec<Fragment>,
}

#[derive(Debug, Clone)]
pub struct Fragment {
    pub duration: u32,
    pub keyframe: bool,
}

impl Default for SegmentState {
    fn default() -> Self {
        Self {
            mutations: HashMap::from_iter(vec![
                ("ready".into(), "false".into()),
                ("discontinuity".into(), "false".into()),
                ("timestamp".into(), Utc::now().to_rfc3339()),
                ("fragment_count".into(), "0".into()),
            ]),
            ready: false,
            discontinuity: false,
            timestamp: Utc::now(),
            fragments: Vec::new(),
        }
    }
}

impl From<HashMap<String, String>> for SegmentState {
    fn from(value: HashMap<String, String>) -> Self {
        let mut mutations = HashMap::new();

        let ready = value
            .get("ready")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or_else(|| {
                mutations.insert("ready".into(), "false".into());
                false
            });

        let discontinuity = value
            .get("discontinuity")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or_else(|| {
                mutations.insert("discontinuity".into(), "false".into());
                false
            });
        let timestamp = value
            .get("timestamp")
            .and_then(|v| {
                DateTime::parse_from_rfc3339(v)
                    .map(|t| t.with_timezone(&Utc))
                    .ok()
            })
            .unwrap_or_else(|| {
                let now = Utc::now();
                mutations.insert("timestamp".into(), now.to_rfc3339());
                now
            });
        let fragment_count = value
            .get("fragment_count")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or_else(|| {
                mutations.insert("fragment_count".into(), "0".into());
                0
            });

        let mut fragments = Vec::with_capacity(fragment_count);
        for i in 0..fragment_count {
            let duration = value
                .get(&format!("fragment_{}_duration", i))
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or_else(|| {
                    mutations.insert(format!("fragment_{}_duration", i), "0".into());
                    0
                });
            let keyframe = value
                .get(&format!("fragment_{}_keyframe", i))
                .and_then(|v| v.parse::<bool>().ok())
                .unwrap_or_else(|| {
                    mutations.insert(format!("fragment_{}_keyframe", i), "false".into());
                    false
                });
            fragments.push(Fragment { duration, keyframe });
        }

        Self {
            mutations,
            ready,
            discontinuity,
            timestamp,
            fragments,
        }
    }
}

impl SegmentState {
    #[inline(always)]
    pub fn ready(&self) -> bool {
        self.ready
    }

    #[inline(always)]
    pub fn discontinuity(&self) -> bool {
        self.discontinuity
    }

    #[inline(always)]
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    #[inline(always)]
    pub fn fragments(&self) -> &[Fragment] {
        &self.fragments
    }

    pub fn set_ready(&mut self, ready: bool) {
        self.mutations.insert("ready".into(), ready.to_string());
        self.ready = ready;
    }

    pub fn set_discontinuity(&mut self, discontinuity: bool) {
        self.mutations
            .insert("discontinuity".into(), discontinuity.to_string());
        self.discontinuity = discontinuity;
    }

    pub fn insert_fragment(&mut self, fragment: Fragment) {
        let idx = self.fragments.len();
        self.mutations.insert(
            format!("fragment_{}_duration", idx),
            fragment.duration.to_string(),
        );
        self.mutations.insert(
            format!("fragment_{}_keyframe", idx),
            fragment.keyframe.to_string(),
        );
        self.mutations
            .insert("fragment_count".into(), (idx + 1).to_string());
        self.fragments.push(fragment);
    }

    pub fn extract_mutations(&mut self) -> HashMap<String, String> {
        std::mem::take(&mut self.mutations)
    }
}
