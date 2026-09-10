mod image_ops;

use image_ops::{
    add_watermark_to_files, compress_files, CompressOptions, ProgressPayload, WatermarkOptions,
};
use tauri::{AppHandle, Emitter};

fn emit_progress(app: &AppHandle, done: usize, total: usize, current: &str) {
    let _ = app.emit(
        "progress",
        ProgressPayload {
            done,
            total,
            current: current.to_string(),
        },
    );
}

#[tauri::command]
async fn add_watermark(app: AppHandle, opts: WatermarkOptions) -> Result<Vec<String>, String> {
    add_watermark_to_files(&opts, |done, total, current| {
        emit_progress(&app, done, total, current);
    })
}

#[tauri::command]
async fn compress_images(app: AppHandle, opts: CompressOptions) -> Result<Vec<String>, String> {
    compress_files(&opts, |done, total, current| {
        emit_progress(&app, done, total, current);
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![add_watermark, compress_images])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
