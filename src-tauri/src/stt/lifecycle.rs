use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum ModelStatus {
    NotInstalled,
    Stopped,
    Starting,
    Running,
    Error(String),
}

pub struct LocalModelManager {
    app_handle: tauri::AppHandle,
    processes: Arc<Mutex<HashMap<String, Child>>>,
    statuses: Arc<Mutex<HashMap<String, ModelStatus>>>,
}

impl LocalModelManager {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            processes: Arc::new(Mutex::new(HashMap::new())),
            statuses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_status(&self, model_id: &str) -> ModelStatus {
        let statuses = self.statuses.lock().await;
        if let Some(status) = statuses.get(model_id) {
            return status.clone();
        }
        if crate::stt::models::is_installed(model_id) {
            ModelStatus::Stopped
        } else {
            ModelStatus::NotInstalled
        }
    }

    pub async fn set_status(&self, model_id: &str, status: ModelStatus) {
        let mut statuses = self.statuses.lock().await;
        statuses.insert(model_id.to_string(), status);
    }

    /// Stop all local models (e.g. when switching to Cloud mode). Only one should run at a time.
    pub async fn stop_all_models(&self) {
        for m in crate::stt::models::known_models() {
            let _ = self.stop_model(m.id).await;
        }
    }

    pub async fn start_model(&self, model_id: &str) -> anyhow::Result<()> {
        if !crate::stt::models::is_installed(model_id) {
            return Err(anyhow::anyhow!("Model not installed"));
        }

        // Only one local model runs at a time: stop any other running model first.
        for m in crate::stt::models::known_models() {
            if m.id != model_id {
                let _ = self.stop_model(m.id).await;
            }
        }

        let current = self.get_status(model_id).await;
        if current == ModelStatus::Running || current == ModelStatus::Starting {
            return Ok(());
        }

        self.set_status(model_id, ModelStatus::Starting).await;

        let manifest = crate::stt::models::known_models()
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown model"))?;

        let model_path = crate::stt::models::model_path(model_id, &manifest)?;

        let sidecar_id = match crate::stt::sidecars::model_id_to_sidecar_id(model_id) {
            Some(id) => id,
            None => {
                self.set_status(
                    model_id,
                    ModelStatus::Error("Unsupported local model".into()),
                )
                .await;
                return Err(anyhow::anyhow!("Unsupported local model"));
            }
        };

        if crate::stt::sidecars::sidecar_download_info(sidecar_id).is_none() {
            self.set_status(
                model_id,
                ModelStatus::Error("Engine not available for this platform".into()),
            )
            .await;
            return Err(anyhow::anyhow!(
                "Sidecar {} not available on this platform",
                sidecar_id
            ));
        }

        if !crate::stt::sidecars::sidecar_is_installed(sidecar_id) {
            if let Err(e) =
                crate::stt::sidecars::download_sidecar_with_progress(sidecar_id, &self.app_handle)
                    .await
            {
                let msg = format!("Engine download failed: {}", e);
                self.set_status(model_id, ModelStatus::Error(msg.clone()))
                    .await;
                return Err(anyhow::anyhow!("{}", msg));
            }
        }

        let binary_path = match crate::stt::sidecars::sidecar_path_for_model(model_id) {
            Ok(p) => p,
            Err(e) => {
                self.set_status(model_id, ModelStatus::Error(e.to_string()))
                    .await;
                return Err(e);
            }
        };

        let work_dir = binary_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();

        let server_args: Vec<String> = match model_id {
            "sensevoice" => {
                let root = crate::stt::models::sherpa_zipformer_model_root(&model_path)
                    .map_err(|e| anyhow::anyhow!("Invalid model layout: {}", e))?;

                let mut args = crate::stt::models::sherpa_zipformer_server_args(&root, 10080)?;

                // For SenseVoice, we can enable inverse text normalization (ITN) for punctuation
                if args.iter().any(|a| a.starts_with("--sense-voice-model")) {
                    args.push("--sense-voice-use-itn=1".to_string());
                }

                args
            }
            "whisper_base" => {
                vec![
                    "--model".to_string(),
                    model_path.to_string_lossy().to_string(),
                    "--port".to_string(),
                    "10080".to_string(),
                ]
            }
            _ => {
                self.set_status(
                    model_id,
                    ModelStatus::Error("Unsupported local model".into()),
                )
                .await;
                return Err(anyhow::anyhow!("Unsupported local model"));
            }
        };

        let child = tokio::process::Command::new(&binary_path)
            .args(&server_args)
            .current_dir(&work_dir)
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn engine: {}", e))?;

        let mut processes = self.processes.lock().await;
        processes.insert(model_id.to_string(), child);
        self.set_status(model_id, ModelStatus::Running).await;

        let statuses_clone = self.statuses.clone();
        let processes_clone = self.processes.clone();
        let model_id_clone = model_id.to_string();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let mut status_to_set = None;
                {
                    let mut guard = processes_clone.lock().await;
                    if let Some(c) = guard.get_mut(&model_id_clone) {
                        match c.try_wait() {
                            Ok(Some(exit)) => {
                                log::info!("Sidecar terminated: {:?}", exit);
                                guard.remove(&model_id_clone);
                                status_to_set = Some(ModelStatus::Stopped);
                            }
                            Ok(None) => {}
                            Err(e) => {
                                log::error!("Sidecar wait error: {}", e);
                                guard.remove(&model_id_clone);
                                status_to_set = Some(ModelStatus::Error(e.to_string()));
                            }
                        }
                    } else {
                        status_to_set = Some(ModelStatus::Stopped);
                    }
                }
                if let Some(s) = status_to_set {
                    let mut statuses = statuses_clone.lock().await;
                    statuses.insert(model_id_clone.clone(), s);
                    break;
                }
            }
        });

        Ok(())
    }

    pub async fn stop_model(&self, model_id: &str) -> anyhow::Result<()> {
        let mut processes = self.processes.lock().await;
        if let Some(mut child) = processes.remove(model_id) {
            child
                .kill()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to kill sidecar: {}", e))?;
            self.set_status(model_id, ModelStatus::Stopped).await;
        }
        Ok(())
    }

    pub async fn restart_model(&self, model_id: &str) -> anyhow::Result<()> {
        self.stop_model(model_id).await?;
        self.start_model(model_id).await
    }

    pub async fn delete_model(&self, model_id: &str) -> anyhow::Result<()> {
        self.stop_model(model_id).await?;

        let manifest = crate::stt::models::known_models()
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown model"))?;

        let model_path = crate::stt::models::model_path(model_id, &manifest)?;
        if model_path.exists() {
            if model_path.is_dir() {
                tokio::fs::remove_dir_all(model_path).await?;
            } else {
                tokio::fs::remove_file(model_path).await?;
            }
        }

        self.set_status(model_id, ModelStatus::NotInstalled).await;
        Ok(())
    }
}
