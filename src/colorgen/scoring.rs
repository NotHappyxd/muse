use crate::colorgen::kmeans::Cluster;

pub fn main_score(cluster: &Cluster, total_pixels: f32) -> f32 {
    let size_weight = cluster.size / total_pixels;
    let chroma = cluster.color.chroma();

    let saturation_boost = 0.2 + 0.8 * (chroma / (chroma + 0.05));

    size_weight * saturation_boost
}

pub fn accent_score(cluster: &Cluster, main: &Cluster, total_pixels: f32, account_light: bool) -> f32 {
    let chroma = cluster.color.chroma();
    let contrast = cluster.color.accent_distance_squared(&main.color).sqrt();
    let lightness_diff = ((cluster.color.l - main.color.l).abs() / 100.0).max(0.2);
    let size_weight = (cluster.size / total_pixels).sqrt();

    chroma * contrast * size_weight * if account_light { lightness_diff } else { 1.0 }
}