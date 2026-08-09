use crate::colorgen::colors::{true_gamut_max_chroma, Lab, Oklch};
use crate::colorgen::conversions::oklab_to_rgb;
use crate::colorgen::generator::nudge_for_contrast;

pub fn nudge_accent(
    color: &Lab,
    background_rgb: [u8; 3],
    target_ratio: f32,
    background_is_dark: bool,
) -> Lab {
    let (floor, ceiling) = if background_is_dark {
        (0.58, 0.85)
    } else {
        (0.15, 0.42)
    };

    nudge_for_contrast(color, background_rgb, target_ratio, floor, ceiling)
}

pub fn generate_mix(main: &Lab, accent: &Lab, t: f32, chroma_scale: f32) -> [u8; 3] {
    let mut mix = mix_colors(accent, main, t);
    mix.scale_chroma(chroma_scale);

    oklab_to_rgb(&mix)
}

fn mix_colors(a: &Lab, b: &Lab, t: f32) -> Lab {
    Lab {
        l: a.l + (b.l - a.l) * t,
        a: a.a + (b.a - a.a) * t,
        b: a.b + (b.b - a.b) * t,
    }
}


pub fn nudge_chroma_floor(color: &mut Lab, min_chroma_percentage: f32) -> Lab {
    let current = color.chroma();

    if current < 0.02 { // Monochromatics are usually under 0.02, no need to alter appearance
        return *color;
    }

    let lch = Oklch::from_oklab(color);
    let true_max = true_gamut_max_chroma(lch.l, lch.h);
    let floor = true_max * min_chroma_percentage;

    if current >= floor {
        return *color;
    }

    color.with_chroma(floor);

    *color
}
