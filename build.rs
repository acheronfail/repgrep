use clap::{IntoApp, crate_name};
use clap_generate::{generate, generators};
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
#[path = "src/cli/args.rs"]
mod cli;

fn generate_manpage<P: AsRef<Path>>(outdir: P) -> io::Result<()> {
    // If asciidoctor isn't installed, don't do anything.
    // This is for platforms where it's unsupported.
    if let Err(err) = Command::new("asciidoctor").output() {
        eprintln!("Could not run 'asciidoctor' binary, skipping man page generation.");
        eprintln!("Error from running 'asciidoctor': {}", err);
        return Ok(());
    }

    let outdir = outdir.as_ref();
    let cwd = env::current_dir()?;
    let template_path = cwd.join("doc").join("rgr.1.template");

    let result = Command::new("asciidoctor")
        .arg("--doctype")
        .arg("manpage")
        .arg("--backend")
        .arg("manpage")
        .arg("--destination-dir")
        .arg(&outdir)
        .arg(&template_path)
        .spawn()?
        .wait()?;

    if !result.success() {
        let msg = format!("'asciidoctor' failed with exit code {:?}", result.code());
        return Err(io::Error::new(io::ErrorKind::Other, msg));
    }
    Ok(())
}

fn main() {
    let mut app = cli::Args::into_app();
    let name = crate_name!();
    let bin_name = "rgr";

    // https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script
    let outdir = env::var_os("OUT_DIR").expect("failed to find OUT_DIR");
    fs::create_dir_all(&outdir).expect("failed to create dirs for OUT_DIR");

    // Create a stamp file. (This is used by CI to help find the OUT_DIR.)
    fs::write(Path::new(&outdir).join("repgrep-stamp"), "").unwrap();

    // Generate completions.
    let f = |name: &str| File::create(Path::new(&outdir).join(name)).unwrap();
    generate::<generators::Zsh, _>(&mut app, name, &mut f(&format!("_{}", bin_name)));
    generate::<generators::Bash, _>(&mut app, name, &mut f(&format!("{}.bash", bin_name)));
    generate::<generators::Fish, _>(&mut app, name, &mut f(&format!("{}.fish", bin_name)));
    generate::<generators::Elvish, _>(&mut app, name, &mut f(&format!("{}.elvish", bin_name)));
    generate::<generators::PowerShell, _>(&mut app, name, &mut f(&format!("_{}.ps1", bin_name)));

    // Generate manpage.
    generate_manpage(&outdir).unwrap();
}
