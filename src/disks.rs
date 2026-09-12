use std::collections::BTreeMap;

/// One filesystem exactly as the operating system reported it, before any
/// filtering. `collect.rs` builds these from `sysinfo`; nothing here reads
/// the OS itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDisk {
    pub device: String,
    pub mount_point: String,
    pub total: u64,
    pub available: u64,
}

/// One real filesystem, identified by its device.
///
/// Note there is no mount point: a filesystem can be mounted at several paths
/// at once, and which paths those are is not a property of the hardware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filesystem {
    pub device: String,
    pub total: u64,
    pub available: u64,
}

/// Collapse a list of mounts into one entry per underlying filesystem.
///
/// The operating system reports every *mount*, not every filesystem, and
/// Kubernetes bind-mounts the same filesystem at many paths — `/etc/hosts`,
/// `/dev/termination-log`, a mounted ConfigMap, and so on. Left alone that
/// causes two bugs:
///
/// 1. One real change is reported six times, once per mount.
/// 2. Mounting anything new invents a brand new "disk", so the act of
///    configuring specdrift for drift detection *created* drift.
///
/// Keying by device fixes both, because a device is a filesystem's stable
/// identity while mount points come and go.
///
/// Filesystems with no capacity are dropped: they carry no information and
/// only add noise.
pub fn unique_filesystems(raw: Vec<RawDisk>) -> Vec<Filesystem> {
    // BTreeMap keeps devices in sorted order, so the output is deterministic
    // for the same reason Snapshot uses one.
    let mut by_device: BTreeMap<String, RawDisk> = BTreeMap::new();

    for disk in raw {
        if disk.total == 0 {
            continue;
        }

        // Several mounts of one device report identical numbers. Where they
        // somehow differ, keep the shortest mount path so the choice is
        // deterministic rather than dependent on the OS listing order.
        by_device
            .entry(disk.device.clone())
            .and_modify(|kept| {
                if disk.mount_point.len() < kept.mount_point.len() {
                    *kept = disk.clone();
                }
            })
            .or_insert(disk);
    }

    by_device
        .into_values()
        .map(|disk| Filesystem {
            device: disk.device,
            total: disk.total,
            available: disk.available,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(device: &str, mount: &str, total: u64, available: u64) -> RawDisk {
        RawDisk {
            device: device.to_string(),
            mount_point: mount.to_string(),
            total,
            available,
        }
    }

    #[test]
    fn no_disks_gives_no_filesystems() {
        assert!(unique_filesystems(vec![]).is_empty());
    }

    #[test]
    fn a_single_filesystem_is_kept() {
        let out = unique_filesystems(vec![raw("/dev/vda1", "/", 100, 40)]);
        assert_eq!(
            out,
            vec![Filesystem {
                device: "/dev/vda1".to_string(),
                total: 100,
                available: 40,
            }]
        );
    }

    #[test]
    fn bind_mounts_of_one_device_collapse_to_one() {
        // Exactly what a Kubernetes pod reports: the same ext4 filesystem
        // mounted at six paths, with identical numbers each time.
        let out = unique_filesystems(vec![
            raw("/dev/vda1", "/work", 100, 40),
            raw("/dev/vda1", "/etc/hosts", 100, 40),
            raw("/dev/vda1", "/etc/hostname", 100, 40),
            raw("/dev/vda1", "/etc/resolv.conf", 100, 40),
            raw("/dev/vda1", "/dev/termination-log", 100, 40),
        ]);
        assert_eq!(out.len(), 1, "six mounts of one device is one filesystem");
        assert_eq!(out[0].device, "/dev/vda1");
    }

    #[test]
    fn different_devices_are_all_kept() {
        let out = unique_filesystems(vec![
            raw("overlay", "/", 500, 200),
            raw("/dev/vda1", "/work", 100, 40),
        ]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn filesystems_with_no_capacity_are_dropped() {
        // A zero-byte filesystem carries no information and only adds noise.
        let out = unique_filesystems(vec![raw("tmpfs", "/proc/sys", 0, 0)]);
        assert!(out.is_empty());
    }

    #[test]
    fn output_is_sorted_by_device() {
        let out = unique_filesystems(vec![
            raw("zdev", "/z", 10, 5),
            raw("adev", "/a", 10, 5),
            raw("mdev", "/m", 10, 5),
        ]);
        let devices: Vec<&str> = out.iter().map(|f| f.device.as_str()).collect();
        assert_eq!(devices, vec!["adev", "mdev", "zdev"]);
    }

    /// Regression test for the bug found by running in Kubernetes.
    ///
    /// Switching specdrift to diff mode mounts the baseline ConfigMap, which
    /// added a brand new `disk./baseline.*` fact and made every run report
    /// drift. Keying by device means a new mount of an already-known device
    /// changes nothing.
    #[test]
    fn mounting_a_known_device_somewhere_new_changes_nothing() {
        let before = unique_filesystems(vec![
            raw("/dev/vda1", "/work", 100, 40),
            raw("/dev/vda1", "/etc/hosts", 100, 40),
        ]);

        let after = unique_filesystems(vec![
            raw("/dev/vda1", "/work", 100, 40),
            raw("/dev/vda1", "/etc/hosts", 100, 40),
            raw("/dev/vda1", "/baseline", 100, 40), // the ConfigMap mount
        ]);

        assert_eq!(before, after, "a new mount must not look like new hardware");
    }
}
