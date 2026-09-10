use crate::facts::{Fact, Snapshot};
use sysinfo::{Disks, System};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// Ask this machine what it is made of.
///
/// This is the only place in the program that talks to the operating system.
/// It contains no decisions — it just fills in a map — so there is very little
/// here that can be wrong, and nothing here that needs testing on three
/// different operating systems.
pub fn collect() -> Snapshot {
    let captured_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());

    let mut snapshot = Snapshot::new(captured_at);

    let mut system = System::new_all();
    system.refresh_all();

    // --- operating system -------------------------------------------------
    if let Some(name) = System::name() {
        snapshot.insert("os.name", Fact::stable(name));
    }
    if let Some(version) = System::os_version() {
        snapshot.insert("os.version", Fact::stable(version));
    }
    if let Some(kernel) = System::kernel_version() {
        snapshot.insert("os.kernel", Fact::stable(kernel));
    }
    snapshot.insert("os.arch", Fact::stable(std::env::consts::ARCH));

    // --- cpu --------------------------------------------------------------
    snapshot.insert("cpu.count", Fact::stable(system.cpus().len().to_string()));
    if let Some(cpu) = system.cpus().first() {
        snapshot.insert("cpu.brand", Fact::stable(cpu.brand().trim()));
        snapshot.insert(
            "cpu.frequency_mhz",
            Fact::stable(cpu.frequency().to_string()),
        );
    }

    // --- memory (bytes) ---------------------------------------------------
    snapshot.insert(
        "memory.total",
        Fact::stable(system.total_memory().to_string()),
    );
    snapshot.insert(
        "memory.available",
        Fact::volatile(system.available_memory().to_string()),
    );
    snapshot.insert(
        "memory.used",
        Fact::volatile(system.used_memory().to_string()),
    );
    snapshot.insert("swap.total", Fact::stable(system.total_swap().to_string()));

    // --- disks ------------------------------------------------------------
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        let mount = disk.mount_point().display().to_string();
        snapshot.insert(
            format!("disk.{mount}.total"),
            Fact::stable(disk.total_space().to_string()),
        );
        snapshot.insert(
            format!("disk.{mount}.available"),
            Fact::volatile(disk.available_space().to_string()),
        );
    }

    // --- uptime -----------------------------------------------------------
    snapshot.insert(
        "system.uptime_secs",
        Fact::volatile(System::uptime().to_string()),
    );

    snapshot
}
