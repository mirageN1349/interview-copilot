#![allow(dead_code)]

#[path = "../src/storage/artifacts.rs"]
mod artifacts;
#[path = "../src/macos/screenshot.rs"]
mod screenshot;

use artifacts::{ArtifactDecision, ArtifactStatus, RecordingFragment};
use screenshot::{
    CaptureGeometryError, DisplayGeometry, LogicalRect, ScreenshotScanDecision,
    prepare_capture_geometry, scan_screenshot,
};

fn negative_origin_retina_display() -> DisplayGeometry {
    DisplayGeometry {
        display_id: 42,
        frame: LogicalRect {
            x: -1_440.0,
            y: -180.0,
            width: 1_440.0,
            height: 900.0,
        },
        backing_scale: 2.0,
    }
}

fn png_with_chunks(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in chunks {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        png.extend_from_slice(*kind);
        png.extend_from_slice(data);
        png.extend_from_slice(&[0; 4]);
    }
    png
}

fn fixture_png(extra_chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let ihdr = [0_u8; 13];
    let mut chunks = vec![
        (b"IHDR", ihdr.as_slice()),
        (b"IDAT", b"synthetic pixels".as_slice()),
    ];
    chunks.extend_from_slice(extra_chunks);
    chunks.push((b"IEND", &[]));
    png_with_chunks(&chunks)
}

#[test]
fn converts_global_negative_origin_area_to_local_retina_geometry() {
    let prepared = prepare_capture_geometry(
        negative_origin_retina_display(),
        Some(LogicalRect {
            x: -1_340.0,
            y: -130.0,
            width: 320.0,
            height: 180.0,
        }),
    )
    .unwrap();

    assert_eq!(prepared.source_rect.x, 100.0);
    assert_eq!(prepared.source_rect.y, 50.0);
    assert_eq!(prepared.pixel_width, 640);
    assert_eq!(prepared.pixel_height, 360);
}

#[test]
fn full_display_uses_local_bounds_and_current_scale() {
    let prepared = prepare_capture_geometry(negative_origin_retina_display(), None).unwrap();

    assert_eq!(prepared.source_rect.x, 0.0);
    assert_eq!(prepared.source_rect.y, 0.0);
    assert_eq!(prepared.pixel_width, 2_880);
    assert_eq!(prepared.pixel_height, 1_800);
}

#[test]
fn rejects_non_finite_empty_and_out_of_display_areas() {
    let display = negative_origin_retina_display();
    for area in [
        LogicalRect {
            x: f64::NAN,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        LogicalRect {
            x: -1_000.0,
            y: 0.0,
            width: 0.0,
            height: 10.0,
        },
        LogicalRect {
            x: -100.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        },
    ] {
        assert_eq!(
            prepare_capture_geometry(display, Some(area)),
            Err(CaptureGeometryError::AreaOutOfBounds)
        );
    }
}

#[test]
fn scan_fails_closed_before_artifact_becomes_attachable() {
    let pending = RecordingFragment::open("shot-1", "meeting-1", 0, 10)
        .unwrap()
        .complete(11, "screenshots/shot-1.png", 32, "checksum")
        .unwrap();
    assert!(!pending.is_attachable());

    let allowed_png = fixture_png(&[]);
    assert_eq!(scan_screenshot(&allowed_png), ScreenshotScanDecision::Allow);
    let allowed = pending.vet(ArtifactDecision::Allow).unwrap();
    assert_eq!(allowed.status, ArtifactStatus::Allowed);
    assert!(allowed.is_attachable());

    let sensitive_png = fixture_png(&[(b"IDAT", b"api_key=fixture-secret")]);
    assert!(matches!(
        scan_screenshot(&sensitive_png),
        ScreenshotScanDecision::Reject { .. }
    ));
}

#[test]
fn scan_removes_sensitive_text_chunk_without_exposing_pending_artifact() {
    let png = fixture_png(&[(b"tEXt", b"note\0password=fixture-secret")]);
    let ScreenshotScanDecision::Redact { bytes, reason } = scan_screenshot(&png) else {
        panic!("sensitive text metadata must be redacted");
    };
    assert_eq!(reason, "Sensitive screenshot metadata was removed");
    assert!(!bytes.windows(9).any(|window| window == b"password="));
    assert!(bytes.ends_with(&[0, 0, 0, 0]));
}

#[test]
fn scan_rejects_empty_malformed_and_oversized_payloads() {
    assert!(matches!(
        scan_screenshot(&[]),
        ScreenshotScanDecision::Reject { .. }
    ));
    assert!(matches!(
        scan_screenshot(b"not a png"),
        ScreenshotScanDecision::Reject { .. }
    ));
    let mut oversized = vec![0; screenshot::MAX_SCREENSHOT_BYTES + 1];
    oversized[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    assert!(matches!(
        scan_screenshot(&oversized),
        ScreenshotScanDecision::Reject { .. }
    ));
}
