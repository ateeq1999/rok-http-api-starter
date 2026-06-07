use std::path::Path;

use tokio::fs;

fn ext_from_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

pub async fn save_avatar(
    storage_dir: &str,
    user_id: &str,
    mime: &str,
    data: &[u8],
) -> Result<String, String> {
    let ext = ext_from_mime(mime).ok_or_else(|| format!("unsupported mime type: {mime}"))?;
    let dir = Path::new(storage_dir).join("avatars");
    fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("failed to create directory: {e}"))?;

    let filename = format!("{}_{}.{ext}", user_id, chrono::Utc::now().timestamp());
    let path = dir.join(&filename);
    fs::write(&path, data)
        .await
        .map_err(|e| format!("failed to write file: {e}"))?;

    Ok(format!("/storage/avatars/{filename}"))
}

pub async fn delete_avatar(storage_dir: &str, url: &str) {
    let relative = url.strip_prefix('/').unwrap_or(url);
    let path = Path::new(storage_dir).join(relative);
    let _ = fs::remove_file(&path).await;
}
