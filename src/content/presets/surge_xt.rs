//! Original Surge XT factory FXP names, packaged in the same order as the native
//! ARM64 Surge sound bank. Each Honeydew .flmpst selects one *complete* upstream
//! patch, not a recreation from a handful of parameters.
use std::sync::OnceLock;
use super::Preset;

pub(super) const COUNT: usize = 641;
pub(super) fn bank() -> &'static [Preset] {
    static BANK: OnceLock<Vec<Preset>> = OnceLock::new();
    BANK.get_or_init(|| {
        let list = include_str!("surge_xt_factory.tsv");
        let presets: Vec<Preset> = list.lines().enumerate().map(|(i, path)| {
            let (category, name) = path.rsplit_once('/').expect("Surge factory category");
            let name = name.strip_suffix(".fxp").expect("Surge FXP suffix");
            let params: &'static [(&'static str, f32)] =
                Box::leak(Box::new([("Factory Preset", (i + 1) as f32 / COUNT as f32)]));
            Preset {
                category,
                name,
                params,
                settings: &[],
            }
        }).collect();
        assert_eq!(presets.len(), COUNT);
        presets
    })
}
