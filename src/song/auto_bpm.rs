//! Per-audio-clip BPM metadata, separate from the song's global tempo and
//! from the sampler's playback speed.
//!
//! Stored as a small Honeydew-owned `HDBP` child of `CLIP`. The existing
//! FLM reader/writer preserves unknown clip children, so older projects load
//! without migration and round-trips retain this data without changing the
//! audio sample or the source file.

use super::{Clip, RawChunk};

const TAG: [u8; 4] = *b"HDBP";
const VERSION: u32 = 1;
const SIZE: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipBpm {
    pub bpm: f32,
    /// Heuristic score, not a probability of being correct.
    pub confidence: f32,
    /// True when read from the audio file's own ACID/FLSM tempo metadata;
    /// false when computed from transients.
    pub embedded: bool,
}

impl ClipBpm {
    pub fn valid(self) -> bool {
        self.bpm.is_finite()
            && (1.0..=999.0).contains(&self.bpm)
            && self.confidence.is_finite()
            && (0.0..=1.0).contains(&self.confidence)
    }
}

/// BPM stored in a clip, if a valid v1 `HDBP` chunk exists.
pub fn read(clip: &Clip) -> Option<ClipBpm> {
    let data = (clip.file.unknown_chunks.iter())
        .find(|chunk| chunk.id == TAG && chunk.payload.len() == SIZE)?
        .payload
        .as_slice();
    let word = |at| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
    if word(0) != VERSION {
        return None;
    }
    let info = ClipBpm {
        bpm: f32::from_bits(word(4)),
        confidence: f32::from_bits(word(8)),
        embedded: match word(12) {
            0 => false,
            1 => true,
            _ => return None,
        },
    };
    info.valid().then_some(info)
}

/// Replace only Honeydew's own tempo tag. `None` clears stale tempo when
/// the clip's source file is replaced with an undetectable sample.
pub fn write(clip: &mut Clip, info: Option<ClipBpm>) {
    clip.file.unknown_chunks.retain(|chunk| chunk.id != TAG);
    if let Some(info) = info.filter(|info| info.valid()) {
        let mut payload = Vec::with_capacity(SIZE);
        for value in [
            VERSION,
            info.bpm.to_bits(),
            info.confidence.to_bits(),
            u32::from(info.embedded),
        ] {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        clip.file.unknown_chunks.push(RawChunk { id: TAG, payload });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::{Lane, LaneKind};

    #[test]
    fn writes_reads_replaces_and_clears_per_clip_bpm() {
        let mut lane = Lane::new(LaneKind::Audio);
        let clip = lane.add_clip(0, 4.0);
        let guessed = ClipBpm {
            bpm: 127.83,
            confidence: 0.76,
            embedded: false,
        };
        write(clip, Some(guessed));
        assert_eq!(read(clip), Some(guessed));
        write(
            clip,
            Some(ClipBpm {
                embedded: true,
                ..guessed
            }),
        );
        assert_eq!(clip.file.unknown_chunks.len(), 1);
        assert_eq!(read(clip).unwrap().embedded, true);
        write(clip, None);
        assert_eq!(read(clip), None);
        assert!(clip.file.unknown_chunks.is_empty());
    }

    #[test]
    fn survives_a_real_project_write_and_reload() {
        use crate::song::{ChannelKind, Song};

        let mut song = Song::default();
        song.add_channel(ChannelKind::AudioClip);
        let clip = song.channels[1].lanes[0].add_clip(0, 4.0);
        let info = ClipBpm {
            bpm: 127.83,
            confidence: 0.87,
            embedded: false,
        };
        write(clip, Some(info));
        let bytes = crate::flm::write(&song).expect("write project");
        let opened = crate::flm::read(&bytes).expect("open project");
        assert_eq!(read(&opened.channels[1].lanes[0].clips[0]), Some(info));
        assert_eq!(opened.tempo_bpm, 120.0, "the song tempo is independent");
    }

    #[test]
    fn ignores_corrupted_unknown_chunk_payloads() {
        let mut lane = Lane::new(LaneKind::Audio);
        let clip = lane.add_clip(0, 4.0);
        write(
            clip,
            Some(ClipBpm {
                bpm: 130.0,
                confidence: 0.8,
                embedded: false,
            }),
        );
        clip.file.unknown_chunks[0].payload[4..8]
            .copy_from_slice(&f32::NAN.to_le_bytes());
        assert_eq!(read(clip), None);
        write(clip, None);
        assert!(clip.file.unknown_chunks.is_empty());
    }

    #[test]
    fn preserves_unrelated_unknown_children() {
        let mut lane = Lane::new(LaneKind::Audio);
        let clip = lane.add_clip(0, 4.0);
        let other = RawChunk { id: *b"TEST", payload: vec![1, 2, 3] };
        clip.file.unknown_chunks.push(other.clone());
        write(clip, Some(ClipBpm { bpm: 120.0, confidence: 1.0, embedded: true }));
        write(clip, None);
        assert_eq!(clip.file.unknown_chunks, vec![other]);
    }
}
