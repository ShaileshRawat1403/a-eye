use crate::aeye::config::AEyeConfig;
use anyhow::Result;
use codex_core::{
    AuthManager, ModelClient, ModelClientSession, ModelInfo, ModelsManager, SessionSource,
    ThreadId, TransportManager,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

static MODEL_INFO_CACHE: Lazy<Mutex<HashMap<String, (Arc<ModelInfo>, Instant)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
/// Creates a new ModelClientSession for making LLM calls.
pub async fn new_model_client_session(
    aeye_config: &AEyeConfig,
    op_name: &str,
) -> Result<ModelClientSession> {
    let config = Arc::new(aeye_config.core.clone());
    let auth_manager = AuthManager::shared(
        aeye_config.codex_home.clone(),
        false,
        codex_core::auth::AuthCredentialsStoreMode::File,
    );

    let model_slug = config.model.as_deref().unwrap_or("gpt-5.1-codex-max");
    let model_info = {
        let cache = MODEL_INFO_CACHE.lock().await;
        let is_stale = cache
            .get(model_slug)
            .map_or(true, |(_, cached_at)| cached_at.elapsed() >= CACHE_TTL);

        if is_stale {
            // Drop the lock before the async call to avoid holding it across an await point.
            drop(cache);

            let models_manager =
                ModelsManager::new(config.codex_home.clone(), auth_manager.clone());
            let info = models_manager.get_model_info(model_slug, &config).await;
            let arc_info = Arc::new(info);

            let mut cache = MODEL_INFO_CACHE.lock().await;
            // Insert the new info, overwriting any stale entry.
            cache.insert(
                model_slug.to_string(),
                (arc_info.clone(), Instant::now()),
            );
            arc_info
        } else {
            // It must exist and be fresh. The unwrap is safe.
            cache.get(model_slug).unwrap().0.clone()
        }
    };

    let conversation_id = ThreadId::new();
    let otel_manager = codex_otel::OtelManager::new(
        conversation_id,
        model_slug,
        model_info.slug.as_str(),
        None,
        None,
        auth_manager.get_auth_mode(),
        false,
        op_name.to_string(),
        SessionSource::Exec,
    );

    let session = ModelClient::new(
        config.clone(),
        None,
        model_info,
        otel_manager,
        config.model_provider.clone(),
        config.model_reasoning_effort,
        config.model_reasoning_summary,
        conversation_id,
        SessionSource::Exec,
        TransportManager::new(),
    )
    .new_session();

    Ok(session)
}
