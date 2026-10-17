//! Platform-independent BSD file-flag logic, kept outside the macOS-gated `chflags`
//! module so its tests run on the Linux CI runner.

pub const UF_HIDDEN: u32 = 0x8000;
pub const UF_IMMUTABLE: u32 = 0x0002;

/// The flags `file.flags` manages, paired with their `chflags(1)` names.
const MANAGED: [(u32, &str); 2] = [(UF_HIDDEN, "hidden"), (UF_IMMUTABLE, "uchg")];

/// Builds the symbolic `chflags(1)` argument that moves `current` to `desired`.
///
/// Symbolic `chflags` is additive: a name sets its flag, a `no`-prefixed name clears
/// it, and unnamed flags are left alone. So only the managed flags that change are
/// named, and clearing is spelled `nohidden`, never `none` (which `chflags` rejects).
/// Returns `None` when no managed flag differs, so there is nothing to run.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn symbolic_delta(current: u32, desired: u32) -> Option<String> {
    let words: Vec<String> = MANAGED
        .iter()
        .filter(|(bit, _)| (current ^ desired) & bit != 0)
        .map(|(bit, name)| {
            if desired & bit != 0 {
                (*name).to_string()
            } else {
                format!("no{name}")
            }
        })
        .collect();
    (!words.is_empty()).then(|| words.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const UF_NODUMP: u32 = 0x0001;

    /// Applies a symbolic argument the way `chflags(1)` does: each name sets its bit,
    /// each `no`-prefixed name clears it, and every flag left unnamed is untouched.
    fn apply(current: u32, arg: &str) -> u32 {
        arg.split(',').fold(current, |acc, word| {
            let (clear, name) = match word.strip_prefix("no") {
                Some(rest) => (true, rest),
                None => (false, word),
            };
            let bit = MANAGED
                .iter()
                .find(|(_, n)| *n == name)
                .unwrap_or_else(|| panic!("chflags rejects {word:?}"))
                .0;
            if clear {
                acc & !bit
            } else {
                acc | bit
            }
        })
    }

    #[test]
    fn no_change_needs_no_command() {
        assert_eq!(None, symbolic_delta(UF_HIDDEN, UF_HIDDEN));
        assert_eq!(None, symbolic_delta(0, 0));
    }

    #[test]
    fn setting_a_flag_names_it() {
        assert_eq!(Some("hidden".to_string()), symbolic_delta(0, UF_HIDDEN));
    }

    #[test]
    fn clearing_one_flag_names_it_with_no_prefix() {
        let current = UF_HIDDEN | UF_IMMUTABLE;
        assert_eq!(
            Some("nohidden".to_string()),
            symbolic_delta(current, UF_IMMUTABLE)
        );
    }

    #[test]
    fn clearing_every_flag_never_uses_none() {
        let arg = symbolic_delta(UF_HIDDEN | UF_IMMUTABLE, 0).unwrap();
        assert_eq!("nohidden,nouchg", arg);
    }

    #[test]
    fn setting_and_clearing_together_names_both() {
        assert_eq!(
            Some("nohidden,uchg".to_string()),
            symbolic_delta(UF_HIDDEN, UF_IMMUTABLE)
        );
    }

    #[test]
    fn unmanaged_flags_are_never_named() {
        let arg = symbolic_delta(UF_NODUMP, UF_NODUMP | UF_HIDDEN).unwrap();
        assert_eq!("hidden", arg);
    }

    proptest! {
        #[test]
        fn delta_applied_to_current_reaches_desired(
            current in 0u32..=0xFFFF,
            set_hidden in any::<Option<bool>>(),
            set_uchg in any::<Option<bool>>(),
        ) {
            let mut desired = current;
            for (want, bit) in [(set_hidden, UF_HIDDEN), (set_uchg, UF_IMMUTABLE)] {
                match want {
                    Some(true) => desired |= bit,
                    Some(false) => desired &= !bit,
                    None => {}
                }
            }
            match symbolic_delta(current, desired) {
                None => prop_assert_eq!(current, desired),
                Some(arg) => prop_assert_eq!(desired, apply(current, &arg)),
            }
        }
    }
}
