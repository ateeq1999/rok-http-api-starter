use std::sync::Arc;

use di::injectable;

use crate::config::AppConfig;
use crate::storage;

#[async_trait::async_trait]
pub trait AvatarStorage: Send + Sync {
    async fn save_avatar(&self, user_id: &str, mime: &str, data: &[u8]) -> Result<String, String>;
    async fn delete_avatar(&self, url: &str);
}

#[injectable]
pub struct LocalAvatarStorage {
    #[inject]
    config: Arc<AppConfig>,
}

#[async_trait::async_trait]
impl AvatarStorage for LocalAvatarStorage {
    async fn save_avatar(&self, user_id: &str, mime: &str, data: &[u8]) -> Result<String, String> {
        storage::save_avatar(&self.config.storage_dir, user_id, mime, data).await
    }

    async fn delete_avatar(&self, url: &str) {
        storage::delete_avatar(&self.config.storage_dir, url).await
    }
}
