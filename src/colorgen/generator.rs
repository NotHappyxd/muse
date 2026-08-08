use crate::colorgen::colors::{find_max_chroma_oklab, wcag_contrast, Lab, Oklch};
use crate::colorgen::conversions::{oklab_to_rgb, rgb_to_oklab};
use crate::colorgen::kmeans::{color_histogram, kmeans};
use crate::colorgen::scoring::main_score;
use crate::colorgen::{colors, conversions, scoring};
use image::DynamicImage;
use crate::colorgen::adjustments::{generate_mix, nudge_accent, nudge_chroma_floor};

#[derive(Debug)]
pub struct Theme {
    pub main: [u8; 3],
    pub accent: [u8; 3],
    pub inactive: Option<[u8; 3]>,
    pub upcoming: Option<[u8; 3]>,
}

pub fn generate_from_image(
    image: &DynamicImage,
    k_clusters: usize,
    max_iterations: usize,
    min_chroma: f32,
    account_light: bool,
    min_chroma_percentage: f32
) -> Theme {
    let clusters = kmeans(&color_histogram(&image), k_clusters, max_iterations);

    const MIN_DIST_SQ: f32 = 0.0225;

    let total_pixels = clusters.iter().map(|cluster| cluster.size).sum();

    let main = clusters
        .iter()
        .filter(|c| c.color.chroma() >= min_chroma)
        .max_by(|a, b| {
            main_score(a, total_pixels)
                .partial_cmp(&main_score(b, total_pixels))
                .unwrap()
        })
        .unwrap_or(clusters.first().unwrap());

    let accent = clusters
        .iter()
        .filter(|c| c.color.distance(&main.color) > MIN_DIST_SQ)
        .filter(|c| c.color.chroma() >= min_chroma)
        .max_by(|x, y| {
            scoring::accent_score(x, main, total_pixels, account_light)
                .partial_cmp(&scoring::accent_score(y, main, total_pixels, account_light))
                .unwrap()
        });

    let mut accent_lab = if let Some(cluster) = accent {
        cluster.color
    } else {
        synthesize_harmonic_accent(&main.color)
    };

    accent_lab = nudge_chroma_floor(&mut accent_lab, min_chroma_percentage);

    let main_color = nudge_for_contrast(&main.color, [13, 13, 13], 3.0, 0.2, 0.82);
    let accent_color = nudge_accent(&accent_lab, main_color, 4.0, true);

    let main_lab = rgb_to_oklab(main_color);
    let accent_lab = rgb_to_oklab(accent_color);

    Theme {
        main: main_color,
        accent: oklab_to_rgb(&accent_lab),
        inactive: Some(generate_mix(&main_lab, &accent_lab, 0.75, 0.55)),
        upcoming: Some(generate_mix(&accent_lab, &main_lab, 0.45, 0.25)),
    }
}

// https://github.com/Harman1307/iris/blob/main/iris/generator.py#L125
pub fn nudge_for_contrast(
    color: &Lab,
    background_rgb: [u8; 3],
    target_ratio: f32,
    hard_min: f32,
    hard_max: f32,
) -> [u8; 3] {
    let mut lch = Oklch::from_oklab(color);
    lch.l = lch.l.clamp(hard_min, hard_max);

    let starting_rgb = oklab_to_rgb(&find_max_chroma_oklab(lch));
    let is_lighter = colors::luminance(starting_rgb) >= colors::luminance(background_rgb);

    let mut low = if is_lighter { lch.l } else { hard_min };
    let mut high = if is_lighter { hard_max } else { lch.l };
    let mut best_rgb = starting_rgb;

    for _ in 0..12 {
        let mid = (low + high) / 2.0;
        let mut test = lch;
        test.l = mid;
        let rgb = oklab_to_rgb(&find_max_chroma_oklab(test));

        if wcag_contrast(rgb, background_rgb) >= target_ratio {
            best_rgb = rgb;
            if is_lighter {
                high = mid;
            } else {
                low = mid;
            }
        } else {
            if is_lighter {
                low = mid;
            } else {
                high = mid;
            }
        }
    }

    best_rgb
}

fn synthesize_harmonic_accent(base_color: &Lab) -> Lab {
    if base_color.chroma() < 0.03 {
        return Lab {
            l: 0.90,
            a: 0.0,
            b: 0.0,
        };
    }

    let mut hsl = conversions::to_hsl(base_color);
    hsl[0] = (hsl[0] + 30.0) % 360.0; // Triadic hue shift
    hsl[1] = (hsl[1] * 1.2).clamp(0.3, 0.85); // Boost saturation for visibility

    let rgb = conversions::hsl_to_rgb(hsl);
    rgb_to_oklab(rgb)
}
