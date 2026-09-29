//! Windows builds embed the CatDo icon and version details in `catdo.exe`.
//! The icon is generated from the same PNG used for Linux packages.

fn main() {
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows::embed_resources();
    }
}

#[cfg(windows)]
mod windows {
    use image::{
        ExtendedColorType, ImageEncoder,
        codecs::{
            ico::{IcoEncoder, IcoFrame},
            png::PngEncoder,
        },
        imageops::FilterType,
    };
    use std::{env, fs::File, path::PathBuf};

    const ICON: &str = "../../assets/com.workercat.catdo.png";

    pub fn embed_resources() {
        println!("cargo:rerun-if-changed={ICON}");
        println!("cargo:rerun-if-changed=windows/catdo.rc");
        let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
        let source = image::open(ICON).expect("CatDo icon").into_rgba8();
        let frames = [16, 20, 24, 32, 40, 48, 64, 256]
            .into_iter()
            .map(|size| {
                let image = image::imageops::resize(&source, size, size, FilterType::Lanczos3);
                let mut png = Vec::new();
                PngEncoder::new(&mut png)
                    .write_image(&image, size, size, ExtendedColorType::Rgba8)
                    .unwrap();
                IcoFrame::with_encoded(png, size, size, ExtendedColorType::Rgba8).unwrap()
            })
            .collect::<Vec<_>>();
        let icon = out.join("catdo.ico");
        IcoEncoder::new(File::create(&icon).unwrap())
            .encode_images(&frames)
            .unwrap();

        let version = env::var("CARGO_PKG_VERSION").unwrap();
        let numeric = version.split(['-', '+']).next().unwrap().replace('.', ",");
        embed_resource::compile(
            "windows/catdo.rc",
            [
                format!(
                    "CATDO_ICON=\"{}\"",
                    icon.display().to_string().replace('\\', "/")
                ),
                format!("CATDO_VERSION=\"{version}\""),
                format!("CATDO_VERSION_NUMERIC={numeric},0"),
            ],
        )
        .manifest_required()
        .unwrap();
    }
}
