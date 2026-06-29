use std::fs;
use tokio::sync::mpsc::UnboundedSender;
use crate::kmeans::{color_histogram, kmeans_recode, Color};
use crate::watcher::AppEvent;

pub async fn fetch_theme(art_url: String, tx: UnboundedSender<AppEvent>) {
    let bytes = match fetch_art_bytes(&art_url).await {
        Ok(bytes) => { bytes}
        Err(e) => { return; }
    };

    if bytes.is_empty() {
        return;
    }

    let image = match image::load_from_memory(&bytes) {
        Ok(image) => { image }
        Err(_) => { return;}
    };

    let img = image.thumbnail(64, 64);
    let histogram = color_histogram(&img);
    if histogram.is_empty() {
        return;
    }

    let clusters = kmeans_recode(&histogram, 6, 20);

    let color = clusters
        .iter()
        .find(|c| is_usable_color(&c.color))
        .or_else(|| clusters.first()) // fall back to dominant if all filtered
        .map(|c| c.color);

    if color.is_some() {
        let found = color.unwrap();
        let _ = tx.send(AppEvent::ThemeFetched {
            rgb: [found.r.clamp(0.0, 255.0) as u8, found.g.clamp(0.0, 255.0) as u8, found.b.clamp(0.0, 255.0) as u8]
        });
    }


}

async fn fetch_art_bytes(art_url: &str) -> Result<Vec<u8>, String> {
    if let Some(path) = art_url.strip_prefix("file://") {
        fs::read(path).map_err(|e| format!("Failed to read art file: {e}"))
    } else {
        reqwest::get(art_url)
            .await
            .map_err(|e| format!("Failed to fetch art: {e}"))?
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read art bytes: {e}"))
    }
}

fn is_usable_color(c: &Color) -> bool {
    let brightness = (c.r + c.g + c.b) / 3.0;
    if brightness < 30.0 || brightness > 230.0 {
        return false;
    }

    // Check saturation — skip near-grey colors.
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    let saturation = if max > 0.0 { (max - min) / max } else { 0.0 };

    saturation > 0.15
}
