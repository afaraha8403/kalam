use std::collections::HashMap;
use std::sync::Arc;
use tauri_plugin_shell::{process::CommandChild, ShellExt};
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
    processes: Arc<Mutex<HashMap<String, CommandChild>>>,
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

    pub async fn start_model(&self, model_id: &str) -> anyhow::Result<()> {
        if !crate::stt::models::is_installed(model_id) {
            return Err(anyhow::anyhow!("Model not installed"));
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

        let sidecar_name = match model_id {
            "sensevoice" | "whisper_base" => crate::stt::sensevoice::SIDECAR_NAME,
            _ => return Err(anyhow::anyhow!("Unsupported local model")),
        };

        // For now, we'll start it in a generic way.
        // In a real implementation, it should start a WebSocket or HTTP server.
        // E.g. sherpa-onnx-offline-websocket-server
        let (mut rx, child) = self
            .app_handle
            .shell()
            .sidecar(sidecar_name)
            .map_err(|e| anyhow::anyhow!("Sidecar not found: {}", e))?
            .args([
                "--model",
                model_path.to_str().unwrap_or(""),
                "--port",
                "10080",
            ])
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn sidecar: {}", e))?;

        let mut processes = self.processes.lock().await;
        processes.insert(model_id.to_string(), child);
        self.set_status(model_id, ModelStatus::Running).await;

        let statuses_clone = self.statuses.clone();
        let processes_clone = self.processes.clone();
        let model_id_clone = model_id.to_string();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                        log::info!("Sidecar terminated: {:?}", payload);
                        let mut processes = processes_clone.lock().await;
                        processes.remove(&model_id_clone);
                        let mut statuses = statuses_clone.lock().await;
                        statuses.insert(model_id_clone.clone(), ModelStatus::Stopped);
                        break;
                    }
                    tauri_plugin_shell::process::CommandEvent::Error(err) => {
                        log::error!("Sidecar error: {}", err);
                        let mut processes = processes_clone.lock().await;
                        processes.remove(&model_id_clone);
                        let mut statuses = statuses_clone.lock().await;
                        statuses.insert(model_id_clone.clone(), ModelStatus::Error(err));
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub async fn stop_model(&self, model_id: &str) -> anyhow::Result<()> {
        let mut processes = self.processes.lock().await;
        if let Some(child) = processes.remove(model_id) {
            child
                .kill()
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
