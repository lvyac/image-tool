use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use ab_glyph::{FontVec, PxScale};
use image::{
    codecs::bmp::BmpEncoder, codecs::jpeg::JpegEncoder, codecs::png::PngEncoder,
    codecs::webp::WebPEncoder, ImageEncoder, Rgba, RgbaImage,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub done: usize,
    pub total: usize,
    pub current: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkOptions {
    pub files: Vec<String>,
    pub output_dir: String,
    pub text: String,
    pub font_size: f32,
    pub color: String,
    pub opacity: f32,
    pub position: String,
    pub margin: u32,
    pub tile: bool,
    pub spacing: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    pub files: Vec<String>,
    pub output_dir: String,
    pub quality: u8,
    pub format: String,
}

pub fn find_font() -> Result<FontVec, String> {
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/Arial Unicode.ttf",
        ]
    } else if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\msyh.ttf",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "C:\\Windows\\Fonts\\simsun.ttc",
            "C:\\Windows\\Fonts\\arial.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
    };

    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            if let Ok(font) = FontVec::try_from_vec_and_index(data, 0) {
                return Ok(font);
            }
        }
    }
    Err("未找到可用的系统字体，无法绘制文字水印".into())
}

fn parse_hex_color(hex: &str) -> Result<[u8; 3], String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(format!("颜色格式应为 #RRGGBB，当前为：{hex}"));
    }
    let v = u32::from_str_radix(hex, 16).map_err(|_| format!("颜色格式错误：{hex}"))?;
    Ok([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

fn rgba_color(hex: &str, opacity: f32) -> Result<Rgba<u8>, String> {
    let [r, g, b] = parse_hex_color(hex)?;
    let a = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    Ok(Rgba([r, g, b, a]))
}

fn output_path(input: &Path, output_dir: &str, suffix: &str, ext: &str) -> PathBuf {
    let dir = if output_dir.trim().is_empty() {
        input.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        PathBuf::from(&output_dir)
    };
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    dir.join(format!("{stem}_{suffix}.{ext}"))
}

fn save_rgba(img: &RgbaImage, path: &Path, quality: u8) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| "png".into());
    let file = File::create(path).map_err(|e| format!("无法创建文件 {}：{e}", path.display()))?;
    let mut writer = BufWriter::new(file);

    match ext.as_str() {
        "jpg" | "jpeg" => {
            let enc = JpegEncoder::new_with_quality(&mut writer, quality);
            enc.write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| format!("JPEG 编码失败：{e}"))?;
        }
        "webp" => {
            WebPEncoder::new_lossless(&mut writer)
                .encode(
                    img.as_raw(),
                    img.width(),
                    img.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| format!("WebP 编码失败：{e}"))?;
        }
        "bmp" => {
            let enc = BmpEncoder::new(&mut writer);
            enc.write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| format!("BMP 编码失败：{e}"))?;
        }
        _ => {
            let enc = PngEncoder::new(&mut writer);
            enc.write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| format!("PNG 编码失败：{e}"))?;
        }
    }
    Ok(())
}

fn rgba_to_rgb_white(img: &RgbaImage) -> image::RgbImage {
    let mut out = image::RgbImage::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let a = p[3] as f32 / 255.0;
        let mut rgb = [0u8; 3];
        for c in 0..3 {
            let v = p[c] as f32 * a + 255.0 * (1.0 - a);
            rgb[c] = v.round().clamp(0.0, 255.0) as u8;
        }
        out.put_pixel(x, y, image::Rgb(rgb));
    }
    out
}

fn save_rgb(img: &image::RgbImage, path: &Path, quality: u8) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("无法创建文件 {}：{e}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let enc = JpegEncoder::new_with_quality(&mut writer, quality);
    enc.write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgb8,
    )
    .map_err(|e| format!("JPEG 编码失败：{e}"))
}

fn draw_text(
    canvas: &mut RgbaImage,
    font: &FontVec,
    text: &str,
    scale: PxScale,
    color: Rgba<u8>,
    x: i32,
    y: i32,
) {
    imageproc::drawing::draw_text_mut(canvas, color, x, y, scale, font, text);
}

fn place_watermark(
    canvas: &mut RgbaImage,
    font: &FontVec,
    opts: &WatermarkOptions,
    color: Rgba<u8>,
) {
    let scale = PxScale::from(opts.font_size.max(1.0));
    let (tw, th) = imageproc::drawing::text_size(scale, font, &opts.text);
    if tw == 0 || th == 0 {
        return;
    }
    let tw = tw as i32;
    let th = th as i32;
    let margin = opts.margin as i32;
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;

    let base_xy = |tw: i32, th: i32| -> (i32, i32) {
        let x = match opts.position.as_str() {
            "nw" | "w" | "sw" => margin,
            "ne" | "e" | "se" => w - tw - margin,
            _ => (w - tw) / 2,
        };
        let y = match opts.position.as_str() {
            "nw" | "n" | "ne" => margin,
            "sw" | "s" | "se" => h - th - margin,
            _ => (h - th) / 2,
        };
        (x, y)
    };

    if opts.tile {
        let spacing = opts.spacing as i32;
        let step_x = (tw + spacing).max(tw);
        let step_y = (th + spacing).max(th);
        let mut y = margin;
        while y + th <= h + margin {
            let mut x = margin;
            while x + tw <= w + margin {
                draw_text(canvas, font, &opts.text, scale, color, x, y);
                x += step_x;
            }
            y += step_y;
        }
    } else {
        let (x, y) = base_xy(tw, th);
        draw_text(canvas, font, &opts.text, scale, color, x, y);
    }
}

