//! Windows: вшить иконку в exe. Манифест не добавляем — его уже даёт eframe/winit,
//! второй манифест ломает линковку.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let png_path = manifest.join("../../assets/icons/icon.png");
    println!("cargo:rerun-if-changed={}", png_path.display());
    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }
    let png = fs::read(&png_path).expect("assets/icons/icon.png");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let ico = out_dir.join("icon.ico");
    fs::write(&ico, png_ico(&png)).expect("write icon.ico");
    let rc = out_dir.join("app.rc");
    let ico_rc = ico.display().to_string().replace('\\', "/");
    fs::write(&rc, format!("1 ICON \"{ico_rc}\"\r\n")).expect("write app.rc");
    embed_resource::compile(&rc, embed_resource::NONE)
        .manifest_optional()
        .expect("embed Windows icon");
}

/// Один PNG внутри ICO (width/height 0 = 256 и больше). Так rc.exe принимает 512×512.
fn png_ico(png: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(22 + png.len());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&(png.len() as u32).to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes());
    out.extend_from_slice(png);
    out
}
