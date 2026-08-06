use crate::colorgen::generator;
use crate::watcher::AppEvent;
use tokio::sync::mpsc::UnboundedSender;

pub async fn fetch_theme(
    song_title: String,
    art_url: String,
    tx: &UnboundedSender<AppEvent>,
    k_clusters: u8,
    max_iterations: u8,
    min_chroma: f32,
) {
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
            } else {
                img
            }
        }
        Err(_) => return,
    };

    let theme = generator::generate_from_image(
        &img,
        k_clusters as usize,
        max_iterations as usize,
        min_chroma,
        false,
    );

    let _ = tx.send(AppEvent::ThemeFetched { song_title, theme });
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
