use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, io};

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
    // https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script
    let outdir = env::var_os("OUT_DIR").expect("failed to find OUT_DIR");
    let outdir = PathBuf::from(outdir);
    fs::create_dir_all(&outdir).expect("failed to create dirs for OUT_DIR");

    // Create a stamp file. (This is used by CI to help find the OUT_DIR.)
    fs::write(Path::new(&outdir).join("repgrep-stamp"), "").unwrap();

    // Generate manpage.
    generate_manpage(&outdir).unwrap();
}
