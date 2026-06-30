use crate::watcher::AppEvent;
use tokio::sync::mpsc::UnboundedSender;
use crate::colorgen::generator;

pub async fn fetch_theme(art_url: String, tx: &UnboundedSender<AppEvent>) {
    let bytes = match fetch_art_bytes(&art_url).await {
        Ok(bytes) => bytes,
        Err(_) => return,
    };

    if bytes.is_empty() {
        return;
    }

    let img = match image::load_from_memory(&bytes) {
        Ok(img) => {
            if img.width() > 256 && img.height() > 256 {
                img.resize(256, 256, image::imageops::FilterType::Lanczos3)
            }else {
                img
            }
        },
        Err(_) => return,
    };

    let theme = generator::generate_from_image(&img, true);

    let _ = tx.send(AppEvent::ThemeFetched {
        rgb: theme.main,
        accent: theme.accent,
    });
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

/*

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
*/