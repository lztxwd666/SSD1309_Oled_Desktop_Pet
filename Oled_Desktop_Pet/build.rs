// build.rs —— 编译期精灵资源生成器 + 眼睛位置检测。
//
// image crate 负责全部图像处理：
//   PNG 解码 → Alpha 合成黑底 → BT.601 亮度 → 阈值二值化 → [u8;512]
//
// 额外扫描生成的单色 buffer，自动定位眼睛像素簇并输出坐标常量，
// 供运行时眨眼动画使用。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use image::GenericImageView;

const PET_SIZE: u32 = 56;
const BUF_PX: u32 = 64;
const BASE_PNG: &str = "ferris.png";
/// 亮度阈值。ferris 铁锈橙身体亮度 ~108，背景透明合成黑底后 = 0。
/// 80 完整捕获身体 + 钳子，同时排除黑色轮廓线。
const THRESHOLD: u8 = 80;

fn main() {
    println!("cargo:rerun-if-changed=assets/images/");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR 未设置");
    let dest = PathBuf::from(&out_dir).join("sprites.rs");
    let png_path = Path::new("assets/images").join(BASE_PNG);

    let code = if png_path.is_file() {
        let (data, sw, sh, eyes) = process_png(&png_path);
        let nz = data.iter().filter(|&&b| b != 0).count();
        eprintln!(
            "[build.rs] {}×{}px, {} non‑zero bytes (threshold={}), eyes L=({},{}), R=({},{})",
            sw, sh, nz, THRESHOLD, eyes.0, eyes.1, eyes.2, eyes.3
        );
        format!(
            "// 自动生成，勿手动编辑\n\
             // 来源：assets/images/{0}，{2}×{3}px\n\
             // 灰度 + 阈值二值化（image crate）\n\n\
             pub const BASE_SPRITE: [u8; 512] = {1};\n\
             #[allow(dead_code)] pub const SPRITE_W: u32 = {2};\n\
             #[allow(dead_code)] pub const SPRITE_H: u32 = {3};\n\
             // 眼睛坐标（build.rs 自动检测白色像素簇）\n\
             pub const EYE_L_X: usize = {4};\n\
             pub const EYE_L_Y: usize = {5};\n\
             pub const EYE_R_X: usize = {6};\n\
             pub const EYE_R_Y: usize = {7};\n\
             pub const EYE_BOX_W: usize = 6;\n\
             pub const EYE_BOX_H: usize = 3;\n",
            BASE_PNG, fmt_array(&data), sw, sh,
            eyes.0, eyes.1, eyes.2, eyes.3
        )
    } else {
        format!("compile_error!(\"未找到精灵图片 assets/images/{0}\");\n", BASE_PNG)
    };

    fs::write(&dest, code).expect("写入 sprites.rs 失败");
}

fn process_png(path: &Path) -> ([u8; 512], u32, u32, (usize, usize, usize, usize)) {
    let img = image::open(path)
        .unwrap_or_else(|e| panic!("无法读取 PNG {}: {}", path.display(), e));
    let (iw, ih) = img.dimensions();
    let rgba = img.into_rgba8();

    let sw = iw.min(PET_SIZE);
    let sh = ih.min(PET_SIZE);

    let ox = ((PET_SIZE as i32 - sw as i32) / 2).max(0);
    let oy = ((PET_SIZE as i32 - sh as i32) / 2).max(0);

    let mut buffer = [0u8; 512];
    for y in 0i32..BUF_PX as i32 {
        for x in 0i32..BUF_PX as i32 {
            let on = if x < PET_SIZE as i32 && y < PET_SIZE as i32 {
                let ix = x - ox;
                let iy = y - oy;
                if (0..sw as i32).contains(&ix) && (0..sh as i32).contains(&iy) {
                    let p = rgba.get_pixel(ix as u32, iy as u32);
                    let a = p[3] as u32;
                    let r = (p[0] as u32 * a / 255) as u8;
                    let g = (p[1] as u32 * a / 255) as u8;
                    let b = (p[2] as u32 * a / 255) as u8;
                    let lum = ((r as u32 * 77 + g as u32 * 150 + b as u32 * 29) / 256) as u8;
                    lum > THRESHOLD
                } else {
                    false
                }
            } else {
                false
            };
            if on {
                let idx = y as usize * 8 + x as usize / 8;
                let bit = 7 - (x as usize % 8);
                buffer[idx] |= 1 << bit;
            }
        }
    }

    let eyes = detect_eyes(&rgba, sw, sh, ox, oy);
    (buffer, sw, sh, eyes)
}

