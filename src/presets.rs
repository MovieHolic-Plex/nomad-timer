use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub message: &'static str,
}

pub const PRESET_CATALOG: [Preset; 8] = [
    Preset {
        id: "rest-start",
        label: "쉬러가요",
        message: "쉬는시간이에요. 물 한 잔 하고 와요!",
    },
    Preset {
        id: "back-to-work",
        label: "복귀했어요",
        message: "다시 집중 모드로 돌아왔어요.",
    },
    Preset {
        id: "stretch",
        label: "기지개",
        message: "어깨 펴고 기지개 한 번!",
    },
    Preset {
        id: "water",
        label: "물 마시기",
        message: "물 한 잔 마시고 돌아와요.",
    },
    Preset {
        id: "wave",
        label: "손 흔들기",
        message: "잠깐 쉬러 왔어요. 다들 안녕!",
    },
    Preset {
        id: "breathe",
        label: "숨 고르기",
        message: "천천히 숨 고르고 다시 가요.",
    },
    Preset {
        id: "snack",
        label: "간식타임",
        message: "작은 간식으로 에너지 채워요.",
    },
    Preset {
        id: "cheer",
        label: "화이팅",
        message: "남은 시간도 가볍게 해봐요.",
    },
];

pub fn preset_catalog() -> &'static [Preset] {
    &PRESET_CATALOG
}

pub fn find_preset(id: &str) -> Option<Preset> {
    PRESET_CATALOG
        .iter()
        .copied()
        .find(|preset| preset.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_catalog_contains_korean_ready_messages() {
        // Given: the app exposes prepared messages only.
        let presets = preset_catalog();

        // When: the preset catalog is inspected.
        let labels: Vec<&str> = presets.iter().map(|preset| preset.label).collect();

        // Then: users can send friendly Korean break/work reactions.
        assert!(
            labels.contains(&"쉬러가요"),
            "expected a prepared Korean break preset"
        );
        assert!(
            labels.contains(&"복귀했어요"),
            "expected a prepared Korean return preset"
        );
        assert!(
            labels.contains(&"물 마시기"),
            "expected a prepared water preset"
        );
        assert!(
            labels.contains(&"손 흔들기"),
            "expected a prepared social preset"
        );
        assert!(
            presets.iter().all(|preset| !preset.id.trim().is_empty()),
            "preset IDs must be non-empty for safe server validation"
        );
    }
}
