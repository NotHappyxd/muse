use image::DynamicImage;
use crate::colorgen::conversions;
use crate::colorgen::kmeans::{color_histogram, kmeans, Cluster, Lab};

pub struct Theme {
    pub main: [u8; 3],
    pub accent: [u8; 3],
}
pub fn generate_from_image(image: &DynamicImage, k_clusters: usize, max_iterations: usize, account_light: bool) -> Theme {
    let clusters = kmeans(&color_histogram(&image), k_clusters, max_iterations);

    let total_pixels = clusters.iter().map(|cluster| cluster.size).sum();

    const MIN_DIST_SQ: f32 = 0.0225;

    let main = clusters.first().unwrap();
    let accent = clusters.iter()
        .filter(|c| c.color.distance(&main.color) > MIN_DIST_SQ)
        .skip(1)
        .max_by(|x, y| {
            accent_score(x, main, total_pixels, account_light).partial_cmp(
                &accent_score(y, main, total_pixels, account_light)
            ).unwrap()
        });

    let accent_lab = if let Some(cluster) = accent {
        cluster.color
    }else {
        synthesize_harmonic_accent(&main.color)
    };

    let main_color = nudge_for_contrast(&main.color, [13, 13, 13], 3.0, 0.2, 0.82);
    Theme {
        main: main_color,
        accent: nudge_for_contrast(&accent_lab, main_color, 4.5, 0.35, 0.82)
    }
}

fn accent_score(cluster: &Cluster, main: &Cluster, total_pixels: f32, account_light: bool) -> f32 {
    let chroma = cluster.color.chroma();
    let contrast = cluster.color.accent_distance_squared(&main.color).sqrt();
    let lightness_diff = ((cluster.color.l - main.color.l).abs() / 100.0).max(0.2);
    let size_weight = (cluster.size / total_pixels).sqrt();

    chroma * contrast * size_weight * if account_light { lightness_diff } else { 1.0 }
}

// https://github.com/Harman1307/iris/blob/main/iris/generator.py#L125
pub fn nudge_for_contrast(
    color: &Lab,
    background_rgb: [u8; 3],
    target_ratio: f32,
    hard_min: f32,
    hard_max: f32
) -> [u8; 3] {
    let mut hsl = conversions::to_hsl(&color);
    hsl[2] = hsl[2].clamp(hard_min, hard_max);

    // True if the foreground color is currently lighter than the background
    let is_lighter = luminance(conversions::hsl_to_rgb(hsl)) >= luminance(background_rgb);

    let mut low = if is_lighter { hsl[2] } else { hard_min };
    let mut high = if is_lighter { hard_max } else { hsl[2] };

    for _ in 0..12 {
        let mid = (low + high) / 2.0;
        let rgb = conversions::hsl_to_rgb([hsl[0], hsl[1], mid]);

        if wcag_contrast(rgb, background_rgb) >= target_ratio {
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

    conversions::hsl_to_rgb([hsl[0], hsl[1], if is_lighter { high } else { low }])
}

fn luminance(rgb: [u8; 3]) -> f32 {
    let [r, g, b] = rgb.map(|color| color as f32 / 255.0)
        .map(|normal| if normal <= 0.03928 {
            normal / 12.92
        }else {
            ((normal + 0.055) / 1.055).powf(2.4)
        });

    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub fn wcag_contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
    let la = luminance(a);
    let lb = luminance(b);
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn synthesize_harmonic_accent(base_color: &Lab) -> Lab {
    let mut hsl = conversions::to_hsl(base_color);
    hsl[0] = (hsl[0] + 120.0) % 360.0; // Triadic hue shift
    hsl[1] = (hsl[1] * 1.3).clamp(0.4, 0.95); // Boost saturation for visibility

    let rgb = conversions::hsl_to_rgb(hsl);
    conversions::rgb_to_oklab(rgb)
}