/// 分析原始 RGBA 像素，定位眼睛坐标。
///
/// 眼球是纯白 (~255,255,255)，蟹脸铁锈橙 (~183,83,39)，
/// 在 RGBA 中亮度差 >100，精确区分。返回宠区坐标。
fn detect_eyes(rgba: &image::RgbaImage, sw: u32, sh: u32, ox: i32, oy: i32) -> (usize, usize, usize, usize) {
    // 在原始图片的面部区域搜索最亮像素簇（眼球）
    let mut bright_spots: Vec<(usize, usize)> = Vec::new();

    // 面部大约在图片上方 20%-50% 区域
    let face_y0 = (sh as f32 * 0.20) as u32;
    let face_y1 = (sh as f32 * 0.50) as u32;
    let face_x0 = (sw as f32 * 0.25) as u32;
    let face_x1 = (sw as f32 * 0.75) as u32;

    for iy in face_y0..face_y1 {
        for ix in face_x0..face_x1 {
            let p = rgba.get_pixel(ix, iy);
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            let a = p[3] as u32;
            // 跳过透明像素
            if a < 128 { continue; }
            // 眼球：RGB 均 > 180（纯白或近白）
            if r > 180 && g > 180 && b > 180 {
                // 转换到宠区坐标
                let px = (ix as i32 + ox) as usize;
                let py = (iy as i32 + oy) as usize;
                if px < 56 && py < 56 { bright_spots.push((px, py)); }
            }
        }
    }

    eprintln!("[build.rs] RGBA 眼球检测：找到 {} 个亮像素", bright_spots.len());

    if bright_spots.len() < 4 { return (22, 18, 34, 18); }

    // 找到中位 x 坐标，将亮像素分为左右两组（左眼/右眼）
    let mut xs: Vec<usize> = bright_spots.iter().map(|(x, _)| *x).collect();
    xs.sort();
    let mid_x = xs[xs.len() / 2];

    let left_spots: Vec<_> = bright_spots.iter().filter(|(x, _)| *x < mid_x).collect();
    let right_spots: Vec<_> = bright_spots.iter().filter(|(x, _)| *x >= mid_x).collect();

    if left_spots.is_empty() || right_spots.is_empty() { return (22, 18, 34, 18); }

    let lx = left_spots.iter().map(|(x, _)| *x).min().unwrap_or(22);
    let ly = left_spots.iter().map(|(_, y)| *y).min().unwrap_or(18);
    let rx = right_spots.iter().map(|(x, _)| *x).min().unwrap_or(34);
    let ry = right_spots.iter().map(|(_, y)| *y).min().unwrap_or(18);

    eprintln!("[build.rs] 眼睛坐标: L=({},{}), R=({},{})", lx, ly, rx, ry);
    (lx, ly, rx, ry)
}

fn fmt_array(data: &[u8; 512]) -> String {
    let mut s = String::with_capacity(4096);
    s.push_str("[\n");
    for (i, &b) in data.iter().enumerate() {
        if i % 16 == 0 { s.push_str("    "); }
        s.push_str(&format!("0x{:02X},", b));
        if i % 16 == 15 || i == 511 { s.push('\n'); }
    }
    s.push(']');
    s
}
