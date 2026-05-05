use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

const USERNAME_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const SKIN_URL_TTL: Duration = Duration::from_secs(30 * 60);
const RENDERED_PNG_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug)]
pub struct AppCache {
    usernames: RwLock<HashMap<String, CacheEntry<String>>>,
    skin_urls: RwLock<HashMap<String, CacheEntry<String>>>,
    rendered_pngs: RwLock<HashMap<RenderedKey, CacheEntry<Vec<u8>>>>,
}

#[derive(Debug)]
struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RenderedKey {
    skin_url: String,
    size: u32,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            usernames: RwLock::new(HashMap::new()),
            skin_urls: RwLock::new(HashMap::new()),
            rendered_pngs: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_username_uuid(&self, username: &str) -> Option<String> {
        get_cache_value(&self.usernames, &username.to_ascii_lowercase()).await
    }

    pub async fn set_username_uuid(&self, username: &str, uuid: String) {
        set_cache_value(
            &self.usernames,
            username.to_ascii_lowercase(),
            uuid,
            USERNAME_TTL,
        )
        .await;
    }

    pub async fn get_skin_url(&self, uuid: &str) -> Option<String> {
        get_cache_value(&self.skin_urls, &uuid.to_owned()).await
    }

    pub async fn set_skin_url(&self, uuid: &str, skin_url: String) {
        set_cache_value(&self.skin_urls, uuid.to_owned(), skin_url, SKIN_URL_TTL).await;
    }

    pub async fn get_rendered_png(&self, skin_url: &str, size: u32) -> Option<Vec<u8>> {
        let key = RenderedKey {
            skin_url: skin_url.to_owned(),
            size,
        };
        get_cache_value(&self.rendered_pngs, &key).await
    }

    pub async fn set_rendered_png(&self, skin_url: &str, size: u32, png: Vec<u8>) {
        let key = RenderedKey {
            skin_url: skin_url.to_owned(),
            size,
        };
        set_cache_value(&self.rendered_pngs, key, png, RENDERED_PNG_TTL).await;
    }
}

impl Default for AppCache {
    fn default() -> Self {
        Self::new()
    }
}

async fn get_cache_value<K, V>(cache: &RwLock<HashMap<K, CacheEntry<V>>>, key: &K) -> Option<V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    let now = Instant::now();
    {
        let guard = cache.read().await;
        if let Some(entry) = guard.get(key) {
            if entry.expires_at > now {
                return Some(entry.value.clone());
            }
        } else {
            return None;
        }
    }

    let mut guard = cache.write().await;
    if guard
        .get(key)
        .is_some_and(|entry| entry.expires_at <= Instant::now())
    {
        guard.remove(key);
    }
    None
}

async fn set_cache_value<K, V>(
    cache: &RwLock<HashMap<K, CacheEntry<V>>>,
    key: K,
    value: V,
    ttl: Duration,
) where
    K: Eq + std::hash::Hash,
{
    let mut guard = cache.write().await;
    let now = Instant::now();
    guard.retain(|_, entry| entry.expires_at > now);
    guard.insert(
        key,
        CacheEntry {
            value,
            expires_at: now + ttl,
        },
    );
}
