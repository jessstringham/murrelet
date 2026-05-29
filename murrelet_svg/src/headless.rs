use murrelet_draw::drawable::{DrawTarget, MixedDrawableShape, ToMixedDrawables};
use murrelet_draw::style::{MurreletPath, StyledPath};
use murrelet_perform::perform::SvgDrawConfig;

use crate::svg::{StyledText, SvgPathCache, SvgPathCacheRef};


// claude added this for being able to produce svgs without making a nannou window or web app.
// Headless svg is the DrawTarget::Svg medium, so a sketch that overrides
// to_mixed_drawables_for can return pen-friendly geometry here.
pub fn add_mixed_drawables<D: ToMixedDrawables>(cache: &SvgPathCacheRef, v: &D) {
    for shape in v.to_mixed_drawables_for(DrawTarget::Svg) {
        let style = shape.style();
        match shape {
            MixedDrawableShape::Shape(s) => {
                let annotations = s.annotations();
                for cd in s.curves() {
                    let path = MurreletPath::curve(cd.clone());
                    let sp = if annotations.is_empty() {
                        StyledPath::new_from_path(path, style.to_style())
                    } else {
                        StyledPath::new_from_path_with_multiple_annotations(
                            path,
                            style.to_style(),
                            annotations.vals().clone(),
                        )
                    };
                    cache.add_styled_path("", sp);
                }
            }
            MixedDrawableShape::Text(text) => {
                for t in text.positions() {
                    cache.add_styled_text(
                        "",
                        StyledText::new(t.text().to_string(), t.loc(), 180.0, style.to_style()),
                    );
                }
            }
        }
    }
}

pub fn render_to_svg<D: ToMixedDrawables>(v: &D, svg_draw_config: &SvgDrawConfig) {
    let cache = SvgPathCache::svg_draw(svg_draw_config);
    add_mixed_drawables(&cache, v);
    cache.save_doc();
}
