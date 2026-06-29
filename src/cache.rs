use std::fs;
use expanduser::expanduser;

const CACHE_DIRECTORY: &'static str = "~/.cache/lyse";

pub fn cache_key(title: &str, artists: &Vec<String>, _album: &str) -> String {
    let key = format!("{}-{}", title, artists.join("-"));

    key.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
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

fn normalize(str: &str) -> String {
    str.chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}
