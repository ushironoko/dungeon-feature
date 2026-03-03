use crate::manifest::{SpriteCategory, SpriteSpec};

const BASE_PROMPT: &str = "\
Create a single 32x32 pixel art sprite with transparent background. \
Style: cute chibi pixel art, warm vibrant colors, retro RPG style. \
Centered in 32x32 canvas. Clear outlines, limited palette (8-16 colors). \
Match the visual style: cute chibi characters with soft pixel shading.";

fn category_prompt(spec: &SpriteSpec) -> String {
    match spec.category {
        SpriteCategory::Enemy => {
            format!(
                "Enemy monster sprite: {}. Threatening but cute (chibi). Front-facing idle pose.",
                spec.description
            )
        }
        SpriteCategory::Item => {
            format!(
                "Equipment/item icon: {}. Clean silhouette, recognizable at small size. Subtle shading.",
                spec.description
            )
        }
    }
}

pub fn build_prompt(spec: &SpriteSpec) -> String {
    format!("{}\n{}", BASE_PROMPT, category_prompt(spec))
}
