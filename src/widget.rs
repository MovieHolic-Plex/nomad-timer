use crate::presets::{Preset, find_preset};
use crate::api_contract::BroadcastMessage;
use crate::timer::{TimerState, WorkPhase};

pub const VISIBLE_PRESET_IDS: [&str; 6] = [
    "rest-start",
    "stretch",
    "water",
    "wave",
    "cheer",
    "back-to-work",
];

pub const DEFAULT_CHAT_LINE: &str = "서버에 연결되면 모두에게 보여요.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatVariant {
    Work,
    Break,
    Stretch,
    Cheer,
}

impl CatVariant {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Work => "집중 고양이",
            Self::Break => "휴식 고양이",
            Self::Stretch => "기지개 고양이",
            Self::Cheer => "응원 고양이",
        }
    }
}

pub fn visible_presets() -> Vec<Preset> {
    VISIBLE_PRESET_IDS
        .iter()
        .filter_map(|id| find_preset(id))
        .collect()
}

pub fn preset_at_x(x: i32, width: i32) -> Option<&'static str> {
    if x < 0 || width <= 0 {
        return None;
    }
    let count = i32::try_from(VISIBLE_PRESET_IDS.len()).ok()?;
    let button_width = width / count;
    if button_width <= 0 || x >= width {
        return None;
    }
    let index = usize::try_from((x / button_width).min(count - 1)).ok()?;
    VISIBLE_PRESET_IDS.get(index).copied()
}

pub fn latest_chat_line(messages: &[BroadcastMessage]) -> String {
    messages
        .first()
        .map(|message| format!("{}: {}", message.sender, message.label))
        .unwrap_or_else(|| DEFAULT_CHAT_LINE.to_owned())
}

pub fn cat_variant(timer: TimerState, latest_preset_id: Option<&str>) -> CatVariant {
    match latest_preset_id {
        Some("stretch") => CatVariant::Stretch,
        Some("cheer") | Some("wave") => CatVariant::Cheer,
        _ => match timer.phase {
            WorkPhase::Work => CatVariant::Work,
            WorkPhase::Break => CatVariant::Break,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(preset_id: &str, label: &str) -> BroadcastMessage {
        BroadcastMessage {
            preset_id: preset_id.to_owned(),
            label: label.to_owned(),
            message: "준비된 메시지".to_owned(),
            sender: "alice".to_owned(),
            sent_at_ms: 1,
        }
    }

    #[test]
    fn visible_presets_include_social_break_reactions() {
        // Given: the Windows widget exposes a compact preset bar.
        let presets = visible_presets();

        // When: visible IDs are collected.
        let ids: Vec<&str> = presets.iter().map(|preset| preset.id).collect();

        // Then: break chat has water, wave, stretch, cheer, and return actions.
        assert_eq!(ids.len(), 6);
        assert!(ids.contains(&"water"));
        assert!(ids.contains(&"wave"));
        assert!(ids.contains(&"back-to-work"));
    }

    #[test]
    fn preset_hit_testing_scales_to_six_buttons() {
        // Given: the widget has six equally sized preset buttons.
        let width = 252;

        // When: the first, middle, and last button slots are clicked.
        let first = preset_at_x(1, width);
        let middle = preset_at_x(104, width);
        let last = preset_at_x(251, width);

        // Then: hit testing maps to the server preset IDs.
        assert_eq!(first, Some("rest-start"));
        assert_eq!(middle, Some("water"));
        assert_eq!(last, Some("back-to-work"));
    }

    #[test]
    fn chat_line_prefers_latest_server_broadcast() {
        // Given: the server returns recent preset broadcasts newest-first.
        let messages = vec![message("wave", "손 흔들기")];

        // When: the widget formats its compact chat line.
        let line = latest_chat_line(&messages);

        // Then: users see who sent the latest prepared reaction.
        assert_eq!(line, "alice: 손 흔들기");
    }

    #[test]
    fn cat_variant_reacts_to_phase_and_latest_preset() {
        // Given: the timer is in break mode.
        let timer = TimerState {
            phase: WorkPhase::Break,
            remaining_secs: 120,
        };

        // When: no preset is active, then a stretch preset arrives.
        let resting = cat_variant(timer, None);
        let stretching = cat_variant(timer, Some("stretch"));

        // Then: the widget can draw the right pixel-cat mood.
        assert_eq!(resting, CatVariant::Break);
        assert_eq!(stretching, CatVariant::Stretch);
    }
}
