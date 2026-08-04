#![allow(dead_code)]

#[path = "../src/storage/artifacts.rs"]
mod artifacts;
#[path = "../src/macos/audio.rs"]
mod audio;
#[path = "../src/macos/capture.rs"]
mod capture;

use artifacts::{ArtifactDecision, RecordingFragment};
use audio::{VadConfig, VadEvent, VadState};
use capture::{
    CaptureConfiguration, CaptureRuntime, CaptureState, RuntimeAction, SourceLostReason,
};

#[test]
fn vad_starts_at_threshold_and_stops_after_hangover() {
    let mut vad = VadState::new(VadConfig::new(0.25, 2).unwrap());

    assert_eq!(vad.observe(0.24), VadEvent::Silent);
    assert_eq!(vad.observe(0.25), VadEvent::SpeechStarted);
    assert_eq!(vad.observe(0.10), VadEvent::SpeechContinued);
    assert_eq!(vad.observe(0.10), VadEvent::SpeechStopped);
    assert_eq!(vad.observe(f32::NAN), VadEvent::Silent);
}

#[test]
fn capture_runtime_pauses_on_source_loss_and_stop_is_idempotent() {
    let config = CaptureConfiguration::new(42, true, true, 48_000, 2, true).unwrap();
    let mut runtime = CaptureRuntime::new(config);

    assert_eq!(
        runtime.on_vad(VadEvent::SpeechStarted),
        RuntimeAction::OpenFragment
    );
    assert_eq!(runtime.state(), CaptureState::Recording);
    assert_eq!(
        runtime.on_vad(VadEvent::SpeechStopped),
        RuntimeAction::FinalizeFragment
    );
    assert_eq!(runtime.state(), CaptureState::Listening);

    let outcome = runtime.source_lost(SourceLostReason::DisplayDisconnected);
    assert_eq!(runtime.state(), CaptureState::PausedSourceLost);
    assert_eq!(outcome.event.display_id, 42);
    assert!(outcome.event.reselection_required);
    assert_eq!(outcome.action, RuntimeAction::None);
    assert_eq!(runtime.stop(), RuntimeAction::StopStream);
    assert_eq!(runtime.stop(), RuntimeAction::None);
    assert_eq!(runtime.state(), CaptureState::Stopped);
}

#[test]
fn source_lost_event_has_stable_public_shape() {
    let mut runtime =
        CaptureRuntime::new(CaptureConfiguration::new(7, true, false, 48_000, 2, true).unwrap());
    let value = serde_json::to_value(
        runtime
            .source_lost(SourceLostReason::DisplayDisconnected)
            .event,
    )
    .unwrap();

    assert_eq!(value["displayId"], 7);
    assert_eq!(value["reason"], "display_disconnected");
    assert_eq!(value["reselectionRequired"], true);
    assert_eq!(
        serde_json::to_string(&SourceLostReason::DisplayUnavailable).unwrap(),
        "\"display_unavailable\""
    );
    assert_eq!(
        serde_json::to_string(&SourceLostReason::StreamFailed).unwrap(),
        "\"stream_failed\""
    );
}

#[test]
fn only_vetted_complete_fragments_are_attachable() {
    let fragment = RecordingFragment::open("fragment-1", "meeting-1", 3, 100).unwrap();
    let pending = fragment
        .complete(
            200,
            "artifacts/meeting-1/fragment-3.caf",
            512,
            "sha256:test",
        )
        .unwrap();

    assert!(!pending.is_attachable());
    assert!(
        pending
            .clone()
            .vet(ArtifactDecision::Allow)
            .unwrap()
            .is_attachable()
    );
    assert!(
        pending
            .clone()
            .vet(ArtifactDecision::Redact)
            .unwrap()
            .is_attachable()
    );
    assert!(
        !pending
            .vet(ArtifactDecision::Reject)
            .unwrap()
            .is_attachable()
    );
}

#[test]
fn source_loss_or_stop_finalizes_an_open_fragment() {
    let config = CaptureConfiguration::new(42, true, true, 48_000, 2, true).unwrap();
    let mut runtime = CaptureRuntime::new(config);
    runtime.on_vad(VadEvent::SpeechStarted);
    let outcome = runtime.source_lost(SourceLostReason::DisplayDisconnected);
    assert_eq!(outcome.action, RuntimeAction::FinalizeFragment);

    let mut runtime = CaptureRuntime::new(config);
    runtime.on_vad(VadEvent::SpeechStarted);
    assert_eq!(runtime.stop(), RuntimeAction::FinalizeAndStop);
}

#[test]
fn incomplete_or_reversed_fragments_fail_closed() {
    assert!(RecordingFragment::open("", "meeting-1", 0, 100).is_err());
    let fragment = RecordingFragment::open("fragment-1", "meeting-1", 0, 100).unwrap();
    assert!(fragment.complete(99, "key", 1, "checksum").is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn native_stream_configuration_applies_audio_and_privacy_intent() {
    use objc2_screen_capture_kit::SCStreamConfiguration;

    let config = CaptureConfiguration::new(42, true, true, 48_000, 2, true).unwrap();
    // SAFETY: the test owns the configuration for the duration of all property reads.
    unsafe {
        let native = SCStreamConfiguration::new();
        capture::apply_screen_capture_kit_configuration(&native, config);
        assert!(native.capturesAudio());
        assert!(native.captureMicrophone());
        assert!(native.excludesCurrentProcessAudio());
        assert!(!native.showsCursor());
        assert_eq!(native.sampleRate(), 48_000);
        assert_eq!(native.channelCount(), 2);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn planar_audio_buffers_are_combined_for_vad_and_artifacts() {
    use objc2_core_audio_types::{AudioBuffer, AudioBufferList};

    #[repr(C)]
    struct TwoBufferList {
        count: u32,
        buffers: [AudioBuffer; 2],
    }

    let mut left = [1_u8, 2, 3];
    let mut right = [4_u8, 5];
    let list = TwoBufferList {
        count: 2,
        buffers: [
            AudioBuffer {
                mNumberChannels: 1,
                mDataByteSize: left.len() as u32,
                mData: left.as_mut_ptr().cast(),
            },
            AudioBuffer {
                mNumberChannels: 1,
                mDataByteSize: right.len() as u32,
                mData: right.as_mut_ptr().cast(),
            },
        ],
    };
    // SAFETY: TwoBufferList has the C flexible-array prefix layout of AudioBufferList.
    let list = unsafe { &*std::ptr::from_ref(&list).cast::<AudioBufferList>() };
    assert_eq!(
        capture::copy_audio_buffer_list_bytes(list),
        Some(vec![1, 2, 3, 4, 5])
    );
}
