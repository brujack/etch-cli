use serde::{Deserialize, Serialize};

/// The result of checking one atom against the current machine state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AtomStatus {
    /// Declared state matches actual state.
    Ok,
    /// Atom has never been applied (target does not exist).
    Missing,
    /// Declared state differs from actual state.
    #[serde(rename = "drifted")]
    Drifted { expected: String, actual: String },
    /// This atom type cannot be checked (e.g. command.run, http.request).
    /// Reports ok in the exit-code sense; surfaced with dim styling.
    Unchecked,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_serializes_correctly() {
        let json = serde_json::to_string(&AtomStatus::Ok).unwrap();
        assert_eq!(json, r#"{"status":"ok"}"#);
    }

    #[test]
    fn missing_serializes_correctly() {
        let json = serde_json::to_string(&AtomStatus::Missing).unwrap();
        assert_eq!(json, r#"{"status":"missing"}"#);
    }

    #[test]
    fn unchecked_serializes_correctly() {
        let json = serde_json::to_string(&AtomStatus::Unchecked).unwrap();
        assert_eq!(json, r#"{"status":"unchecked"}"#);
    }

    #[test]
    fn drifted_serializes_with_expected_and_actual() {
        let s = AtomStatus::Drifted {
            expected: "foo".to_string(),
            actual: "bar".to_string(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["status"], "drifted");
        assert_eq!(v["expected"], "foo");
        assert_eq!(v["actual"], "bar");
    }

    #[test]
    fn roundtrip_ok() {
        let s = AtomStatus::Ok;
        let json = serde_json::to_string(&s).unwrap();
        let back: AtomStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn roundtrip_drifted() {
        let s = AtomStatus::Drifted {
            expected: "a".to_string(),
            actual: "b".to_string(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: AtomStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
