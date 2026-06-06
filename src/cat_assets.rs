use crate::widget::CatVariant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatBitmap {
    Work,
    Stretch,
    Sleep,
    Cheer,
    Water,
    Wave,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BmpInfo {
    pub width: i32,
    pub height: i32,
    pub bit_count: u16,
    pub pixel_offset: usize,
}

impl CatBitmap {
    pub const fn bytes(self) -> &'static [u8] {
        match self {
            Self::Work => include_bytes!("../assets/cats/runtime/cat-work.bmp"),
            Self::Stretch => include_bytes!("../assets/cats/runtime/cat-stretch.bmp"),
            Self::Sleep => include_bytes!("../assets/cats/runtime/cat-sleep.bmp"),
            Self::Cheer => include_bytes!("../assets/cats/runtime/cat-cheer.bmp"),
            Self::Water => include_bytes!("../assets/cats/runtime/cat-water.bmp"),
            Self::Wave => include_bytes!("../assets/cats/runtime/cat-wave.bmp"),
        }
    }

    pub fn info(self) -> Option<BmpInfo> {
        parse_bmp(self.bytes())
    }
}

pub fn bitmap_for_variant(variant: CatVariant, latest_preset_id: Option<&str>) -> CatBitmap {
    match latest_preset_id {
        Some("water") => CatBitmap::Water,
        Some("wave") => CatBitmap::Wave,
        Some("stretch") => CatBitmap::Stretch,
        Some("cheer") => CatBitmap::Cheer,
        _ => match variant {
            CatVariant::Work => CatBitmap::Work,
            CatVariant::Break => CatBitmap::Sleep,
            CatVariant::Stretch => CatBitmap::Stretch,
            CatVariant::Cheer => CatBitmap::Cheer,
        },
    }
}

pub fn parse_bmp(bytes: &[u8]) -> Option<BmpInfo> {
    if bytes.len() < 54 || bytes.get(0..2)? != b"BM" {
        return None;
    }
    Some(BmpInfo {
        width: i32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?),
        height: i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?),
        bit_count: u16::from_le_bytes(bytes.get(28..30)?.try_into().ok()?),
        pixel_offset: u32::from_le_bytes(bytes.get(10..14)?.try_into().ok()?) as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_cat_bitmaps_have_runtime_dimensions() {
        // Given: the Windows widget embeds generated cat image derivatives.
        let cats = [
            CatBitmap::Work,
            CatBitmap::Stretch,
            CatBitmap::Sleep,
            CatBitmap::Cheer,
            CatBitmap::Water,
            CatBitmap::Wave,
        ];

        // When: each BMP header is parsed.
        let infos: Vec<BmpInfo> = cats
            .iter()
            .map(|cat| cat.info().expect("cat bitmap should parse"))
            .collect();

        // Then: every runtime asset is a compact 64px bitmap usable by GDI.
        assert!(infos.iter().all(|info| info.width == 64));
        assert!(infos.iter().all(|info| info.height == 64));
        assert!(infos.iter().all(|info| info.bit_count == 24));
    }

    #[test]
    fn latest_preset_can_select_matching_cat_bitmap() {
        // Given: the widget has a generic break cat state.
        let variant = CatVariant::Break;

        // When: water and wave broadcasts are the latest messages.
        let water = bitmap_for_variant(variant, Some("water"));
        let wave = bitmap_for_variant(variant, Some("wave"));

        // Then: the real generated cat images can match the chat context.
        assert_eq!(water, CatBitmap::Water);
        assert_eq!(wave, CatBitmap::Wave);
    }
}
