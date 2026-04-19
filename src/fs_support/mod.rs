use std::fs;
use std::path::Path;

pub fn read_text_if_exists(path: &Path) -> std::io::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    fs::read_to_string(path).map(Some)
}
