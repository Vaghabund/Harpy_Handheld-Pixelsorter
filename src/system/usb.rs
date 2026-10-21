use std::path::PathBuf;

/// Filesystem types used by USB sticks we can export to
const USB_FS_TYPES: [&str; 3] = ["vfat", "exfat", "ntfs"];

/// Find the first writable USB drive mounted under /media/
pub fn find_writable_usb_mount() -> Option<PathBuf> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    usb_mount_points(&mounts).into_iter().find(|mount_point| {
        // Test if we can write to it
        let test_file = mount_point.join(".pixelsort_usb_check");
        if std::fs::write(&test_file, "test").is_ok() {
            let _ = std::fs::remove_file(&test_file);
            log::debug!("Found writable USB drive at: {}", mount_point.display());
            true
        } else {
            log::debug!("Cannot write to USB at: {}", mount_point.display());
            false
        }
    })
}

/// Parse /proc/mounts content and return USB-like mount points under /media/
fn usb_mount_points(mounts: &str) -> Vec<PathBuf> {
    mounts
        .lines()
        .filter_map(|line| {
            // Format: <device> <mount point> <fs type> <options> <dump> <pass>
            let mut fields = line.split_whitespace();
            let mount_point = fields.next().and(fields.next())?;
            let fs_type = fields.next()?;
            let mount_point = unescape_mount_path(mount_point);
            (mount_point.starts_with("/media/") && USB_FS_TYPES.contains(&fs_type))
                .then(|| PathBuf::from(mount_point))
        })
        .collect()
}

/// /proc/mounts escapes space, tab, newline and backslash as octal (e.g. "\040")
fn unescape_mount_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() && bytes[i + 1..i + 4].iter().all(|b| (b'0'..=b'7').contains(b)) {
            let code = bytes[i + 1..i + 4].iter().fold(0u32, |acc, b| acc * 8 + u32::from(b - b'0'));
            out.push(code as u8);
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_usb_mounts_under_media() {
        let mounts = "\
/dev/mmcblk0p2 / ext4 rw,noatime 0 0
/dev/sda1 /media/pixelsort/STICK vfat rw,nosuid 0 0
/dev/sdb1 /media/pixelsort/MY\\040DRIVE exfat rw 0 0
/dev/sdc1 /mnt/backup vfat rw 0 0
/dev/sdd1 /media/pixelsort/LINUX ext4 rw 0 0
";
        assert_eq!(
            usb_mount_points(mounts),
            vec![
                PathBuf::from("/media/pixelsort/STICK"),
                PathBuf::from("/media/pixelsort/MY DRIVE"),
            ]
        );
    }
}
