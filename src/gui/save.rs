//! Save WAV (UI `s123-UI` § 5.2; security review `s123-SH` § 4.7).
//!
//! The path comes from the user through a native dialog, so there is no traversal threat to
//! defend; the only attacker-influenced string near a path is the SUGGESTED name, built in
//! [`super::text::suggest_wav_name`] from the picture's stem only (V-1).
//!
//! The write itself is atomic (V-3): the library's own `synth_sink::write_stereo_wav` writes to a
//! temporary file in the SAME directory, which is then renamed over the target. A failed write
//! never truncates the user's previous file, and the temporary file is removed on every error.
//!
//! The samples written are exactly `RenderOutput.interleaved` at `RenderOutput.sample_rate`
//! (the Architect's S123 minimum, `s123-01` § 4): the window keeps the rendered buffer as data and
//! bakes no layout or rate into a type of its own.

use crate::synth_sink::write_stereo_wav;
use std::path::{Path, PathBuf};

/// Why a save failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SaveError {
    /// The target has no parent directory or no file name.
    #[error("not a file path: {0}")]
    BadPath(String),
    /// V-4: the target is the picture that is open.
    #[error("that is the picture you opened; saving there would replace it with sound")]
    WouldOverwritePicture,
    /// Writing the temporary file failed (the target is untouched).
    #[error("{0}")]
    Write(String),
    /// Renaming the temporary file over the target failed (the target is untouched).
    #[error("could not move the finished file into place: {0}")]
    Rename(String),
}

/// The temporary sibling for `target`: `.<file name>.partial-<pid>` in the same directory, so
/// the final rename never crosses a filesystem.
pub fn partial_path(target: &Path) -> Result<PathBuf, SaveError> {
    let parent = target
        .parent()
        .ok_or_else(|| SaveError::BadPath(target.display().to_string()))?;
    let name = target
        .file_name()
        .ok_or_else(|| SaveError::BadPath(target.display().to_string()))?;
    Ok(parent.join(format!(
        ".{}.partial-{}",
        name.to_string_lossy(),
        std::process::id()
    )))
}

/// V-3: write `interleaved` (stereo, `sample_rate`) to `target` atomically: temp file in the same
/// directory, then rename. On any error the temp file is removed and `target` is unchanged.
///
/// `s123-SH2` N-4: the temp name is predictable, so it is first CREATED with `create_new`, which
/// fails if anything (a file, or a symlink, dangling or not) already has that name. Only then does
/// the library's writer open it (`File::create` inside `write_stereo_wav`), so a planted symlink
/// can never redirect the write onto another file. In a sticky shared directory nobody else can
/// swap the file between the two opens.
pub fn write_atomic(target: &Path, sample_rate: u32, interleaved: &[f32]) -> Result<(), SaveError> {
    let tmp = partial_path(target)?;
    // Reserve the name. On failure nothing was created, so there is nothing to remove (and the
    // thing already at that name is NOT ours to remove).
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| SaveError::Write(format!("could not create {}: {e}", tmp.display())))?;
    if let Err(e) = write_stereo_wav(&tmp, sample_rate, interleaved) {
        let _ = std::fs::remove_file(&tmp);
        return Err(SaveError::Write(e.to_string()));
    }
    if let Err(e) = std::fs::rename(&tmp, target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(SaveError::Rename(e.to_string()));
    }
    Ok(())
}

/// V-4: refuse a target that is the open picture's own file.
pub fn check_not_the_picture(target: &Path, picture: Option<&Path>) -> Result<(), SaveError> {
    let Some(picture) = picture else {
        return Ok(());
    };
    let same = match (
        std::fs::canonicalize(target),
        std::fs::canonicalize(picture),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => target == picture,
    };
    if same {
        Err(SaveError::WouldOverwritePicture)
    } else {
        Ok(())
    }
}

/// V-1: where the save dialog opens: the last folder saved to this session, else the user's
/// Music folder, else Documents, else home. Never the working directory or the binary's folder.
pub fn default_save_dir(last_used: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = last_used {
        return Some(p.to_path_buf());
    }
    let dirs = directories::UserDirs::new()?;
    dirs.audio_dir()
        .or_else(|| dirs.document_dir())
        .map(Path::to_path_buf)
        .or_else(|| Some(dirs.home_dir().to_path_buf()))
}

