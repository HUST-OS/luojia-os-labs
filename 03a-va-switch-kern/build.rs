use std::env;
use std::io::{Result, Write};
use std::fs::{self, File, read_dir};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/linker64.ld");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    fs::File::create(out_dir.join("linker64.ld"))
        .unwrap()
        .write_all(include_bytes!("src/linker64.ld"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());

    println!("cargo:rerun-if-changed=../03-mmu-users/src/");
    let target_dir = "target/riscv64imac-unknown-none-elf/debug/";
        // .to_string_lossy().replace("\\", "\\\\"); // 转义
    insert_app_data(&target_dir).unwrap();
}

fn insert_app_data(target_dir: &str) -> Result<()> {
    let mut f = File::create("src/link_apps.S").unwrap();
    let mut apps: Vec<_> = read_dir("../03-mmu-users/src/bin")
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    apps.sort();

    writeln!(f, r#"
    .align 3
    .section .data
    .global _app_meta
_app_meta:
    .quad {}
    "#, apps.len())?;

    for (i, name_with_ext) in apps.iter().enumerate() {
        writeln!(f, r#"    .quad {}"#, name_with_ext.len())?;
        writeln!(f, r#"    .asciz "{}""#, name_with_ext)?;
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(f, r#"
    .section .data
    .p2align 12
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "{2}/{1}"
app_{0}_end:"#, idx, app, target_dir)?;
    }
    Ok(())
}
