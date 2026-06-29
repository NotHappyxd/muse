use crate::kmeans::{Color, color_histogram, kmeans_recode};
use crate::watcher::AppEvent;
use tokio::sync::mpsc::UnboundedSender;

pub async fn fetch_theme(art_url: String, tx: &UnboundedSender<AppEvent>) {
    let bytes = match fetch_art_bytes(&art_url).await {
        Ok(bytes) => bytes,
        Err(_) => return,
    };

    if bytes.is_empty() {
        return;
    }

    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img.thumbnail(64, 64),
        Err(_) => return,
    };

    let histogram = color_histogram(&img);
    if histogram.is_empty() {
        return;
    }

    let clusters = kmeans_recode(&histogram, 6, 20);

    let main = clusters
        .iter()
        .find(|c| is_usable_color(&c.color))
        .or_else(|| clusters.first())
        .map(|c| c.color);

    let accent = clusters
        .iter()
        .filter(|c| is_usable_color(&c.color))
        .max_by(|a, b| {
            saturation(&a.color)
                .partial_cmp(&saturation(&b.color))
                .unwrap()
        })
        .or_else(|| clusters.first())
        .map(|c| c.color);

    if let (Some(main), Some(acc)) = (main, accent) {
        let _ = tx.send(AppEvent::ThemeFetched {
            rgb: [
                main.r.clamp(0.0, 255.0) as u8,
                main.g.clamp(0.0, 255.0) as u8,
                main.b.clamp(0.0, 255.0) as u8,
            ],
            accent: [
                acc.r.clamp(0.0, 255.0) as u8,
                acc.g.clamp(0.0, 255.0) as u8,
                acc.b.clamp(0.0, 255.0) as u8,
            ],
        });
    }
}

async fn fetch_art_bytes(art_url: &str) -> Result<Vec<u8>, String> {
    if let Some(path) = art_url.strip_prefix("file://") {
        tokio::fs::read(path)
            .await
            .map_err(|e| format!("Failed to read art file: {e}"))
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
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    let saturation = if max > 0.0 { (max - min) / max } else { 0.0 };
    saturation > 0.15
}

fn saturation(c: &Color) -> f32 {
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    if max > 0.0 { (max - min) / max } else { 0.0 }
}
