use crate::colorgen::colors::Lab;
use crate::colorgen::conversions;
use crate::colorgen::kmeans::Cluster;

pub fn main_score(cluster: &Cluster, total_pixels: f32) -> f32 {
    const CHROMA_WEIGHT: f32 = 0.35;
    const SATURATION_WEIGHT: f32 = 0.2;
    const PRESENCE_WEIGHT: f32 = 0.45;

    let presence = cluster.size / total_pixels;
    let chroma = cluster.color.chroma();
    let saturation = conversions::to_hsl(&cluster.color)[1];

    (CHROMA_WEIGHT * chroma)
        + (SATURATION_WEIGHT * saturation * 100.0)
        + (PRESENCE_WEIGHT * presence * 100.0)
}

pub fn accent_score(cluster: &Cluster, main: &Cluster, total_pixels: f32, account_light: bool) -> f32 {
    let chroma = (cluster.color.chroma() / 0.30).clamp(0.0, 1.0);
    let size_weight = (cluster.size / total_pixels).sqrt();
    let lightness_diff = ((cluster.color.l - main.color.l).abs() / 0.4).clamp(0.2, 1.0);

    let hue = |lab: &Lab| lab.b.atan2(lab.a);
    let mut hue_diff = (hue(&cluster.color) - hue(&main.color)).abs();
    if hue_diff > std::f32::consts::PI {
        hue_diff = 2.0 * std::f32::consts::PI - hue_diff;
    }
    let hue_distinctness = (hue_diff / (std::f32::consts::PI / 2.0)).min(1.0); // caps at 90°

    let accent_score = chroma * hue_distinctness * size_weight * if account_light { lightness_diff } else { 1.0 };

    accent_score
}