pub fn add_watermark_to_files(
    opts: &WatermarkOptions,
    mut on_progress: impl FnMut(usize, usize, &str),
) -> Result<Vec<String>, String> {
    if opts.text.trim().is_empty() {
        return Err("请先输入水印文字".into());
    }
    let font = find_font()?;
    let color = rgba_color(&opts.color, opts.opacity)?;
    let total = opts.files.len();
    let mut results = Vec::with_capacity(total);

    for (i, path_str) in opts.files.iter().enumerate() {
        let input = Path::new(path_str);
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path_str)
            .to_string();
        let ext = input
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "png".into());
        let out = output_path(input, &opts.output_dir, "watermark", &ext);

        let result = (|| -> Result<(), String> {
            let img = image::open(input).map_err(|e| format!("无法读取图片 {name}：{e}"))?;
            let mut canvas = img.to_rgba8();
            place_watermark(&mut canvas, &font, opts, color);
            save_rgba(&canvas, &out, 95)
        })();

        match result {
            Ok(()) => results.push(out.display().to_string()),
            Err(e) => return Err(e),
        }
        on_progress(i + 1, total, &name);
    }
    Ok(results)
}

pub fn compress_files(
    opts: &CompressOptions,
    mut on_progress: impl FnMut(usize, usize, &str),
) -> Result<Vec<String>, String> {
    let quality = opts.quality.clamp(1, 100);
    let total = opts.files.len();
    let mut results = Vec::with_capacity(total);

    for (i, path_str) in opts.files.iter().enumerate() {
        let input = Path::new(path_str);
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path_str)
            .to_string();
        let result = (|| -> Result<(), String> {
            let img = image::open(input).map_err(|e| format!("无法读取图片 {name}：{e}"))?;
            let format = opts.format.to_lowercase();
            if format == "jpeg" {
                let out = output_path(input, &opts.output_dir, "compressed", "jpg");
                let rgb = rgba_to_rgb_white(&img.to_rgba8());
                save_rgb(&rgb, &out, quality)?;
                results.push(out.display().to_string());
            } else {
                let src_ext = input
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_else(|| "png".into());
                let (ext, quality) = match src_ext.as_str() {
                    "jpg" | "jpeg" => ("jpg", quality),
                    "webp" => ("webp", quality),
                    "bmp" => ("bmp", 95),
                    _ => ("png", 0),
                };
                let out = output_path(input, &opts.output_dir, "compressed", ext);
                let canvas = img.to_rgba8();
                save_rgba(&canvas, &out, quality)?;
                results.push(out.display().to_string());
            }
            Ok(())
        })();

        match result {
            Ok(()) => {}
            Err(e) => return Err(e),
        }
        on_progress(i + 1, total, &name);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_dir() -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("image-tool-test-{ts}-{id}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_test_png(dir: &Path) -> PathBuf {
        let mut img = RgbaImage::new(200, 150);
        for p in img.pixels_mut() {
            *p = Rgba([255, 0, 0, 255]);
        }
        let path = dir.join("source.png");
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn watermark_creates_output() {
        let dir = tmp_dir();
        let src = make_test_png(&dir);
        let mut progress = vec![];
        let opts = WatermarkOptions {
            files: vec![src.display().to_string()],
            output_dir: dir.display().to_string(),
            text: "测试水印".into(),
            font_size: 30.0,
            color: "#ffffff".into(),
            opacity: 0.8,
            position: "se".into(),
            margin: 10,
            tile: false,
            spacing: 0,
        };
        let out = add_watermark_to_files(&opts, |done, total, cur| {
            progress.push((done, total, cur.to_string()));
        })
        .unwrap();
        assert_eq!(out.len(), 1);
        assert!(Path::new(&out[0]).exists());
        assert_eq!(progress.len(), 1);
        let saved = image::open(&out[0]).unwrap().to_rgba8();
        assert_eq!(saved.dimensions(), (200, 150));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn watermark_tile_does_not_panic() {
        let dir = tmp_dir();
        let src = make_test_png(&dir);
        let opts = WatermarkOptions {
            files: vec![src.display().to_string()],
            output_dir: dir.display().to_string(),
            text: "平铺".into(),
            font_size: 20.0,
            color: "#000000".into(),
            opacity: 0.5,
            position: "c".into(),
            margin: 0,
            tile: true,
            spacing: 30,
        };
        add_watermark_to_files(&opts, |_, _, _| {}).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn compress_to_jpeg_works() {
        let dir = tmp_dir();
        let src = make_test_png(&dir);
        let opts = CompressOptions {
            files: vec![src.display().to_string()],
            output_dir: dir.display().to_string(),
            quality: 80,
            format: "jpeg".into(),
        };
        let out = compress_files(&opts, |_, _, _| {}).unwrap();
        assert_eq!(out.len(), 1);
        let path = Path::new(&out[0]);
        assert!(path.exists());
        assert_eq!(path.extension().unwrap(), "jpg");
        let saved = image::open(path).unwrap();
        assert_eq!(saved.width(), 200);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
