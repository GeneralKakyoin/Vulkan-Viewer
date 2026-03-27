use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args.len() > 4 {
        eprintln!("Usage: screenshot_diff <baseline_dir> <candidate_dir> [max_mean_abs_error]");
        std::process::exit(2);
    }

    let baseline_dir = PathBuf::from(&args[1]);
    let candidate_dir = PathBuf::from(&args[2]);
    let max_mean_abs_error = args
        .get(3)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0)
        .max(0.0);

    let baseline = collect_pngs(&baseline_dir)?;
    let candidate = collect_pngs(&candidate_dir)?;
    if baseline.is_empty() {
        anyhow::bail!(
            "no png files found in baseline dir: {}",
            baseline_dir.display()
        );
    }

    let mut compared = 0usize;
    let mut failed = 0usize;
    for (name, baseline_path) in &baseline {
        let Some(candidate_path) = candidate.get(name) else {
            failed += 1;
            eprintln!("FAIL missing candidate image: {name}");
            continue;
        };

        compared += 1;
        match compare_png_pair(baseline_path, candidate_path) {
            Ok(score) => {
                if score > max_mean_abs_error {
                    failed += 1;
                    eprintln!(
                        "FAIL {name}: mean_abs_error={score:.6} threshold={max_mean_abs_error:.6}"
                    );
                } else {
                    println!(
                        "PASS {name}: mean_abs_error={score:.6} threshold={max_mean_abs_error:.6}"
                    );
                }
            }
            Err(err) => {
                failed += 1;
                eprintln!("FAIL {name}: {err:#}");
            }
        }
    }

    if failed > 0 {
        eprintln!(
            "screenshot_diff failed: compared={compared}, failed={failed}, baseline_total={}",
            baseline.len()
        );
        std::process::exit(1);
    }

    println!(
        "screenshot_diff passed: compared={compared}, failed={failed}, baseline_total={}",
        baseline.len()
    );
    Ok(())
}

fn collect_pngs(dir: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut map = BTreeMap::new();
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_png = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("png"))
            .unwrap_or(false);
        if !is_png {
            continue;
        }
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        map.insert(name.to_string(), path);
    }
    Ok(map)
}

fn compare_png_pair(baseline: &Path, candidate: &Path) -> Result<f64> {
    let baseline_img = image::open(baseline)
        .with_context(|| format!("failed to decode baseline png: {}", baseline.display()))?
        .to_rgba8();
    let candidate_img = image::open(candidate)
        .with_context(|| format!("failed to decode candidate png: {}", candidate.display()))?
        .to_rgba8();

    if baseline_img.width() != candidate_img.width()
        || baseline_img.height() != candidate_img.height()
    {
        anyhow::bail!(
            "dimension mismatch baseline={}x{} candidate={}x{}",
            baseline_img.width(),
            baseline_img.height(),
            candidate_img.width(),
            candidate_img.height()
        );
    }

    let a = baseline_img.as_raw();
    let b = candidate_img.as_raw();
    if a.len() != b.len() || a.is_empty() {
        anyhow::bail!("invalid image byte sizes for comparison");
    }

    let mut sum_abs: u64 = 0;
    for (lhs, rhs) in a.iter().zip(b.iter()) {
        sum_abs += lhs.abs_diff(*rhs) as u64;
    }

    Ok(sum_abs as f64 / a.len() as f64)
}
