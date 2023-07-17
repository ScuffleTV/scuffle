use wasm_bindgen_test::*;

use crate::hls::{
    self,
    master::{Media, MediaType, ScufGroup, Stream},
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn parse_hls_master() {
    const HLS_MASTER: &str = include_str!("./data/master_multi_transcode.m3u8");

    let playlist = HLS_MASTER.parse::<hls::Playlist>().unwrap();

    let master = match playlist {
        hls::Playlist::Master(master) => master,
        _ => panic!("Expected master playlist"),
    };

    assert!(master.groups.len() == 6);
    assert!(master.scuf_groups.len() == 2);
    assert!(master.streams.len() == 10);

    assert_eq!(
        master.scuf_groups.get("opus"),
        Some(&ScufGroup { priority: 1 })
    );
    assert_eq!(
        master.scuf_groups.get("aac"),
        Some(&ScufGroup { priority: 2 })
    );

    assert_eq!(
        master.groups.get("043897a5-cda1-458b-84d6-ce7a879a6a1e"),
        Some(&vec![Media {
            autoselect: true,
            media_type: MediaType::Audio,
            bandwidth: 98304,
            codecs: "opus".to_string(),
            group_id: "043897a5-cda1-458b-84d6-ce7a879a6a1e".to_string(),
            default: true,
            forced: false,
            frame_rate: None,
            name: "043897a5-cda1-458b-84d6-ce7a879a6a1e".to_string(),
            resolution: None,
            uri: "043897a5-cda1-458b-84d6-ce7a879a6a1e/index.m3u8".to_string(),
        }])
    );

    assert_eq!(
        master.groups.get("19c0a428-0925-40f7-ac28-979905df98a1"),
        Some(&vec![Media {
            autoselect: true,
            media_type: MediaType::Audio,
            bandwidth: 131072,
            codecs: "mp4a.40.2".to_string(),
            group_id: "19c0a428-0925-40f7-ac28-979905df98a1".to_string(),
            default: true,
            forced: false,
            frame_rate: None,
            name: "19c0a428-0925-40f7-ac28-979905df98a1".to_string(),
            resolution: None,
            uri: "19c0a428-0925-40f7-ac28-979905df98a1/index.m3u8".to_string(),
        }])
    );

    assert_eq!(
        master.groups.get("541cf75a-ba88-4c91-b4a5-9023d7213f5d"),
        Some(&vec![Media {
            autoselect: true,
            media_type: MediaType::Video,
            bandwidth: 8192000,
            codecs: "av01.0.13M.08.0.110.01.01.01.0".to_string(),
            group_id: "541cf75a-ba88-4c91-b4a5-9023d7213f5d".to_string(),
            default: true,
            forced: false,
            frame_rate: Some(60.0),
            name: "541cf75a-ba88-4c91-b4a5-9023d7213f5d".to_string(),
            resolution: Some((3840, 2160)),
            uri: "541cf75a-ba88-4c91-b4a5-9023d7213f5d/index.m3u8".to_string(),
        }])
    );

    assert_eq!(
        master.groups.get("e32eb6be-fd84-4bbe-8cd3-67f25be03845"),
        Some(&vec![Media {
            autoselect: true,
            media_type: MediaType::Video,
            bandwidth: 4096000,
            codecs: "avc1.640033".to_string(),
            group_id: "e32eb6be-fd84-4bbe-8cd3-67f25be03845".to_string(),
            default: true,
            forced: false,
            frame_rate: Some(60.0),
            name: "e32eb6be-fd84-4bbe-8cd3-67f25be03845".to_string(),
            resolution: Some((1280, 720)),
            uri: "e32eb6be-fd84-4bbe-8cd3-67f25be03845/index.m3u8".to_string(),
        }])
    );

    assert_eq!(
        master.groups.get("68b9c364-e04e-4fd7-b467-b751d74ef082"),
        Some(&vec![Media {
            autoselect: true,
            media_type: MediaType::Video,
            bandwidth: 2048000,
            codecs: "avc1.640033".to_string(),
            group_id: "68b9c364-e04e-4fd7-b467-b751d74ef082".to_string(),
            default: true,
            forced: false,
            frame_rate: Some(30.0),
            name: "68b9c364-e04e-4fd7-b467-b751d74ef082".to_string(),
            resolution: Some((853, 480)),
            uri: "68b9c364-e04e-4fd7-b467-b751d74ef082/index.m3u8".to_string(),
        }])
    );

    assert_eq!(
        master.groups.get("9c24b89d-9fa8-4b69-9c26-261393055869"),
        Some(&vec![Media {
            autoselect: true,
            media_type: MediaType::Video,
            bandwidth: 1024000,
            codecs: "avc1.640033".to_string(),
            group_id: "9c24b89d-9fa8-4b69-9c26-261393055869".to_string(),
            default: true,
            forced: false,
            frame_rate: Some(30.0),
            name: "9c24b89d-9fa8-4b69-9c26-261393055869".to_string(),
            resolution: Some((640, 360)),
            uri: "9c24b89d-9fa8-4b69-9c26-261393055869/index.m3u8".to_string(),
        }])
    );

    assert_eq!(
        master.streams[0],
        Stream {
            audio: Some("043897a5-cda1-458b-84d6-ce7a879a6a1e".to_string()),
            video: None,
            bandwidth: 98304,
            codecs: "opus".to_string(),
            group: "opus".to_string(),
            name: "audio-only".to_string(),
            uri: "043897a5-cda1-458b-84d6-ce7a879a6a1e/index.m3u8".to_string(),
            frame_rate: None,
            resolution: None,
        }
    );

    assert_eq!(
        master.streams[1],
        Stream {
            audio: Some("19c0a428-0925-40f7-ac28-979905df98a1".to_string()),
            video: None,
            bandwidth: 131072,
            codecs: "mp4a.40.2".to_string(),
            group: "aac".to_string(),
            name: "audio-only".to_string(),
            uri: "19c0a428-0925-40f7-ac28-979905df98a1/index.m3u8".to_string(),
            frame_rate: None,
            resolution: None,
        }
    );

    assert_eq!(
        master.streams[2],
        Stream {
            audio: Some("043897a5-cda1-458b-84d6-ce7a879a6a1e".to_string()),
            video: Some("541cf75a-ba88-4c91-b4a5-9023d7213f5d".to_string()),
            bandwidth: 8192000 + 98304,
            codecs: "av01.0.13M.08.0.110.01.01.01.0,opus".to_string(),
            group: "opus".to_string(),
            name: "source".to_string(),
            uri: "541cf75a-ba88-4c91-b4a5-9023d7213f5d/index.m3u8".to_string(),
            frame_rate: Some(60.0),
            resolution: Some((3840, 2160)),
        }
    );

    assert_eq!(
        master.streams[3],
        Stream {
            audio: Some("19c0a428-0925-40f7-ac28-979905df98a1".to_string()),
            video: Some("541cf75a-ba88-4c91-b4a5-9023d7213f5d".to_string()),
            bandwidth: 8192000 + 131072,
            codecs: "av01.0.13M.08.0.110.01.01.01.0,mp4a.40.2".to_string(),
            group: "aac".to_string(),
            name: "source".to_string(),
            uri: "541cf75a-ba88-4c91-b4a5-9023d7213f5d/index.m3u8".to_string(),
            frame_rate: Some(60.0),
            resolution: Some((3840, 2160)),
        }
    );

    assert_eq!(
        master.streams[4],
        Stream {
            audio: Some("043897a5-cda1-458b-84d6-ce7a879a6a1e".to_string()),
            video: Some("e32eb6be-fd84-4bbe-8cd3-67f25be03845".to_string()),
            bandwidth: 4096000 + 98304,
            codecs: "avc1.640033,opus".to_string(),
            group: "opus".to_string(),
            name: "720p".to_string(),
            uri: "e32eb6be-fd84-4bbe-8cd3-67f25be03845/index.m3u8".to_string(),
            frame_rate: Some(60.0),
            resolution: Some((1280, 720)),
        }
    );

    assert_eq!(
        master.streams[5],
        Stream {
            audio: Some("19c0a428-0925-40f7-ac28-979905df98a1".to_string()),
            video: Some("e32eb6be-fd84-4bbe-8cd3-67f25be03845".to_string()),
            bandwidth: 4096000 + 131072,
            codecs: "avc1.640033,mp4a.40.2".to_string(),
            group: "aac".to_string(),
            name: "720p".to_string(),
            uri: "e32eb6be-fd84-4bbe-8cd3-67f25be03845/index.m3u8".to_string(),
            frame_rate: Some(60.0),
            resolution: Some((1280, 720)),
        }
    );

    assert_eq!(
        master.streams[6],
        Stream {
            audio: Some("043897a5-cda1-458b-84d6-ce7a879a6a1e".to_string()),
            video: Some("68b9c364-e04e-4fd7-b467-b751d74ef082".to_string()),
            bandwidth: 2048000 + 98304,
            codecs: "avc1.640033,opus".to_string(),
            group: "opus".to_string(),
            name: "480p".to_string(),
            uri: "68b9c364-e04e-4fd7-b467-b751d74ef082/index.m3u8".to_string(),
            frame_rate: Some(30.0),
            resolution: Some((853, 480)),
        }
    );

    assert_eq!(
        master.streams[7],
        Stream {
            audio: Some("19c0a428-0925-40f7-ac28-979905df98a1".to_string()),
            video: Some("68b9c364-e04e-4fd7-b467-b751d74ef082".to_string()),
            bandwidth: 2048000 + 131072,
            codecs: "avc1.640033,mp4a.40.2".to_string(),
            group: "aac".to_string(),
            name: "480p".to_string(),
            uri: "68b9c364-e04e-4fd7-b467-b751d74ef082/index.m3u8".to_string(),
            frame_rate: Some(30.0),
            resolution: Some((853, 480)),
        }
    );

    assert_eq!(
        master.streams[8],
        Stream {
            audio: Some("043897a5-cda1-458b-84d6-ce7a879a6a1e".to_string()),
            video: Some("9c24b89d-9fa8-4b69-9c26-261393055869".to_string()),
            bandwidth: 1024000 + 98304,
            codecs: "avc1.640033,opus".to_string(),
            group: "opus".to_string(),
            name: "360p".to_string(),
            uri: "9c24b89d-9fa8-4b69-9c26-261393055869/index.m3u8".to_string(),
            frame_rate: Some(30.0),
            resolution: Some((640, 360)),
        }
    );

    assert_eq!(
        master.streams[9],
        Stream {
            audio: Some("19c0a428-0925-40f7-ac28-979905df98a1".to_string()),
            video: Some("9c24b89d-9fa8-4b69-9c26-261393055869".to_string()),
            bandwidth: 1024000 + 131072,
            codecs: "avc1.640033,mp4a.40.2".to_string(),
            group: "aac".to_string(),
            name: "360p".to_string(),
            uri: "9c24b89d-9fa8-4b69-9c26-261393055869/index.m3u8".to_string(),
            frame_rate: Some(30.0),
            resolution: Some((640, 360)),
        }
    );
}
