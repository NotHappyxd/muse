use std::fs;
use std::sync::OnceLock;
use expanduser::expanduser;
use regex::Regex;

const CACHE_DIRECTORY: &'static str = "~/.cache/lyse";
static NORMALIZE_RE: OnceLock<Regex> = OnceLock::new();

pub fn cache_key(title: &str, artists: &Vec<String>, album: &str) -> String {
    let re = NORMALIZE_RE.get_or_init(|| {
        Regex::new(r"[^\w]+").unwrap()
    });

    let key = format!("{}-{}", title, artists.join("-"));
    re.replace_all(&key.to_lowercase(), "").to_string()
}

pub fn find_cache(title: &str, artists: &Vec<String>) -> Option<String> {
    let parent = expanduser(CACHE_DIRECTORY);

    if parent.is_err() {
        return None;
    }

    let mut file = parent.unwrap();
    file.push(cache_key(title, artists, ""));

    if let Ok(exists) = fs::exists(&file) {
        if exists {
            if let Ok(content) = fs::read_to_string(&file) {
                return Some(content);
            }
        }
    }

    None
}

pub fn write_to_cache(key: &str, contents: &str) -> bool {
    if let Ok(mut parent) = expanduser(CACHE_DIRECTORY) {
        if let Err(e) = fs::create_dir_all(&parent) {
            eprintln!("Failed to create dirs: {e}");
            return false;
        }

        parent.push(key.to_string());

        return match fs::write(&parent, contents) {
            Ok(_) => true,
            Err(_) => false
        }
    }

    false
}