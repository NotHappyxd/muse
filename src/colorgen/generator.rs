use image::DynamicImage;
use crate::colorgen::conversions;
use crate::colorgen::conversions::lab_to_rgb;
use crate::colorgen::kmeans::{color_histogram, kmeans_recode, Cluster, Lab};

pub struct Theme {
    pub main: [u8; 3],
    pub accent: [u8; 3],
}
pub fn generate_from_image(image: &DynamicImage, account_light: bool) -> Theme {
    let clusters = kmeans_recode(&color_histogram(&image), 6, 30);

    let total_pixels = clusters.iter().map(|cluster| cluster.size).sum();

    let main = clusters.first().unwrap();
    let accent = clusters.iter()
        .skip(1)
        .max_by(|x, y| {
            accent_score(x, main, total_pixels, account_light).partial_cmp(
                &accent_score(y, main, total_pixels, account_light)
            ).unwrap()
        })
        .unwrap();

    Theme {
        main: readable_accent_lab(main.color),
        accent: lab_to_rgb(accent.color)
    }
}

fn accent_score(cluster: &Cluster, main: &Cluster, total_pixels: f32, account_light: bool) -> f32 {
    let chroma = cluster.color.chroma();
    let contrast = cluster.color.contrast(&main.color);
    let lightness_diff = ((cluster.color.l - main.color.l).abs() / 100.0).max(0.2);
    let size_weight = (cluster.size / total_pixels).sqrt();

    chroma * contrast * size_weight * if account_light { lightness_diff } else { 1.0 }
}

pub fn readable_accent_lab(lab: Lab) -> [u8; 3] {
    readable_accent([lab.l, lab.a, lab.b])
}

pub fn readable_accent(lab: [f32; 3]) -> [u8; 3] {
    const MIN_L: f32 = 45.0; // floor: avoid colors too dark to read on dark bg
    const MAX_L: f32 = 90.0; // ceiling: avoid near-white, low-contrast accents

    let clamped = [lab[0].clamp(MIN_L, MAX_L), lab[1], lab[2]];

    conversions::lab_arr_to_rgb(clamped)
}