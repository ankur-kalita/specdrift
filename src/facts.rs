use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version of the snapshot file format.
pub const SNAPSHOT_VERSION: u32 = 1;

/// Whether a fact is expected to hold still or change constantly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FactClass {
    /// Does not change during normal use: CPU model, total RAM, OS name.
    Stable,
    /// Changes moment to moment: free memory, battery level, uptime.
    Volatile,
}

/// One recorded thing about a machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub value: String,
    pub class: FactClass,
}

impl Fact {
    pub fn stable(value: impl Into<String>) -> Self {
        Fact {
            value: value.into(),
            class: FactClass::Stable,
        }
    }

    pub fn volatile(value: impl Into<String>) -> Self {
        Fact {
            value: value.into(),
            class: FactClass::Volatile,
        }
    }
}

/// Everything recorded about a machine at one moment.
///
/// `facts` is a `BTreeMap` on purpose: it keeps keys sorted, so serializing
/// the same snapshot twice produces byte-identical JSON. Without that, every
/// diff would be full of phantom changes caused by reordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub captured_at: String,
    pub facts: BTreeMap<String, Fact>,
}

impl Snapshot {
    pub fn new(captured_at: impl Into<String>) -> Self {
        Snapshot {
            version: SNAPSHOT_VERSION,
            captured_at: captured_at.into(),
            facts: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, fact: Fact) {
        self.facts.insert(key.into(), fact);
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Snapshot {
        let mut s = Snapshot::new("2026-09-10T12:00:00Z");
        s.insert("cpu.model", Fact::stable("Apple M4"));
        s.insert("memory.total", Fact::stable("17179869184"));
        s.insert("memory.available", Fact::volatile("3221225472"));
        s
    }

    #[test]
    fn json_round_trip_is_identity() {
        let original = sample();
        let json = original.to_json().expect("serialize");
        let restored = Snapshot::from_json(&json).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn serialization_is_deterministic() {
        let first = sample().to_json().expect("serialize");
        let second = sample().to_json().expect("serialize");
        assert_eq!(first, second);
    }

    #[test]
    fn keys_serialize_in_sorted_order() {
        let json = sample().to_json().expect("serialize");
        let cpu = json.find("cpu.model").expect("cpu.model present");
        let avail = json.find("memory.available").expect("memory.available present");
        let total = json.find("memory.total").expect("memory.total present");
        assert!(cpu < avail, "cpu.model must come before memory.available");
        assert!(avail < total, "memory.available must come before memory.total");
    }

    #[test]
    fn snapshot_records_the_version() {
        assert_eq!(sample().version, SNAPSHOT_VERSION);
    }
}
