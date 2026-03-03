use std::collections::VecDeque;

use image::{GenericImageView, ImageReader, RgbaImage};

use crate::error::SpriteGenError;

const TARGET_SIZE: u32 = 32;
const FLOOD_FILL_THRESHOLD: f64 = 30.0;

pub fn postprocess_image(raw_bytes: &[u8]) -> Result<RgbaImage, SpriteGenError> {
    let cursor = std::io::Cursor::new(raw_bytes);
    let img = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| SpriteGenError::ImageProcessing(format!("format detection failed: {}", e)))?
        .decode()
        .map_err(|e| SpriteGenError::ImageProcessing(format!("decode failed: {}", e)))?;

    // 正方形にクロップ（中央）
    let (w, h) = img.dimensions();
    let side = w.min(h);
    let x_off = (w - side) / 2;
    let y_off = (h - side) / 2;
    let cropped = img.crop_imm(x_off, y_off, side, side);

    // 32x32 に Nearest Neighbor リサイズ
    let resized = image::imageops::resize(
        &cropped.to_rgba8(),
        TARGET_SIZE,
        TARGET_SIZE,
        image::imageops::FilterType::Nearest,
    );

    // flood fill 透過処理
    let result = flood_fill_transparency(resized, FLOOD_FILL_THRESHOLD);

    Ok(result)
}

fn flood_fill_transparency(mut img: RgbaImage, threshold: f64) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return img;
    }

    let mut visited = vec![false; (w * h) as usize];
    let mut queue = VecDeque::new();

    // 外周ピクセルをシードとして収集
    for x in 0..w {
        enqueue_seed(&img, x, 0, w, &mut visited, &mut queue);
        enqueue_seed(&img, x, h - 1, w, &mut visited, &mut queue);
    }
    for y in 1..h.saturating_sub(1) {
        enqueue_seed(&img, 0, y, w, &mut visited, &mut queue);
        enqueue_seed(&img, w - 1, y, w, &mut visited, &mut queue);
    }

    // 外周ピクセルの平均色を背景色として推定
    let bg_color = estimate_background_color(&img, w, h);

    // BFS で背景を透明化
    while let Some((x, y)) = queue.pop_front() {
        let pixel = img.get_pixel(x, y);
        if color_distance(pixel, &bg_color) < threshold {
            img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));

            // 隣接ピクセルを走査
            for (nx, ny) in neighbors(x, y, w, h) {
                let idx = (ny * w + nx) as usize;
                if !visited[idx] {
                    visited[idx] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }

    img
}

fn enqueue_seed(
    _img: &RgbaImage,
    x: u32,
    y: u32,
    w: u32,
    visited: &mut [bool],
    queue: &mut VecDeque<(u32, u32)>,
) {
    let idx = (y * w + x) as usize;
    if !visited[idx] {
        visited[idx] = true;
        queue.push_back((x, y));
    }
}

fn estimate_background_color(img: &RgbaImage, w: u32, h: u32) -> image::Rgba<u8> {
    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut count: u64 = 0;

    // 外周ピクセルの色を集計
    for x in 0..w {
        accumulate_pixel(
            img.get_pixel(x, 0),
            &mut r_sum,
            &mut g_sum,
            &mut b_sum,
            &mut count,
        );
        if h > 1 {
            accumulate_pixel(
                img.get_pixel(x, h - 1),
                &mut r_sum,
                &mut g_sum,
                &mut b_sum,
                &mut count,
            );
        }
    }
    for y in 1..h.saturating_sub(1) {
        accumulate_pixel(
            img.get_pixel(0, y),
            &mut r_sum,
            &mut g_sum,
            &mut b_sum,
            &mut count,
        );
        if w > 1 {
            accumulate_pixel(
                img.get_pixel(w - 1, y),
                &mut r_sum,
                &mut g_sum,
                &mut b_sum,
                &mut count,
            );
        }
    }

    if count == 0 {
        return image::Rgba([255, 255, 255, 255]);
    }

    image::Rgba([
        (r_sum / count) as u8,
        (g_sum / count) as u8,
        (b_sum / count) as u8,
        255,
    ])
}

fn accumulate_pixel(
    pixel: &image::Rgba<u8>,
    r: &mut u64,
    g: &mut u64,
    b: &mut u64,
    count: &mut u64,
) {
    *r += pixel[0] as u64;
    *g += pixel[1] as u64;
    *b += pixel[2] as u64;
    *count += 1;
}

fn color_distance(a: &image::Rgba<u8>, b: &image::Rgba<u8>) -> f64 {
    let dr = a[0] as f64 - b[0] as f64;
    let dg = a[1] as f64 - b[1] as f64;
    let db = a[2] as f64 - b[2] as f64;
    (dr * dr + dg * dg + db * db).sqrt()
}

fn neighbors(x: u32, y: u32, w: u32, h: u32) -> impl Iterator<Item = (u32, u32)> {
    let mut result = Vec::with_capacity(4);
    if x > 0 {
        result.push((x - 1, y));
    }
    if x + 1 < w {
        result.push((x + 1, y));
    }
    if y > 0 {
        result.push((x, y - 1));
    }
    if y + 1 < h {
        result.push((x, y + 1));
    }
    result.into_iter()
}
