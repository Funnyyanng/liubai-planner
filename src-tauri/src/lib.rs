use planner_contracts::*;
use planner_coordinator::Service;
use planner_framework::{platform, time};
use std::{path::PathBuf, sync::Arc};
use tauri::Manager;
struct Desktop {
    service: Option<Arc<Service>>,
    error: Option<String>,
    path: PathBuf,
}
fn service(state: &Desktop) -> Result<Arc<Service>> {
    state
        .service
        .clone()
        .ok_or_else(|| AppError::new("diagnostic", state.error.as_deref().unwrap_or("存储不可用")))
}
#[tauri::command]
async fn get_state(state: tauri::State<'_, Desktop>) -> Result<AppState> {
    let s = service(&state)?;
    tauri::async_runtime::spawn_blocking(move || s.state())
        .await
        .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn dispatch(
    state: tauri::State<'_, Desktop>,
    name: String,
    payload: serde_json::Value,
    version: u64,
    command_id: String,
) -> Result<AppState> {
    let s = service(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        s.command(&name, payload, version, &command_id, time::now())
    })
    .await
    .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn wake(state: tauri::State<'_, Desktop>) -> Result<AppState> {
    let s = service(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        s.wake(time::now())?;
        s.state()
    })
    .await
    .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn generate_draft(
    state: tauri::State<'_, Desktop>,
    prompt: String,
    task_ids: Vec<String>,
    event_ids: Vec<String>,
) -> Result<AiDraft> {
    let s = service(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        s.draft(
            &platform::HttpModel,
            &prompt,
            &task_ids,
            &event_ids,
            time::now(),
        )
    })
    .await
    .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn save_api_key(key: String) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || platform::save_api_key(&key))
        .await
        .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn import_file(state: tauri::State<'_, Desktop>, version: u64) -> Result<Option<AppState>> {
    let s = service(&state)?;
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter("ICS 日历", &["ics"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let bytes = file.read().await;
    let text = String::from_utf8(bytes)
        .map_err(|_| AppError::new("encoding", "日历文件必须使用 UTF-8 编码"))?;
    tauri::async_runtime::spawn_blocking(move || {
        s.command(
            "importIcs",
            serde_json::json!(text),
            version,
            &format!(
                "import:{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
            time::now(),
        )
        .map(Some)
    })
    .await
    .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn backup(state: tauri::State<'_, Desktop>) -> Result<Option<String>> {
    let s = service(&state)?;
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_file_name("留白备份.sqlite")
        .save_file()
        .await
    else {
        return Ok(None);
    };
    let path = file.path().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        s.store.backup(&path)?;
        Ok(Some(path.to_string_lossy().to_string()))
    })
    .await
    .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn restore(state: tauri::State<'_, Desktop>) -> Result<Option<AppState>> {
    let s = service(&state)?;
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter("SQLite 备份", &["sqlite", "db"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let path = file.path().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        s.restore(&path)?;
        s.state().map(Some)
    })
    .await
    .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn export_calendar(state: tauri::State<'_, Desktop>) -> Result<Option<String>> {
    let s = service(&state)?;
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_file_name("留白计划.ics")
        .save_file()
        .await
    else {
        return Ok(None);
    };
    let text = s.export_ics()?;
    file.write(text.as_bytes())
        .await
        .map_err(|e| AppError::new("file", e.to_string()))?;
    Ok(Some("计划已导出".into()))
}
#[tauri::command]
fn diagnostics(state: tauri::State<'_, Desktop>) -> serde_json::Value {
    serde_json::json!({"path":state.path,"error":state.error})
}
#[tauri::command]
async fn recover_database(app: tauri::AppHandle, state: tauri::State<'_, Desktop>) -> Result<()> {
    if state.service.is_some() {
        return Err(AppError::new("recovery", "请在设置页恢复备份"));
    }
    let Some(file) = rfd::AsyncFileDialog::new()
        .add_filter("SQLite 备份", &["sqlite", "db"])
        .pick_file()
        .await
    else {
        return Ok(());
    };
    let path = file.path().to_owned();
    let destination = state.path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let ms: Vec<ModuleManifest> = serde_json::from_str(include_str!("../../modules.json"))
            .map_err(|e| AppError::new("manifest", e.to_string()))?;
        planner_framework::storage::recover_file(&destination, &path, &ms)
    })
    .await
    .map_err(|e| AppError::new("worker", e.to_string()))??;
    app.restart();
}
#[tauri::command]
async fn calendar_occurrences(
    state: tauri::State<'_, Desktop>,
    from: i64,
    until: i64,
) -> Result<Vec<CalendarEvent>> {
    if until - from > 60 * 1440 {
        return Err(AppError::new("range", "最多读取 60 天日程"));
    }
    service(&state)?.occurrences(from, until)
}
#[tauri::command]
async fn desktop_notice(title: String, body: String) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || platform::notify(&title, &body))
        .await
        .map_err(|e| AppError::new("worker", e.to_string()))?
}
#[tauri::command]
async fn canvas_save_token(token: String) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        planner_framework::external::save_canvas_token(&token)
    })
    .await
    .map_err(|_| AppError::new("worker", "凭据保存失败"))?
}
#[tauri::command]
async fn canvas_courses(state: tauri::State<'_, Desktop>) -> Result<Vec<planner_canvas::Course>> {
    let s = service(&state)?;
    tauri::async_runtime::spawn_blocking(move || s.canvas_courses())
        .await
        .map_err(|_| AppError::new("worker", "课程读取失败"))?
}
#[tauri::command]
async fn canvas_preview(
    state: tauri::State<'_, Desktop>,
    course_id: u64,
) -> Result<planner_canvas::Preview> {
    let s = service(&state)?;
    tauri::async_runtime::spawn_blocking(move || s.canvas_preview(course_id))
        .await
        .map_err(|_| AppError::new("worker", "预览读取失败"))?
}
#[tauri::command]
async fn canvas_import(
    state: tauri::State<'_, Desktop>,
    request: planner_canvas::Import,
    preview: planner_canvas::Preview,
    version: u64,
    command_id: String,
) -> Result<AppState> {
    let s = service(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        s.canvas_import(request, preview, version, &command_id)
    })
    .await
    .map_err(|_| AppError::new("worker", "同步失败"))?
}
#[tauri::command]
fn canvas_open_page(url: String) -> Result<()> {
    planner_framework::external::open_canvas_page(&url)
}
#[tauri::command]
async fn canvas_scan(state: tauri::State<'_, Desktop>) -> Result<Vec<planner_canvas::Preview>> {
    let s = service(&state)?;
    tauri::async_runtime::spawn_blocking(move || s.canvas_scan())
        .await
        .map_err(|_| AppError::new("worker", "扫描失败"))?
}
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let path = std::env::var_os("LIUBAI_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?)
                .join("planner.sqlite");
            let ms: Vec<ModuleManifest> = serde_json::from_str(include_str!("../../modules.json"))?;
            let state = match Service::open(&path, ms) {
                Ok(s) => Desktop {
                    service: Some(Arc::new(s)),
                    error: None,
                    path,
                },
                Err(e) => Desktop {
                    service: None,
                    error: Some(e.to_string()),
                    path,
                },
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            canvas_scan,
            canvas_open_page,
            canvas_save_token,
            canvas_courses,
            canvas_preview,
            canvas_import,
            get_state,
            dispatch,
            wake,
            generate_draft,
            save_api_key,
            import_file,
            backup,
            restore,
            export_calendar,
            diagnostics,
            desktop_notice,
            calendar_occurrences,
            recover_database
        ])
        .run(tauri::generate_context!())
        .expect("无法启动桌面应用");
}
