//! Preallocated latency alignment for sources, rack insertions and post-fader sends.
use crate::mixer::{MixBuffer, Mixer};

pub struct StereoDelay {
    samples: Vec<[f32; 2]>,
    at: usize,
    delay: usize,
    old_delay: usize,
    fade: usize,
    written: usize,
}
impl StereoDelay {
    pub fn new(maximum: usize) -> Self {
        Self {
            samples: if maximum == 0 {
                Vec::new()
            } else {
                vec![[0.0; 2]; maximum + 1]
            },
            at: 0,
            delay: 0,
            old_delay: 0,
            fade: 0,
            written: 0,
        }
    }
    pub fn set_delay(&mut self, delay: usize) {
        let delay = delay.min(self.samples.len().saturating_sub(1));
        if delay != self.delay {
            self.old_delay = self.delay;
            self.delay = delay;
            self.fade = if self.written == 0 { 0 } else { 128 };
        }
    }
    pub fn reset(&mut self) {
        self.samples.fill([0.0; 2]);
        self.at = 0;
        self.written = 0;
        self.fade = 0;
    }
    pub fn frame(&mut self, input: [f32; 2]) -> [f32; 2] {
        if self.samples.is_empty() {
            return input;
        }
        let len = self.samples.len();
        self.samples[self.at] = input;
        let mut out = self.samples[(self.at + len - self.delay) % len];
        if self.fade > 0 {
            let old = self.samples[(self.at + len - self.old_delay) % len];
            let weight = self.fade as f32 / 128.0;
            for ch in 0..2 {
                out[ch] += (old[ch] - out[ch]) * weight;
            }
            self.fade -= 1;
        }
        self.at = (self.at + 1) % len;
        self.written = self.written.saturating_add(1);
        out
    }
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let out = self.frame([*l, *r]);
            *l = out[0];
            *r = out[1];
        }
    }
}
pub struct DelayCompensation {
    raw: Vec<StereoDelay>,
    sends: Vec<Vec<StereoDelay>>,
    inputs: Vec<usize>,
    max_inputs: Vec<usize>,
    outputs: Vec<usize>,
    racks: Vec<usize>,
    enabled: bool,
    maximum: usize,
}
impl DelayCompensation {
    /// Maximum rack delays bound every buffer before the renderer is published.
    pub fn new(mixer: &Mixer, maximum: &[usize]) -> Self {
        let count = maximum.len();
        let mut inputs = vec![0usize; count];
        let mut outputs = vec![0usize; count];
        for &source in mixer.order() {
            outputs[source] = inputs[source] + maximum[source];
            for send in mixer.sends(source) {
                inputs[send.target] = inputs[send.target].max(outputs[source]);
            }
        }
        let maximum_output = outputs[0];
        Self {
            raw: inputs.iter().map(|&d| StereoDelay::new(d)).collect(),
            sends: (0..count)
                .map(|source| {
                    mixer
                        .sends(source)
                        .iter()
                        .map(|send| StereoDelay::new(inputs[send.target]))
                        .collect()
                })
                .collect(),
            max_inputs: inputs.clone(),
            inputs: vec![0; count],
            outputs: vec![0; count],
            racks: vec![0; count],
            enabled: maximum_output > 0,
            maximum: maximum_output,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn maximum(&self) -> usize {
        self.maximum
    }
    pub fn output_latency(&self) -> usize {
        self.outputs[0]
    }
    pub fn input_latency(&self, channel: usize) -> usize {
        self.inputs[channel]
    }
    pub fn maximum_input_latency(&self, channel: usize) -> usize {
        self.max_inputs[channel]
    }
    pub fn set_rack_latency(&mut self, channel: usize, delay: usize) {
        self.racks[channel] = delay;
    }
    pub fn update(&mut self, mixer: &Mixer) {
        if !self.enabled {
            return;
        }
        self.inputs.fill(0);
        for &source in mixer.order() {
            self.outputs[source] = self.inputs[source] + self.racks[source];
            for send in mixer.sends(source) {
                self.inputs[send.target] = self.inputs[send.target].max(self.outputs[source]);
            }
        }
        for &source in mixer.order() {
            self.raw[source].set_delay(self.inputs[source]);
            for (index, send) in mixer.sends(source).iter().enumerate() {
                self.sends[source][index]
                    .set_delay(self.inputs[send.target].saturating_sub(self.outputs[source]));
            }
        }
    }
    pub fn delay_raw<B: MixBuffer>(&mut self, buffers: &mut [B]) {
        if self.enabled {
            for (delay, buffer) in self.raw.iter_mut().zip(buffers) {
                let (l, r) = buffer.stereo();
                delay.process(l, r);
            }
        }
    }
    pub(crate) fn send(&mut self, source: usize, index: usize, input: [f32; 2]) -> [f32; 2] {
        self.sends[source][index].frame(input)
    }
    pub fn reset(&mut self) {
        for d in &mut self.raw {
            d.reset();
        }
        for d in self.sends.iter_mut().flatten() {
            d.reset();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixer::{ChannelMix, OwnedStereoBuffer, SendMix};
    #[test]
    fn parallel_paths_recombine_at_the_same_sample() {
        let send = SendMix {
            target: 0,
            enabled: 1.0,
            level: 0.8531,
        };
        let strips = [
            ChannelMix {
                level: 0.8531,
                pan: 0.5,
                sends: vec![],
            },
            ChannelMix {
                level: 0.8531,
                pan: 0.5,
                sends: vec![send],
            },
            ChannelMix {
                level: 0.8531,
                pan: 0.5,
                sends: vec![send],
            },
        ];
        let mixer = Mixer::new(&strips).unwrap();
        let mut pdc = DelayCompensation::new(&mixer, &[0, 8, 0]);
        pdc.set_rack_latency(1, 8);
        pdc.update(&mixer);
        let mut buffers: Vec<_> = (0..3)
            .map(|_| OwnedStereoBuffer {
                left: vec![0.0; 32],
                right: vec![0.0; 32],
            })
            .collect();
        buffers[1].left[8] = 1.0;
        buffers[2].left[0] = 1.0;
        for stage in 0..mixer.stages().len() {
            mixer.route_stage_compensated(&mut buffers, stage, &mut pdc);
        }
        assert!(buffers[0].left[8] > 1.99);
        assert!(buffers[0].left[..8].iter().all(|v| *v == 0.0));
        assert_eq!(pdc.output_latency(), 8);
    }
}
