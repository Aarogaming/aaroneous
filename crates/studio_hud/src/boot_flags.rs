//! DX-01: Safe Mode Boot Flag and Launch Configuration
//! Pure, injection-based launch flags without ambient environment reads.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchFlags {
    pub safe_mode: bool,
}

impl LaunchFlags {
    pub fn new(safe_mode: bool) -> Self {
        Self { safe_mode }
    }

    pub fn from_args<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut flags = Self::default();
        for arg in args {
            if arg.as_ref() == "--safe-mode" {
                flags.safe_mode = true;
            }
        }
        flags
    }
}

/// Checks if safe mode was requested via explicit launch flags
pub fn is_safe_mode_requested(flags: &LaunchFlags) -> bool {
    flags.safe_mode
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_flags_parsing() {
        let flags = LaunchFlags::from_args(vec!["--dev", "--safe-mode"]);
        assert!(is_safe_mode_requested(&flags));

        let clean_flags = LaunchFlags::from_args(vec!["--dev"]);
        assert!(!is_safe_mode_requested(&clean_flags));
    }
}
