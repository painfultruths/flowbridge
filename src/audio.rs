use rodio::{OutputStream, Sink, Source};
use std::time::Duration;

// Generate a chime-like tone with harmonics and reverb
struct ChimeSource {
    frequency: f32,
    sample_rate: u32,
    num_samples: usize,
    current_sample: usize,
    reverb_buffer: Vec<f32>,
    reverb_delays: Vec<usize>,
}

impl ChimeSource {
    fn new(frequency: f32, duration_ms: u64) -> Self {
        let sample_rate = 48000;
        let num_samples = (sample_rate as u64 * duration_ms / 1000) as usize;

        // Reverb delays in samples (different delay times for depth)
        let reverb_delays = vec![
            (sample_rate as f32 * 0.03) as usize,  // 30ms
            (sample_rate as f32 * 0.05) as usize,  // 50ms
            (sample_rate as f32 * 0.08) as usize,  // 80ms
        ];

        let max_delay = *reverb_delays.iter().max().unwrap();
        let reverb_buffer = vec![0.0; max_delay];

        ChimeSource {
            frequency,
            sample_rate,
            num_samples,
            current_sample: 0,
            reverb_buffer,
            reverb_delays,
        }
    }

    fn generate_chime_sample(&self, t: f32) -> f32 {
        let pi2 = 2.0 * std::f32::consts::PI;

        // Bell-like harmonics (non-integer ratios for realistic chime)
        let fundamental = (t * self.frequency * pi2).sin() * 0.4;
        let harmonic1 = (t * self.frequency * 2.76 * pi2).sin() * 0.15;
        let harmonic2 = (t * self.frequency * 5.40 * pi2).sin() * 0.10;
        let harmonic3 = (t * self.frequency * 8.93 * pi2).sin() * 0.05;

        fundamental + harmonic1 + harmonic2 + harmonic3
    }
}

impl Iterator for ChimeSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_sample >= self.num_samples {
            return None;
        }

        let t = self.current_sample as f32 / self.sample_rate as f32;

        // Generate base chime sound
        let mut value = self.generate_chime_sample(t);

        // Exponential decay envelope for bell-like sound
        let decay = (-t * 3.0).exp();

        // Quick attack
        let attack = if self.current_sample < 100 {
            self.current_sample as f32 / 100.0
        } else {
            1.0
        };

        value *= decay * attack;

        // Add reverb by mixing with delayed samples
        let buffer_pos = self.current_sample % self.reverb_buffer.len();
        let mut reverb_sum = 0.0;
        for (i, &delay) in self.reverb_delays.iter().enumerate() {
            if self.current_sample >= delay {
                let delay_pos = (self.current_sample - delay) % self.reverb_buffer.len();
                let attenuation = 0.3 / (i + 1) as f32; // Each echo gets quieter
                reverb_sum += self.reverb_buffer[delay_pos] * attenuation;
            }
        }

        // Store current sample in reverb buffer
        self.reverb_buffer[buffer_pos] = value;

        // Mix original with reverb
        let final_value = (value + reverb_sum) * 0.25; // Overall volume

        self.current_sample += 1;
        Some(final_value)
    }
}

impl Source for ChimeSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis((self.num_samples as u64 * 1000) / self.sample_rate as u64))
    }
}

// Play a perfect 5th chime (C5 to G5) with bell-like harmonics and reverb
pub fn play_completion_chime() {
    std::thread::spawn(|| {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            let sink = Sink::try_new(&stream_handle).unwrap();

            // C5 note (523.25 Hz) for 350ms
            let c5 = ChimeSource::new(523.25, 350);
            sink.append(c5);

            // Small gap
            std::thread::sleep(Duration::from_millis(80));

            // G5 note (783.99 Hz) for 500ms - the perfect 5th, higher and longer
            let g5 = ChimeSource::new(783.99, 500);
            sink.append(g5);

            sink.sleep_until_end();
        }
    });
}
