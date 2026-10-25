use planner_contracts::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
pub trait ModelTransport: Send + Sync {
    fn complete(&self, config: &AiConfig, system: &str, prompt: &str) -> Result<String>;
}
pub struct HttpModel;
pub fn save_api_key(key: &str) -> Result<()> {
    let entry = keyring::Entry::new("local.student-planner", "model-api")
        .map_err(|e| AppError::new("credentials", e.to_string()))?;
    entry
        .set_password(key)
        .map_err(|e| AppError::new("credentials", e.to_string()))
}
impl ModelTransport for HttpModel {
    fn complete(&self, c: &AiConfig, system: &str, prompt: &str) -> Result<String> {
        let url = reqwest::Url::parse(&c.endpoint)
            .map_err(|_| AppError::new("endpoint", "请输入完整 HTTPS 模型接口地址"))?;
        if url.scheme() != "https" {
            return Err(AppError::new("endpoint", "云端模型接口必须使用 HTTPS"));
        }
        let key = keyring::Entry::new("local.student-planner", "model-api")
            .and_then(|e| e.get_password())
            .map_err(|_| AppError::new("credentials", "请先保存模型 API 密钥"))?;
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|e| AppError::new("network", e.to_string()))?;
        let resp=client.post(url).bearer_auth(key).json(&serde_json::json!({"model":c.model,"messages":[{"role":"system","content":system},{"role":"user","content":prompt}],"temperature":0.2,"max_tokens":4000,"response_format":{"type":"json_object"}})).send().map_err(|_|AppError::new("model_timeout","模型连接失败或超时，当前计划未改变"))?;
        if !resp.status().is_success() {
            return Err(AppError::new(
                "model_http",
                format!("模型接口返回 {}", resp.status()),
            ));
        }
        let value: serde_json::Value = resp
            .json()
            .map_err(|_| AppError::new("model_response", "模型响应不是合法 JSON"))?;
        value["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| AppError::new("model_response", "模型响应缺少 content"))
    }
}
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst)
    }
    pub fn checkpoint(&self, started: Instant, timeout: Duration) -> Result<()> {
        if self.0.load(Ordering::SeqCst) {
            return Err(AppError::new("cancelled", "工作已取消"));
        }
        if started.elapsed() > timeout {
            return Err(AppError::new("timeout", "工作已超过时限"));
        }
        Ok(())
    }
}
pub fn notify(title: &str, body: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let esc = |s: &str| {
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', " ")
        };
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "display notification \"{}\" with title \"{}\"",
                esc(body),
                esc(title)
            ))
            .output()
            .map_err(|e| AppError::new("notification", e.to_string()))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_and_deadline_are_enforced() {
        let c = Cancellation::default();
        assert!(c.checkpoint(Instant::now(), Duration::from_secs(1)).is_ok());
        c.cancel();
        assert_eq!(
            c.checkpoint(Instant::now(), Duration::from_secs(1))
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert_eq!(
            Cancellation::default()
                .checkpoint(
                    Instant::now() - Duration::from_secs(2),
                    Duration::from_secs(1)
                )
                .unwrap_err()
                .code,
            "timeout"
        );
    }
}
