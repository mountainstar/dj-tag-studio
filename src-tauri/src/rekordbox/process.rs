use sysinfo::{ProcessRefreshKind, RefreshKind, System};

const BLOCKING_PROCESS_NAMES: &[&str] = &["rekordbox", "rekordboxagent"];

pub fn blocking_processes() -> Vec<String> {
    let mut system = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut found = Vec::new();
    for process in system.processes().values() {
        let name = process.name().to_string_lossy().to_lowercase();
        if BLOCKING_PROCESS_NAMES
            .iter()
            .any(|needle| name.contains(needle))
        {
            found.push(process.name().to_string_lossy().into_owned());
        }
    }
    found.sort();
    found.dedup();
    found
}

pub fn is_rekordbox_running() -> bool {
    !blocking_processes().is_empty()
}

pub fn rekordbox_write_block_reason() -> Option<String> {
    let processes = blocking_processes();
    if processes.is_empty() {
        return None;
    }
    Some(format!(
        "Close Rekordbox before writing tags (running: {})",
        processes.join(", ")
    ))
}
