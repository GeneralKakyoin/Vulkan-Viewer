use image::{Rgb, RgbImage};
use std::path::Path;

fn main() {
    let base_path = Path::new("test_assets");
    if !base_path.exists() {
        std::fs::create_dir_all(base_path).expect("Failed to create test_assets dir");
    }

    // 1. Water Diffuse (Blue ripples)
    let mut water = RgbImage::new(256, 256);
    for (x, y, pixel) in water.enumerate_pixels_mut() {
        let val = ((((x as f32 * 0.1).sin() + (y as f32 * 0.1).cos()) * 0.5 + 0.5) * 40.0) as u8;
        *pixel = Rgb([
            50u8.saturating_add(val),
            100u8.saturating_add(val),
            200u8.saturating_add(val),
        ]);
    }
    water.save(base_path.join("water_diffuse.png")).unwrap();

    // 2. Stone Diffuse (Gray blocks)
    let mut stone = RgbImage::new(256, 256);
    for (x, y, pixel) in stone.enumerate_pixels_mut() {
        let is_edge = x % 64 < 2 || y % 64 < 2;
        let val = if is_edge {
            50
        } else {
            100 + (x % 30 + y % 30) as u8
        };
        *pixel = Rgb([val, val, val]);
    }
    stone.save(base_path.join("stone_diffuse.png")).unwrap();

    // 3. Stone Normal (Purple-ish)
    let mut normal = RgbImage::new(256, 256);
    for (x, y, pixel) in normal.enumerate_pixels_mut() {
        let is_edge = x % 64 < 2 || y % 64 < 2;
        if is_edge {
            *pixel = Rgb([128, 128, 255]); // Flat
        } else {
            // Bevel effect
            let dx = if x % 64 < 8 {
                160
            } else if x % 64 > 56 {
                90
            } else {
                128
            };
            let dy = if y % 64 < 8 {
                160
            } else if y % 64 > 56 {
                90
            } else {
                128
            };
            *pixel = Rgb([dx, dy, 255]);
        }
    }
    normal.save(base_path.join("stone_normal.png")).unwrap();

    // 4. Fire Flipbook (4x4 grid)
    let mut fire = RgbImage::new(256, 256);
    for row in 0..4 {
        for col in 0..4 {
            let offset_x = col * 64;
            let offset_y = row * 64;
            let frame_idx = row * 4 + col;
            let radius = (frame_idx as f32 / 15.0 * 25.0) as i32;

            for ly in 0..64 {
                for lx in 0..64 {
                    let dx = lx as i32 - 32;
                    let dy = ly as i32 - 32;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < radius * radius {
                        let g = 100u8.saturating_add((frame_idx as u8).saturating_mul(8));
                        fire.put_pixel(offset_x + lx, offset_y + ly, Rgb([255, g, 0]));
                    }
                }
            }
        }
    }
    fire.save(base_path.join("fire_flipbook.png")).unwrap();

    println!("Generated 4 test assets in test_assets/");
}