/// V-2: does the window confirm an overwrite itself? The native Windows and macOS save dialogs
/// always confirm. On Linux and the BSDs the confirmation depends on the `rfd` backend
/// (xdg-desktop-portal: the portal implementation decides; a fallback may not ask), so it is not
/// guaranteed and the window asks itself when the target already exists.
pub const fn window_confirms_overwrite() -> bool {
    !cfg!(any(target_os = "windows", target_os = "macos"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frames: usize) -> Vec<f32> {
        (0..frames * 2)
            .map(|i| ((i / 2) as f32 * 0.05).sin() * 0.25)
            .collect()
    }

    #[test]
    fn t18_happy_path_replaces_and_reads_back() {
        let dir = crate::gui::test_scratch_dir("save-t18");
        let target = dir.join("out.wav");
        std::fs::write(&target, b"old contents").expect("seed old file");
        let buf = tone(4410);
        write_atomic(&target, 44_100, &buf).expect("save");
        let r = hound::WavReader::open(&target).expect("valid wav");
        let spec = r.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 44_100);
        assert_eq!(r.len() as usize, buf.len());
        assert!(!partial_path(&target).expect("partial").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_rate_is_carried_not_baked_in() {
        // The Architect's minimum: the writer takes the buffer's own rate.
        let dir = crate::gui::test_scratch_dir("save-rate");
        let target = dir.join("48k.wav");
        write_atomic(&target, 48_000, &tone(480)).expect("save");
        assert_eq!(
            hound::WavReader::open(&target)
                .expect("wav")
                .spec()
                .sample_rate,
            48_000
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn t17_failed_write_leaves_the_original_and_no_partial() {
        use std::os::unix::fs::PermissionsExt;
        let dir = crate::gui::test_scratch_dir("save-t17");
        let target = dir.join("keep.wav");
        std::fs::write(&target, b"the user's previous file").expect("seed");
        let before = std::fs::read(&target).expect("read");
        // Read-only directory: the temp file cannot be created.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).expect("chmod");
        let root = std::fs::File::create(dir.join("probe-write")).is_ok();
        let r = write_atomic(&target, 44_100, &tone(100));
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod back");
        if root {
            // Running as root: permissions are not enforced; the property cannot be tested here.
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        assert!(matches!(r, Err(SaveError::Write(_))), "{r:?}");
        assert_eq!(std::fs::read(&target).expect("read"), before);
        assert!(!partial_path(&target).expect("partial").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn n4_a_planted_symlink_at_the_temp_name_cannot_redirect_the_save() {
        let dir = crate::gui::test_scratch_dir("save-n4");
        let victim = dir.join("victim.txt");
        std::fs::write(&victim, b"the user's other file").expect("seed victim");
        let target = dir.join("piece.wav");
        let tmp = partial_path(&target).expect("partial");
        std::os::unix::fs::symlink(&victim, &tmp).expect("plant symlink");
        let r = write_atomic(&target, 44_100, &tone(100));
        assert!(matches!(r, Err(SaveError::Write(_))), "{r:?}");
        assert_eq!(
            std::fs::read(&victim).expect("victim"),
            b"the user's other file",
            "the file the link points at is untouched"
        );
        assert!(!target.exists(), "nothing was saved through the link");
        assert!(
            std::fs::symlink_metadata(&tmp).is_ok(),
            "the planted link is not ours to delete"
        );
        // A dangling link at the temp name is refused the same way.
        std::fs::remove_file(&tmp).expect("rm link");
        std::os::unix::fs::symlink(dir.join("nowhere"), &tmp).expect("dangling");
        assert!(matches!(
            write_atomic(&target, 44_100, &tone(100)),
            Err(SaveError::Write(_))
        ));
        assert!(!dir.join("nowhere").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v4_refuses_the_open_picture() {
        let dir = crate::gui::test_scratch_dir("save-v4");
        let pic = dir.join("photo.jpg");
        std::fs::write(&pic, b"jpeg").expect("seed");
        assert_eq!(
            check_not_the_picture(&pic, Some(&pic)),
            Err(SaveError::WouldOverwritePicture)
        );
        assert!(check_not_the_picture(&dir.join("photo.wav"), Some(&pic)).is_ok());
        assert!(check_not_the_picture(&pic, None).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_dir_prefers_the_last_used() {
        let p = PathBuf::from("/some/where");
        assert_eq!(default_save_dir(Some(&p)), Some(p));
    }
}
