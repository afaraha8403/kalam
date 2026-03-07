use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub struct NotificationManager {
    app_handle: AppHandle,
}

impl NotificationManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn info(&self, message: &str) -> anyhow::Result<()> {
        self.show("Kalam", message)
    }

    pub fn success(&self, message: &str) -> anyhow::Result<()> {
        self.show("Kalam", message)
    }

    pub fn warning(&self, message: &str) -> anyhow::Result<()> {
        self.show("Kalam - Warning", message)
    }

    pub fn error(&self, message: &str) -> anyhow::Result<()> {
        self.show("Kalam - Error", message)
    }

    fn show(&self, title: &str, body: &str) -> anyhow::Result<()> {
        self.app_handle
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()?;
        Ok(())
    }
}
