//! Locating ffmpeg (needed only for video backgrounds) and downloading a private copy.
//!
//! The private copy lives in the install dir (`ffmpeg\ffmpeg.exe`): admin-only, removed on
//! uninstall, and it never touches PATH or other ffmpeg installs.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::config;

/// gyan.dev "release essentials" build: includes libx264, published with a SHA-256.
const BUILD_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn private_dir() -> PathBuf {
    config::install_dir().join("ffmpeg")
}

pub fn command(path: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut c = Command::new(path);
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

fn works(path: &Path) -> bool {
    command(path).arg("-version").output().is_ok_and(|o| o.status.success())
}

/// First working ffmpeg: AIO_FFMPEG, the private copy, then any install on PATH.
pub fn find() -> Option<PathBuf> {
    let winget_link = std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join(r"Microsoft\WinGet\Links\ffmpeg.exe"));
    std::env::var_os("AIO_FFMPEG")
        .map(PathBuf::from)
        .into_iter()
        .chain([private_dir().join("ffmpeg.exe"), PathBuf::from("ffmpeg")])
        .chain(winget_link)
        .find(|p| works(p))
}

/// Download the build, verify its SHA-256, and extract ffmpeg.exe (plus its licence) into
/// `dest`. `progress` receives (bytes downloaded, total bytes if known).
pub fn download(dest: &Path, mut progress: impl FnMut(u64, Option<u64>)) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("cannot create {}: {e}", dest.display()))?;
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(15))).build().into();

    let expected = agent
        .get(&format!("{BUILD_URL}.sha256"))
        .call()
        .map_err(|e| format!("cannot reach gyan.dev: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut resp = agent.get(BUILD_URL).call().map_err(|e| format!("download failed: {e}"))?;
    let total = resp.body().content_length();
    let zip_path = dest.join("ffmpeg-download.zip");
    let mut file = std::fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut reader = resp.body_mut().with_config().limit(1 << 30).reader();
    let mut hasher = Sha256::new();
    let (mut buf, mut done) = (vec![0u8; 1 << 16], 0u64);
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("download failed: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        progress(done, total);
    }
    drop(file);

    let actual: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if expected.len() != 64 || actual != expected {
        let _ = std::fs::remove_file(&zip_path);
        return Err("download was corrupted (SHA-256 mismatch); try again".into());
    }

    let result = (|| {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&zip_path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let names: Vec<String> = zip.file_names().map(String::from).collect();
        let exe = names.iter().find(|n| n.ends_with("/bin/ffmpeg.exe")).ok_or("ffmpeg.exe not found in the download")?;
        extract(&mut zip, exe, &dest.join("ffmpeg.exe"))?;
        if let Some(lic) = names.iter().find(|n| n.ends_with("/LICENSE")) {
            extract(&mut zip, lic, &dest.join("LICENSE.txt"))?;
        }
        Ok(dest.join("ffmpeg.exe"))
    })();
    let _ = std::fs::remove_file(&zip_path);
    result
}

fn extract(zip: &mut zip::ZipArchive<std::fs::File>, name: &str, to: &Path) -> Result<(), String> {
    let mut entry = zip.by_name(name).map_err(|e| e.to_string())?;
    let tmp = to.with_extension("part");
    let mut out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    drop(out);
    std::fs::rename(&tmp, to).map_err(|e| e.to_string())
}

/// `aio-screen fetch-ffmpeg [result-file]`: download into the private dir. Progress is
/// written to `<result-file>.progress` as "done total" for the GUI to poll.
pub fn fetch_command(result_file: Option<&String>) -> Result<(), String> {
    let progress_file = result_file.map(|f| PathBuf::from(format!("{f}.progress")));
    let mut last = 0u64;
    download(&private_dir(), |done, total| {
        if let Some(p) = &progress_file {
            if done - last >= 512 * 1024 || Some(done) == total {
                last = done;
                let _ = std::fs::write(p, format!("{done} {}", total.unwrap_or(0)));
            }
        }
    })
    .map(|_| ())
}
