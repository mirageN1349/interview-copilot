use serde::{Deserialize, Serialize};

pub const MAX_SCREENSHOT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayGeometry {
    pub display_id: u32,
    pub frame: LogicalRect,
    pub backing_scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedCaptureGeometry {
    pub source_rect: LogicalRect,
    pub pixel_width: usize,
    pub pixel_height: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureGeometryError {
    AreaOutOfBounds,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScreenshotScanDecision {
    Allow,
    Redact {
        bytes: Vec<u8>,
        reason: &'static str,
    },
    Reject {
        reason: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeScreenshotError {
    DisplayNotFound,
    AreaOutOfBounds,
    PermissionDenied,
    CaptureFailed,
    TimedOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnCapturePrivacy {
    pub current_application_excluded: bool,
    pub cursor_included: bool,
    pub third_party_cursor_controlled: bool,
}

pub const fn own_capture_privacy() -> OwnCapturePrivacy {
    OwnCapturePrivacy {
        current_application_excluded: true,
        cursor_included: false,
        third_party_cursor_controlled: false,
    }
}

pub fn prepare_capture_geometry(
    display: DisplayGeometry,
    area: Option<LogicalRect>,
) -> Result<PreparedCaptureGeometry, CaptureGeometryError> {
    if !valid_rect(display.frame)
        || !display.backing_scale.is_finite()
        || display.backing_scale <= 0.0
    {
        return Err(CaptureGeometryError::AreaOutOfBounds);
    }
    let global = area.unwrap_or(display.frame);
    let right = global.x + global.width;
    let bottom = global.y + global.height;
    let display_right = display.frame.x + display.frame.width;
    let display_bottom = display.frame.y + display.frame.height;
    if !valid_rect(global)
        || global.x < display.frame.x
        || global.y < display.frame.y
        || right > display_right
        || bottom > display_bottom
    {
        return Err(CaptureGeometryError::AreaOutOfBounds);
    }
    Ok(PreparedCaptureGeometry {
        source_rect: LogicalRect {
            x: global.x - display.frame.x,
            y: global.y - display.frame.y,
            width: global.width,
            height: global.height,
        },
        pixel_width: scaled_pixels(global.width, display.backing_scale),
        pixel_height: scaled_pixels(global.height, display.backing_scale),
    })
}

pub fn scan_screenshot(bytes: &[u8]) -> ScreenshotScanDecision {
    if bytes.len() < 8 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return ScreenshotScanDecision::Reject {
            reason: "Screenshot encoding is invalid",
        };
    }
    if bytes.len() > MAX_SCREENSHOT_BYTES {
        return ScreenshotScanDecision::Reject {
            reason: "Screenshot exceeds the local size limit",
        };
    }
    let mut offset = 8;
    let mut redacted = bytes[..8].to_vec();
    let mut removed_sensitive_text = false;
    let mut saw_header = false;
    let mut saw_end = false;
    while offset + 12 <= bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let Some(chunk_end) = offset.checked_add(12 + length) else {
            return invalid_encoding();
        };
        if chunk_end > bytes.len() {
            return invalid_encoding();
        }
        let chunk_type = &bytes[offset + 4..offset + 8];
        let data = &bytes[offset + 8..offset + 8 + length];
        saw_header |= chunk_type == b"IHDR" && offset == 8;
        saw_end |= chunk_type == b"IEND" && length == 0;
        let sensitive = contains_secret_marker(data);
        let textual = matches!(chunk_type, b"tEXt" | b"zTXt" | b"iTXt");
        if sensitive && textual {
            removed_sensitive_text = true;
        } else {
            if sensitive {
                return ScreenshotScanDecision::Reject {
                    reason: "Screenshot contains a blocked secret marker",
                };
            }
            redacted.extend_from_slice(&bytes[offset..chunk_end]);
        }
        offset = chunk_end;
        if saw_end {
            break;
        }
    }
    if !saw_header || !saw_end || offset != bytes.len() {
        return invalid_encoding();
    }
    if removed_sensitive_text {
        return ScreenshotScanDecision::Redact {
            bytes: redacted,
            reason: "Sensitive screenshot metadata was removed",
        };
    }
    ScreenshotScanDecision::Allow
}

fn contains_secret_marker(bytes: &[u8]) -> bool {
    let lower = bytes.iter().map(u8::to_ascii_lowercase).collect::<Vec<_>>();
    [
        b"api_key=".as_slice(),
        b"password=".as_slice(),
        b"private_key".as_slice(),
    ]
    .iter()
    .any(|marker| lower.windows(marker.len()).any(|window| window == *marker))
}

fn invalid_encoding() -> ScreenshotScanDecision {
    ScreenshotScanDecision::Reject {
        reason: "Screenshot encoding is invalid",
    }
}

fn valid_rect(rect: LogicalRect) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .iter()
        .all(|value| value.is_finite())
        && rect.width > 0.0
        && rect.height > 0.0
}

fn scaled_pixels(points: f64, scale: f64) -> usize {
    (points * scale).round().max(1.0) as usize
}

#[cfg(target_os = "macos")]
pub fn capture_display_png(
    display_id: u32,
    area: Option<LogicalRect>,
    own_bundle_identifier: &str,
) -> Result<Vec<u8>, NativeScreenshotError> {
    use std::{
        sync::mpsc,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use block2::RcBlock;
    use objc2::AnyThread;
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_core_graphics::CGDisplayPixelsWide;
    use objc2_foundation::{NSArray, NSError};
    use objc2_screen_capture_kit::{
        SCContentFilter, SCScreenshotManager, SCShareableContent, SCStreamConfiguration, SCWindow,
    };

    let (sender, receiver) = mpsc::channel();
    let bundle_identifier = own_bundle_identifier.to_owned();
    let completion = RcBlock::new(
        move |content: *mut SCShareableContent, error: *mut NSError| unsafe {
            if content.is_null() {
                let _ = sender.send(Err(if error.is_null() {
                    NativeScreenshotError::CaptureFailed
                } else {
                    NativeScreenshotError::PermissionDenied
                }));
                return;
            }
            let content = &*content;
            let displays = content.displays();
            let Some(display) = displays
                .iter()
                .find(|display| display.displayID() == display_id)
            else {
                let _ = sender.send(Err(NativeScreenshotError::DisplayNotFound));
                return;
            };
            let frame = display.frame();
            let display_geometry = DisplayGeometry {
                display_id,
                frame: LogicalRect {
                    x: frame.origin.x,
                    y: frame.origin.y,
                    width: frame.size.width,
                    height: frame.size.height,
                },
                backing_scale: CGDisplayPixelsWide(display_id) as f64 / frame.size.width.max(1.0),
            };
            let prepared = match prepare_capture_geometry(display_geometry, area) {
                Ok(prepared) => prepared,
                Err(_) => {
                    let _ = sender.send(Err(NativeScreenshotError::AreaOutOfBounds));
                    return;
                }
            };
            let applications = content.applications();
            let excluded = applications
                .iter()
                .filter(|application| {
                    application.bundleIdentifier().to_string() == bundle_identifier
                })
                .collect::<Vec<_>>();
            if excluded.is_empty() {
                let _ = sender.send(Err(NativeScreenshotError::CaptureFailed));
                return;
            }
            let excluded = NSArray::from_retained_slice(&excluded);
            let no_windows = NSArray::<SCWindow>::from_slice(&[]);
            let filter = SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(
                SCContentFilter::alloc(),
                &display,
                &excluded,
                &no_windows,
            );
            let config = SCStreamConfiguration::new();
            config.setShowsCursor(false);
            config.setSourceRect(CGRect {
                origin: CGPoint {
                    x: prepared.source_rect.x,
                    y: prepared.source_rect.y,
                },
                size: CGSize {
                    width: prepared.source_rect.width,
                    height: prepared.source_rect.height,
                },
            });
            config.setWidth(prepared.pixel_width);
            config.setHeight(prepared.pixel_height);

            let sequence = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "interview-copilot-screenshot-{}-{sequence}.png",
                std::process::id()
            ));
            let capture_sender = sender.clone();
            let capture = RcBlock::new(
                move |image: *mut objc2_core_graphics::CGImage, error: *mut NSError| {
                    let result = if image.is_null() {
                        Err(if error.is_null() {
                            NativeScreenshotError::CaptureFailed
                        } else {
                            NativeScreenshotError::PermissionDenied
                        })
                    } else {
                        encode_png(image, &path)
                    };
                    let _ = std::fs::remove_file(&path);
                    let _ = capture_sender.send(result);
                },
            );
            SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(
                &filter,
                &config,
                Some(&capture),
            );
        },
    );
    unsafe { SCShareableContent::getShareableContentWithCompletionHandler(&completion) };
    receiver
        .recv_timeout(Duration::from_secs(15))
        .map_err(|_| NativeScreenshotError::TimedOut)?
}

#[cfg(target_os = "macos")]
fn encode_png(
    image: *mut objc2_core_graphics::CGImage,
    path: &std::path::Path,
) -> Result<Vec<u8>, NativeScreenshotError> {
    use core::ffi::c_void;
    use objc2_foundation::{NSString, NSURL};

    #[link(name = "ImageIO", kind = "framework")]
    unsafe extern "C" {
        fn CGImageDestinationCreateWithURL(
            url: *const c_void,
            type_identifier: *const c_void,
            count: usize,
            options: *const c_void,
        ) -> *mut c_void;
        fn CGImageDestinationAddImage(
            destination: *mut c_void,
            image: *const objc2_core_graphics::CGImage,
            properties: *const c_void,
        );
        fn CGImageDestinationFinalize(destination: *mut c_void) -> bool;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(value: *const c_void);
    }

    let Some(path) = path.to_str() else {
        return Err(NativeScreenshotError::CaptureFailed);
    };
    let path = NSString::from_str(path);
    let url = NSURL::fileURLWithPath(&path);
    let png = NSString::from_str("public.png");
    let destination = unsafe {
        CGImageDestinationCreateWithURL(
            (&*url as *const NSURL).cast(),
            (&*png as *const NSString).cast(),
            1,
            std::ptr::null(),
        )
    };
    if destination.is_null() {
        return Err(NativeScreenshotError::CaptureFailed);
    }
    unsafe {
        CGImageDestinationAddImage(destination, image, std::ptr::null());
    }
    let finalized = unsafe { CGImageDestinationFinalize(destination) };
    unsafe { CFRelease(destination) };
    if !finalized {
        return Err(NativeScreenshotError::CaptureFailed);
    }
    std::fs::read(path.to_string()).map_err(|_| NativeScreenshotError::CaptureFailed)
}

#[cfg(not(target_os = "macos"))]
pub fn capture_display_png(
    _display_id: u32,
    _area: Option<LogicalRect>,
    _own_bundle_identifier: &str,
) -> Result<Vec<u8>, NativeScreenshotError> {
    Err(NativeScreenshotError::CaptureFailed)
}
