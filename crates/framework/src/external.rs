//! Read-only, bounded transport for the SJTU Canvas instance.
use planner_contracts::*;
use serde_json::Value;
use std::{
    io::Read,
    time::{Duration, Instant},
};
pub trait ExternalReader: Sync {
    fn list(&self, path: &str) -> Result<Vec<Value>>;
}
pub struct CanvasReader;
pub fn save_canvas_token(token: &str) -> Result<()> {
    if token.trim().is_empty() {
        return Err(AppError::new("credentials", "令牌不能为空"));
    }
    keyring::Entry::new("local.student-planner", "canvas:oc.sjtu.edu.cn")
        .and_then(|e| e.set_password(token.trim()))
        .map_err(|_| AppError::new("credentials", "无法保存 Canvas 令牌至钥匙串"))
}
fn checked_url(path: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse("https://oc.sjtu.edu.cn")
        .unwrap()
        .join(path)
        .map_err(|_| AppError::new("canvas_url", "接口地址无效"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("oc.sjtu.edu.cn")
        || url.port().is_some()
        || !url.path().starts_with("/api/v1/")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::new(
            "canvas_url",
            "拒绝向其他站点发送 Canvas 凭据",
        ));
    }
    Ok(url)
}
pub struct CanvasSession {
    key: String,
    client: reqwest::blocking::Client,
}
impl CanvasReader {
    pub fn connect() -> Result<CanvasSession> {
        let key = keyring::Entry::new("local.student-planner", "canvas:oc.sjtu.edu.cn")
            .and_then(|e| e.get_password())
            .map_err(|_| AppError::new("credentials", "请先保存 Canvas 令牌"))?;
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| AppError::new("network", "网络初始化失败"))?;
        Ok(CanvasSession { key, client })
    }
}
impl ExternalReader for CanvasReader {
    fn list(&self, path: &str) -> Result<Vec<Value>> {
        Self::connect()?.list(path)
    }
}
impl ExternalReader for CanvasSession {
    fn list(&self, path: &str) -> Result<Vec<Value>> {
        let mut url = checked_url(path)?;
        let mut rows = vec![];
        let mut seen = std::collections::BTreeSet::new();
        let started = Instant::now();
        for _ in 0..50 {
            if started.elapsed() > Duration::from_secs(90) || !seen.insert(url.to_string()) {
                break;
            }
            let response = self
                .client
                .get(url.clone())
                .bearer_auth(&self.key)
                .send()
                .map_err(|_| AppError::new("network", "Canvas 连接失败或超时，请重试"))?;
            if !response.status().is_success() {
                return Err(AppError::new(
                    "canvas_http",
                    format!(
                        "Canvas 返回 {}，请检查令牌与课程访问权限",
                        response.status().as_u16()
                    ),
                ));
            }
            let next = response
                .headers()
                .get("link")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.split(',').find(|p| p.contains("rel=\"next\"")))
                .and_then(|p| p.split('<').nth(1))
                .and_then(|p| p.split('>').next())
                .map(str::to_owned);
            let mut bytes = vec![];
            response
                .take(4_000_001)
                .read_to_end(&mut bytes)
                .map_err(|_| AppError::new("network", "Canvas 响应读取失败"))?;
            if bytes.len() > 4_000_000 {
                return Err(AppError::new("canvas_limit", "Canvas 响应过大"));
            }
            let page: Vec<Value> = serde_json::from_slice(&bytes)
                .map_err(|_| AppError::new("canvas_json", "Canvas 响应格式无效"))?;
            rows.extend(page);
            if rows.len() > 5000 {
                break;
            }
            match next {
                None => return Ok(rows),
                Some(next) => url = checked_url(&next)?,
            }
        }
        Err(AppError::new(
            "canvas_limit",
            "同步超过范围或时间上限，未应用部分数据",
        ))
    }
}
pub fn open_canvas_page(page: &str) -> Result<()> {
    let url = reqwest::Url::parse(page).map_err(|_| AppError::new("canvas_url", "页面地址无效"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("oc.sjtu.edu.cn")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || !url.path().starts_with("/courses/")
    {
        return Err(AppError::new("canvas_url", "仅允许打开交大课程页面"));
    }
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(url.as_str())
        .spawn()
        .map_err(|_| AppError::new("platform", "无法打开浏览器"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prevents_credential_redirect() {
        assert!(checked_url("/api/v1/courses").is_ok());
        for p in [
            "https://example.com/api/v1/courses",
            "//example.com/api/v1/courses",
            "/login",
            "http://oc.sjtu.edu.cn/api/v1/courses",
        ] {
            assert!(checked_url(p).is_err());
        }
    }
}
