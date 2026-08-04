#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VadConfig {
    threshold: f32,
    hangover_frames: u16,
}

impl VadConfig {
    pub fn new(threshold: f32, hangover_frames: u16) -> Result<Self, &'static str> {
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err("VAD threshold must be between zero and one");
        }
        if hangover_frames == 0 {
            return Err("VAD hangover must contain at least one frame");
        }
        Ok(Self {
            threshold,
            hangover_frames,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VadEvent {
    Silent,
    SpeechStarted,
    SpeechContinued,
    SpeechStopped,
}

#[derive(Debug)]
pub struct VadState {
    config: VadConfig,
    speaking: bool,
    quiet_frames: u16,
}

impl VadState {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            speaking: false,
            quiet_frames: 0,
        }
    }

    pub fn observe(&mut self, normalized_rms: f32) -> VadEvent {
        let audible = normalized_rms.is_finite() && normalized_rms >= self.config.threshold;
        if audible {
            self.quiet_frames = 0;
            return if std::mem::replace(&mut self.speaking, true) {
                VadEvent::SpeechContinued
            } else {
                VadEvent::SpeechStarted
            };
        }

        if !self.speaking {
            return VadEvent::Silent;
        }
        self.quiet_frames = self.quiet_frames.saturating_add(1);
        if self.quiet_frames < self.config.hangover_frames {
            return VadEvent::SpeechContinued;
        }

        self.speaking = false;
        self.quiet_frames = 0;
        VadEvent::SpeechStopped
    }
}

pub fn normalized_pcm_rms(bytes: &[u8], bits_per_channel: u32, float: bool) -> Option<f32> {
    let (sum, count) = match (float, bits_per_channel) {
        (true, 32) => bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_ne_bytes(chunk.try_into().expect("four-byte chunk")))
            .filter(|sample| sample.is_finite())
            .fold((0.0_f64, 0_u64), |(sum, count), sample| {
                (sum + f64::from(sample).powi(2), count + 1)
            }),
        (false, 16) => bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_ne_bytes(chunk.try_into().expect("two-byte chunk")))
            .fold((0.0_f64, 0_u64), |(sum, count), sample| {
                let normalized = f64::from(sample) / f64::from(i16::MAX);
                (sum + normalized.powi(2), count + 1)
            }),
        (false, 32) => bytes
            .chunks_exact(4)
            .map(|chunk| i32::from_ne_bytes(chunk.try_into().expect("four-byte chunk")))
            .fold((0.0_f64, 0_u64), |(sum, count), sample| {
                let normalized = f64::from(sample) / f64::from(i32::MAX);
                (sum + normalized.powi(2), count + 1)
            }),
        _ => return None,
    };
    (count > 0).then(|| (sum / count as f64).sqrt().clamp(0.0, 1.0) as f32)
}
