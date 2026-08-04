use serde::Serialize;

use super::audio::VadEvent;

#[cfg(target_os = "macos")]
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn copy_audio_buffer_list_bytes(
    list: &objc2_core_audio_types::AudioBufferList,
) -> Option<Vec<u8>> {
    let count = usize::try_from(list.mNumberBuffers).ok()?;
    if count == 0 {
        return None;
    }
    // SAFETY: AudioBufferList is a C flexible-array structure. ScreenCaptureKit owns a list
    // containing mNumberBuffers entries for the duration of the sample callback.
    let buffers = unsafe { std::slice::from_raw_parts(list.mBuffers.as_ptr(), count) };
    let capacity = buffers.iter().try_fold(0_usize, |total, buffer| {
        total.checked_add(buffer.mDataByteSize as usize)
    })?;
    if capacity == 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(capacity);
    for buffer in buffers {
        let length = buffer.mDataByteSize as usize;
        if length == 0 {
            continue;
        }
        let data = std::ptr::NonNull::new(buffer.mData.cast::<u8>())?;
        // SAFETY: CoreMedia guarantees each AudioBuffer data pointer is valid for its byte size
        // while the retained sample buffer is alive; the bytes are copied before callback return.
        bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(data.as_ptr(), length) });
    }
    Some(bytes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureConfiguration {
    pub display_id: u32,
    pub capture_system_audio: bool,
    pub capture_microphone: bool,
    pub sample_rate: i32,
    pub channel_count: i32,
    pub exclude_current_process_audio: bool,
}

impl CaptureConfiguration {
    pub fn new(
        display_id: u32,
        capture_system_audio: bool,
        capture_microphone: bool,
        sample_rate: i32,
        channel_count: i32,
        exclude_current_process_audio: bool,
    ) -> Result<Self, &'static str> {
        if display_id == 0 {
            return Err("A display must be selected");
        }
        if !(8_000..=192_000).contains(&sample_rate) {
            return Err("Unsupported audio sample rate");
        }
        if !(1..=8).contains(&channel_count) {
            return Err("Unsupported audio channel count");
        }
        Ok(Self {
            display_id,
            capture_system_audio,
            capture_microphone,
            sample_rate,
            channel_count,
            exclude_current_process_audio,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Listening,
    Recording,
    PausedSourceLost,
    Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeAction {
    None,
    OpenFragment,
    FinalizeFragment,
    FinalizeAndStop,
    StopStream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLostReason {
    DisplayDisconnected,
    DisplayUnavailable,
    StreamFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLostEvent {
    pub display_id: u32,
    pub reason: SourceLostReason,
    pub reselection_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLostOutcome {
    pub event: SourceLostEvent,
    pub action: RuntimeAction,
}

pub struct CaptureRuntime {
    configuration: CaptureConfiguration,
    state: CaptureState,
    #[cfg(all(target_os = "macos", not(test)))]
    native: Option<native::NativeCaptureSession>,
}

impl std::fmt::Debug for CaptureRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CaptureRuntime")
            .field("configuration", &self.configuration)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl CaptureRuntime {
    pub fn new(configuration: CaptureConfiguration) -> Self {
        Self {
            configuration,
            state: CaptureState::Listening,
            #[cfg(all(target_os = "macos", not(test)))]
            native: None,
        }
    }

    pub fn state(&self) -> CaptureState {
        self.state
    }

    pub fn on_vad(&mut self, event: VadEvent) -> RuntimeAction {
        match (self.state, event) {
            (CaptureState::Listening, VadEvent::SpeechStarted) => {
                self.state = CaptureState::Recording;
                RuntimeAction::OpenFragment
            }
            (CaptureState::Recording, VadEvent::SpeechStopped) => {
                self.state = CaptureState::Listening;
                RuntimeAction::FinalizeFragment
            }
            _ => RuntimeAction::None,
        }
    }

    pub fn source_lost(&mut self, reason: SourceLostReason) -> SourceLostOutcome {
        let action = if self.state == CaptureState::Recording {
            RuntimeAction::FinalizeFragment
        } else {
            RuntimeAction::None
        };
        self.state = CaptureState::PausedSourceLost;
        SourceLostOutcome {
            event: SourceLostEvent {
                display_id: self.configuration.display_id,
                reason,
                reselection_required: true,
            },
            action,
        }
    }

    pub fn stop(&mut self) -> RuntimeAction {
        if self.state == CaptureState::Stopped {
            return RuntimeAction::None;
        }
        let action = if self.state == CaptureState::Recording {
            RuntimeAction::FinalizeAndStop
        } else {
            RuntimeAction::StopStream
        };
        #[cfg(all(target_os = "macos", not(test)))]
        if let Some(native) = self.native.as_ref() {
            native.stop();
        }
        self.state = CaptureState::Stopped;
        action
    }

    #[cfg(all(target_os = "macos", not(test)))]
    pub fn start_native(
        &mut self,
        app: tauri::AppHandle,
        owner_user_id: String,
        meeting_id: String,
    ) -> Result<(), String> {
        if self.native.is_some() {
            return Ok(());
        }
        self.native = Some(native::NativeCaptureSession::start(
            self.configuration,
            app,
            owner_user_id,
            meeting_id,
        )?);
        Ok(())
    }

    #[cfg(all(target_os = "macos", not(test)))]
    pub fn delivered_samples(&self) -> (u64, u64, u64) {
        self.native
            .as_ref()
            .map(native::NativeCaptureSession::delivered_samples)
            .unwrap_or_default()
    }

    #[cfg(all(target_os = "macos", test))]
    pub fn start_native(
        &mut self,
        _app: tauri::AppHandle,
        _owner_user_id: String,
        _meeting_id: String,
    ) -> Result<(), String> {
        Err("native capture is unavailable in the unit-test harness".to_owned())
    }

    #[cfg(all(target_os = "macos", test))]
    pub fn delivered_samples(&self) -> (u64, u64, u64) {
        (0, 0, 0)
    }
}

#[cfg(target_os = "macos")]
pub fn apply_screen_capture_kit_configuration(
    native: &objc2_screen_capture_kit::SCStreamConfiguration,
    configuration: CaptureConfiguration,
) {
    // SAFETY: these setters only mutate the caller-owned configuration object.
    unsafe {
        native.setCapturesAudio(configuration.capture_system_audio);
        native.setCaptureMicrophone(configuration.capture_microphone);
        native.setExcludesCurrentProcessAudio(configuration.exclude_current_process_audio);
        native.setSampleRate(configuration.sample_rate as isize);
        native.setChannelCount(configuration.channel_count as isize);
        native.setShowsCursor(false);
        native.setQueueDepth(5);
    }
}

#[cfg(all(target_os = "macos", not(test)))]
mod native {
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicU64, Ordering},
            mpsc,
        },
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use block2::RcBlock;
    use dispatch2::{DispatchQueue, DispatchQueueAttr, DispatchRetained};
    use objc2::{AnyThread, DefinedClass, define_class, msg_send, rc::Retained};
    use objc2_core_audio_types::{AudioBufferList, kAudioFormatFlagIsFloat, kAudioFormatLinearPCM};
    use objc2_core_media::{
        CMAudioFormatDescriptionGetStreamBasicDescription, CMSampleBuffer,
        kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment,
    };
    use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol};
    use objc2_screen_capture_kit::{
        SCContentFilter, SCShareableContent, SCStream, SCStreamConfiguration, SCStreamDelegate,
        SCStreamOutput, SCStreamOutputType,
    };
    use sha2::{Digest, Sha256};
    use tauri::{Emitter, Manager};

    use super::{
        CaptureConfiguration, SourceLostEvent, SourceLostReason,
        apply_screen_capture_kit_configuration, copy_audio_buffer_list_bytes,
    };
    use crate::{
        commands::profiles::ProfileCommandState,
        macos::audio::{VadConfig, VadEvent, VadState, normalized_pcm_rms},
        storage::{StorageKey, artifacts::RecordingFragment},
    };

    const CALLBACK_TIMEOUT: Duration = Duration::from_secs(15);

    #[derive(Default)]
    struct SampleCounters {
        screen: AtomicU64,
        system_audio: AtomicU64,
        microphone: AtomicU64,
    }

    struct OpenFragment {
        model: RecordingFragment,
        bytes: Vec<u8>,
    }

    struct CaptureSink {
        app: tauri::AppHandle,
        owner_user_id: String,
        meeting_id: String,
        display_id: u32,
        sequence: u64,
        vad: VadState,
        open: Option<OpenFragment>,
        counters: Arc<SampleCounters>,
    }

    impl CaptureSink {
        fn audio_sample(&mut self, sample: &CMSampleBuffer, output_type: SCStreamOutputType) {
            if output_type == SCStreamOutputType::Audio {
                self.counters.system_audio.fetch_add(1, Ordering::Relaxed);
            } else if output_type == SCStreamOutputType::Microphone {
                self.counters.microphone.fetch_add(1, Ordering::Relaxed);
            } else {
                return;
            }
            let Some((bytes, bits, is_float, sample_rate, channels)) = copy_pcm_bytes(sample)
            else {
                return;
            };
            let Some(rms) = normalized_pcm_rms(&bytes, bits, is_float) else {
                return;
            };
            let event = self.vad.observe(rms);
            if event == VadEvent::SpeechStarted {
                self.open_fragment();
            }
            if matches!(
                event,
                VadEvent::SpeechStarted | VadEvent::SpeechContinued | VadEvent::SpeechStopped
            ) {
                if let Some(open) = self.open.as_mut() {
                    append_sample_envelope(
                        &mut open.bytes,
                        output_type,
                        sample_rate,
                        channels,
                        bits,
                        is_float,
                        &bytes,
                    );
                }
            }
            if event == VadEvent::SpeechStopped {
                self.finalize_fragment();
            }
        }

        fn open_fragment(&mut self) {
            if self.open.is_some() {
                return;
            }
            let started_at = now_ms();
            let id = format!("audio-{}-{}", started_at, self.sequence);
            if let Ok(model) =
                RecordingFragment::open(id, self.meeting_id.clone(), self.sequence, started_at)
            {
                self.sequence += 1;
                self.open = Some(OpenFragment {
                    model,
                    bytes: b"ICAF\x01".to_vec(),
                });
            }
        }

        fn finalize_fragment(&mut self) {
            let Some(open) = self.open.take() else {
                return;
            };
            if let Err(code) = persist_fragment(
                &self.app,
                &self.owner_user_id,
                open.model,
                now_ms(),
                &open.bytes,
            ) {
                let _ = self.app.emit("capture://error", code);
            }
        }

        fn source_lost(&mut self) {
            self.finalize_fragment();
            let _ = self.app.emit(
                "capture://source-lost",
                SourceLostEvent {
                    display_id: self.display_id,
                    reason: SourceLostReason::StreamFailed,
                    reselection_required: true,
                },
            );
        }
    }

    struct CaptureDelegateIvars {
        sink: Arc<Mutex<CaptureSink>>,
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[ivars = CaptureDelegateIvars]
        struct CaptureDelegate;

        unsafe impl NSObjectProtocol for CaptureDelegate {}

        unsafe impl SCStreamOutput for CaptureDelegate {
            #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
            unsafe fn stream_did_output_sample_buffer_of_type(
                &self,
                _stream: &SCStream,
                sample_buffer: &CMSampleBuffer,
                output_type: SCStreamOutputType,
            ) {
                if !unsafe { sample_buffer.is_valid() && sample_buffer.data_is_ready() } {
                    return;
                }
                if output_type == SCStreamOutputType::Screen {
                    if let Ok(sink) = self.ivars().sink.lock() {
                        sink.counters.screen.fetch_add(1, Ordering::Relaxed);
                    }
                } else if let Ok(mut sink) = self.ivars().sink.lock() {
                    sink.audio_sample(sample_buffer, output_type);
                }
            }
        }

        unsafe impl SCStreamDelegate for CaptureDelegate {
            #[unsafe(method(stream:didStopWithError:))]
            unsafe fn stream_did_stop_with_error(&self, _stream: &SCStream, _error: &NSError) {
                if let Ok(mut sink) = self.ivars().sink.lock() {
                    sink.source_lost();
                }
            }
        }
    );

    impl CaptureDelegate {
        fn new(sink: Arc<Mutex<CaptureSink>>) -> Retained<Self> {
            let this = Self::alloc().set_ivars(CaptureDelegateIvars { sink });
            unsafe { msg_send![super(this), init] }
        }
    }

    pub(super) struct NativeCaptureSession {
        stream: Retained<SCStream>,
        _delegate: Retained<CaptureDelegate>,
        _queue: DispatchRetained<DispatchQueue>,
        sink: Arc<Mutex<CaptureSink>>,
        counters: Arc<SampleCounters>,
        stopped: AtomicBool,
    }

    // SAFETY: ScreenCaptureKit streams, their delegate, and dispatch queues are designed for
    // cross-thread control. Rust access is limited to atomic counters and SCStream's thread-safe
    // start/stop methods; delegate state is protected by a mutex.
    unsafe impl Send for NativeCaptureSession {}
    unsafe impl Sync for NativeCaptureSession {}

    impl NativeCaptureSession {
        pub(super) fn start(
            configuration: CaptureConfiguration,
            app: tauri::AppHandle,
            owner_user_id: String,
            meeting_id: String,
        ) -> Result<Self, String> {
            let content = shareable_content()?;
            let display = unsafe { content.displays() }
                .iter()
                .find(|display| unsafe { display.displayID() == configuration.display_id })
                .ok_or_else(|| "DISPLAY_NOT_FOUND".to_owned())?;
            let excluded = NSArray::new();
            let filter = unsafe {
                SCContentFilter::initWithDisplay_excludingWindows(
                    SCContentFilter::alloc(),
                    &display,
                    &excluded,
                )
            };
            let native_configuration = unsafe { SCStreamConfiguration::new() };
            apply_screen_capture_kit_configuration(&native_configuration, configuration);

            let counters = Arc::new(SampleCounters::default());
            let sink = Arc::new(Mutex::new(CaptureSink {
                app,
                owner_user_id,
                meeting_id,
                display_id: configuration.display_id,
                sequence: 0,
                vad: VadState::new(VadConfig::new(0.02, 8).expect("static VAD config")),
                open: None,
                counters: counters.clone(),
            }));
            let delegate = CaptureDelegate::new(sink.clone());
            let stream = unsafe {
                SCStream::initWithFilter_configuration_delegate(
                    SCStream::alloc(),
                    &filter,
                    &native_configuration,
                    Some(objc2::runtime::ProtocolObject::from_ref(&*delegate)),
                )
            };
            let queue = DispatchQueue::new(
                "com.interview-copilot.capture.samples",
                DispatchQueueAttr::SERIAL,
            );
            let output = objc2::runtime::ProtocolObject::from_ref(&*delegate);
            unsafe {
                stream
                    .addStreamOutput_type_sampleHandlerQueue_error(
                        output,
                        SCStreamOutputType::Screen,
                        Some(&queue),
                    )
                    .map_err(|error| error.localizedDescription().to_string())?;
                if configuration.capture_system_audio {
                    stream
                        .addStreamOutput_type_sampleHandlerQueue_error(
                            output,
                            SCStreamOutputType::Audio,
                            Some(&queue),
                        )
                        .map_err(|error| error.localizedDescription().to_string())?;
                }
                if configuration.capture_microphone {
                    stream
                        .addStreamOutput_type_sampleHandlerQueue_error(
                            output,
                            SCStreamOutputType::Microphone,
                            Some(&queue),
                        )
                        .map_err(|error| error.localizedDescription().to_string())?;
                }
            }
            start_stream(&stream)?;
            Ok(Self {
                stream,
                _delegate: delegate,
                _queue: queue,
                sink,
                counters,
                stopped: AtomicBool::new(false),
            })
        }

        pub(super) fn stop(&self) {
            if self.stopped.swap(true, Ordering::AcqRel) {
                return;
            }
            if let Ok(mut sink) = self.sink.lock() {
                sink.finalize_fragment();
            }
            unsafe { self.stream.stopCaptureWithCompletionHandler(None) };
        }

        pub(super) fn delivered_samples(&self) -> (u64, u64, u64) {
            (
                self.counters.screen.load(Ordering::Relaxed),
                self.counters.system_audio.load(Ordering::Relaxed),
                self.counters.microphone.load(Ordering::Relaxed),
            )
        }
    }

    impl Drop for NativeCaptureSession {
        fn drop(&mut self) {
            self.stop();
        }
    }

    fn shareable_content() -> Result<Retained<SCShareableContent>, String> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let completion = RcBlock::new(
            move |content: *mut SCShareableContent, error: *mut NSError| {
                let result = if !content.is_null() {
                    unsafe { Retained::retain(content) }
                        .ok_or_else(|| "SCREEN_CAPTURE_CONTENT_UNAVAILABLE".to_owned())
                } else if let Some(error) = unsafe { error.as_ref() } {
                    Err(error.localizedDescription().to_string())
                } else {
                    Err("SCREEN_CAPTURE_CONTENT_UNAVAILABLE".to_owned())
                };
                let _ = sender.send(result);
            },
        );
        unsafe { SCShareableContent::getShareableContentWithCompletionHandler(&completion) };
        receiver
            .recv_timeout(CALLBACK_TIMEOUT)
            .map_err(|_| "SCREEN_CAPTURE_CONTENT_TIMEOUT".to_owned())?
    }

    fn start_stream(stream: &SCStream) -> Result<(), String> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let completion = RcBlock::new(move |error: *mut NSError| {
            let result = unsafe { error.as_ref() }
                .map(|error| Err(error.localizedDescription().to_string()))
                .unwrap_or(Ok(()));
            let _ = sender.send(result);
        });
        unsafe { stream.startCaptureWithCompletionHandler(Some(&completion)) };
        receiver
            .recv_timeout(CALLBACK_TIMEOUT)
            .map_err(|_| "SCREEN_CAPTURE_START_TIMEOUT".to_owned())?
    }

    fn copy_pcm_bytes(sample: &CMSampleBuffer) -> Option<(Vec<u8>, u32, bool, f64, u32)> {
        let format = unsafe { sample.format_description()? };
        let description = unsafe { CMAudioFormatDescriptionGetStreamBasicDescription(&format) };
        let description = unsafe { description.as_ref()? };
        if description.mFormatID != kAudioFormatLinearPCM {
            return None;
        }
        let mut required_size = 0_usize;
        // SAFETY: the first call only queries the required flexible AudioBufferList size.
        unsafe {
            sample.audio_buffer_list_with_retained_block_buffer(
                &mut required_size,
                std::ptr::null_mut(),
                0,
                None,
                None,
                kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment,
                std::ptr::null_mut(),
            );
        }
        if required_size < std::mem::size_of::<AudioBufferList>() {
            return None;
        }
        let word_size = std::mem::size_of::<usize>();
        let mut storage = vec![0_usize; required_size.div_ceil(word_size)];
        let list = storage.as_mut_ptr().cast::<AudioBufferList>();
        // SAFETY: storage is aligned and at least required_size bytes; sample remains retained
        // throughout extraction and bytes are copied before storage is released.
        let status = unsafe {
            sample.audio_buffer_list_with_retained_block_buffer(
                &mut required_size,
                list,
                storage.len() * word_size,
                None,
                None,
                kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment,
                std::ptr::null_mut(),
            )
        };
        if status != 0 {
            return None;
        }
        // SAFETY: CoreMedia initialized list after the successful call above.
        let bytes = copy_audio_buffer_list_bytes(unsafe { &*list })?;
        Some((
            bytes,
            description.mBitsPerChannel,
            description.mFormatFlags & kAudioFormatFlagIsFloat != 0,
            description.mSampleRate,
            description.mChannelsPerFrame,
        ))
    }

    fn append_sample_envelope(
        destination: &mut Vec<u8>,
        output_type: SCStreamOutputType,
        sample_rate: f64,
        channels: u32,
        bits: u32,
        is_float: bool,
        bytes: &[u8],
    ) {
        destination.push(if output_type == SCStreamOutputType::Audio {
            1
        } else {
            2
        });
        destination.extend_from_slice(&sample_rate.to_le_bytes());
        destination.extend_from_slice(&channels.to_le_bytes());
        destination.extend_from_slice(&bits.to_le_bytes());
        destination.push(u8::from(is_float));
        destination.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        destination.extend_from_slice(bytes);
    }

    fn persist_fragment(
        app: &tauri::AppHandle,
        owner_user_id: &str,
        fragment: RecordingFragment,
        ended_at_ms: i64,
        bytes: &[u8],
    ) -> Result<(), &'static str> {
        let state = app.state::<ProfileCommandState>();
        let sequence = fragment.sequence();
        let meeting_id = fragment.meeting_id().to_owned();
        let storage_key =
            StorageKey::parse(format!("recordings/{meeting_id}/fragment-{sequence}.icaf"))
                .map_err(|_| "CAPTURE_FRAGMENT_KEY_INVALID")?;
        let checksum = format!("{:x}", Sha256::digest(bytes));
        let pending = fragment
            .complete(
                ended_at_ms,
                storage_key.as_str(),
                bytes.len() as u64,
                &checksum,
            )
            .map_err(|_| "CAPTURE_FRAGMENT_INVALID")?;
        state
            .files
            .write(&storage_key, bytes)
            .map_err(|_| "CAPTURE_FRAGMENT_WRITE_FAILED")?;
        let database = state
            .database
            .lock()
            .map_err(|_| "CAPTURE_STORAGE_UNAVAILABLE")?;
        let inserted = database.connection().execute(
            "INSERT INTO artifacts(id, meeting_id, kind, storage_key, mime_type, byte_length, \
             checksum, content_status, created_at_ms, expires_at_ms) \
             SELECT ?1, m.id, 'audio_chunk', ?2, 'application/x-interview-audio-fragment', \
                    ?3, ?4, 'pending', ?5, m.retention_expires_at_ms \
             FROM meetings m JOIN launch_policies lp ON lp.id = m.launch_policy_id \
             WHERE m.id = ?6 AND lp.owner_user_id = ?7 AND m.status = 'running'",
            rusqlite::params![
                &pending.id,
                storage_key.as_str(),
                pending.byte_length as i64,
                &pending.checksum,
                pending.started_at_ms,
                &pending.meeting_id,
                owner_user_id,
            ],
        );
        if !matches!(inserted, Ok(1)) {
            let _ = state.files.delete(&storage_key);
            return Err("CAPTURE_FRAGMENT_METADATA_FAILED");
        }
        let vetted = pending
            .vet_audio_bytes(bytes.len())
            .map_err(|_| "CAPTURE_FRAGMENT_VETTING_FAILED")?;
        let status = if vetted.is_attachable() {
            "allowed"
        } else {
            "rejected"
        };
        let updated = database
            .connection()
            .execute(
                "UPDATE artifacts SET content_status = ?2 \
                 WHERE id = ?1 AND content_status = 'pending'",
                rusqlite::params![&vetted.id, status],
            )
            .map_err(|_| "CAPTURE_FRAGMENT_VETTING_FAILED")?;
        if updated != 1 {
            let _ = state.files.delete(&storage_key);
            return Err("CAPTURE_FRAGMENT_VETTING_FAILED");
        }
        if !vetted.is_attachable() {
            let _ = state.files.delete(&storage_key);
        }
        drop(database);
        let _ = app.emit("capture://artifact", &vetted.id);
        Ok(())
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }
}
