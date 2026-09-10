use crate::facts::{FactClass, Snapshot};
use std::collections::BTreeSet;

/// One difference between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Added {
        key: String,
        value: String,
    },
    Removed {
        key: String,
        value: String,
    },
    Changed {
        key: String,
        from: String,
        to: String,
    },
}

impl Change {
    pub fn key(&self) -> &str {
        match self {
            Change::Added { key, .. } => key,
            Change::Removed { key, .. } => key,
            Change::Changed { key, .. } => key,
        }
    }
}

/// Compare two snapshots.
///
/// Volatile facts are skipped unless `include_volatile` is true — otherwise
/// free memory alone would make every comparison report drift.
///
/// This function deliberately never touches the operating system. It takes two
/// plain structs and returns a plain list, which is why it can be tested on any
/// machine, including a CI runner with completely different hardware.
pub fn diff(baseline: &Snapshot, current: &Snapshot, include_volatile: bool) -> Vec<Change> {
    let mut keys: BTreeSet<&str> = BTreeSet::new();
    keys.extend(baseline.facts.keys().map(String::as_str));
    keys.extend(current.facts.keys().map(String::as_str));

    let mut changes = Vec::new();

    for key in keys {
        let before = baseline.facts.get(key);
        let after = current.facts.get(key);

        // Classify by whichever side we have; prefer the current one.
        let class = after
            .or(before)
            .map(|fact| fact.class)
            .unwrap_or(FactClass::Stable);

        if class == FactClass::Volatile && !include_volatile {
            continue;
        }

        match (before, after) {
            (Some(before), Some(after)) if before.value != after.value => {
                changes.push(Change::Changed {
                    key: key.to_string(),
                    from: before.value.clone(),
                    to: after.value.clone(),
                });
            }
            (Some(_), Some(_)) => {}
            (None, Some(after)) => changes.push(Change::Added {
                key: key.to_string(),
                value: after.value.clone(),
            }),
            (Some(before), None) => changes.push(Change::Removed {
                key: key.to_string(),
                value: before.value.clone(),
            }),
            (None, None) => unreachable!("key came from one of the two maps"),
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{Fact, Snapshot};

    fn snap(pairs: &[(&str, Fact)]) -> Snapshot {
        let mut s = Snapshot::new("2026-09-10T12:00:00Z");
        for (key, fact) in pairs {
            s.insert(*key, fact.clone());
        }
        s
    }

    #[test]
    fn identical_snapshots_have_no_changes() {
        let a = snap(&[("cpu.model", Fact::stable("Apple M4"))]);
        let b = snap(&[("cpu.model", Fact::stable("Apple M4"))]);
        assert!(diff(&a, &b, false).is_empty());
    }

    #[test]
    fn changed_stable_value_is_reported() {
        let a = snap(&[("os.version", Fact::stable("15.6"))]);
        let b = snap(&[("os.version", Fact::stable("15.7"))]);
        assert_eq!(
            diff(&a, &b, false),
            vec![Change::Changed {
                key: "os.version".to_string(),
                from: "15.6".to_string(),
                to: "15.7".to_string(),
            }]
        );
    }

    #[test]
    fn new_key_is_reported_as_added() {
        let a = snap(&[]);
        let b = snap(&[("disk.usb", Fact::stable("62.9 GB"))]);
        assert_eq!(
            diff(&a, &b, false),
            vec![Change::Added {
                key: "disk.usb".to_string(),
                value: "62.9 GB".to_string(),
            }]
        );
    }

    #[test]
    fn missing_key_is_reported_as_removed() {
        let a = snap(&[("disk.usb", Fact::stable("62.9 GB"))]);
        let b = snap(&[]);
        assert_eq!(
            diff(&a, &b, false),
            vec![Change::Removed {
                key: "disk.usb".to_string(),
                value: "62.9 GB".to_string(),
            }]
        );
    }

    #[test]
    fn volatile_changes_are_ignored_by_default() {
        let a = snap(&[("memory.available", Fact::volatile("9000"))]);
        let b = snap(&[("memory.available", Fact::volatile("3000"))]);
        assert!(diff(&a, &b, false).is_empty());
    }

    #[test]
    fn volatile_changes_are_reported_when_requested() {
        let a = snap(&[("memory.available", Fact::volatile("9000"))]);
        let b = snap(&[("memory.available", Fact::volatile("3000"))]);
        assert_eq!(
            diff(&a, &b, true),
            vec![Change::Changed {
                key: "memory.available".to_string(),
                from: "9000".to_string(),
                to: "3000".to_string(),
            }]
        );
    }

    #[test]
    fn changes_come_back_in_sorted_key_order() {
        let a = snap(&[("zulu", Fact::stable("1")), ("alpha", Fact::stable("1"))]);
        let b = snap(&[("zulu", Fact::stable("2")), ("alpha", Fact::stable("2"))]);
        let changes = diff(&a, &b, false);
        let keys: Vec<&str> = changes.iter().map(|c| c.key()).collect();
        assert_eq!(keys, vec!["alpha", "zulu"]);
    }
}
