use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Mode {
    Build,
    Plan,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ToolPermission {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub network: bool,
}

impl Mode {
    pub fn toggle(&self) -> Self {
        match self {
            Mode::Build => Mode::Plan,
            Mode::Plan => Mode::Build,
        }
    }

    #[allow(dead_code)]
    pub fn color(&self) -> ratatui::style::Color {
        match self {
            Mode::Build => ratatui::style::Color::Rgb(0, 150, 255),
            Mode::Plan => ratatui::style::Color::Rgb(255, 165, 0),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Mode::Build => "BUILD",
            Mode::Plan => "PLAN",
        }
    }

    #[allow(dead_code)]
    pub fn permissions(&self) -> ToolPermission {
        match self {
            Mode::Build => ToolPermission { read: true, write: true, execute: true, network: true },
            Mode::Plan => ToolPermission { read: true, write: false, execute: false, network: false },
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_build_to_plan() {
        assert_eq!(Mode::Build.toggle(), Mode::Plan);
    }

    #[test]
    fn test_toggle_plan_to_build() {
        assert_eq!(Mode::Plan.toggle(), Mode::Build);
    }

    #[test]
    fn test_toggle_twice_returns_original() {
        assert_eq!(Mode::Build.toggle().toggle(), Mode::Build);
        assert_eq!(Mode::Plan.toggle().toggle(), Mode::Plan);
    }

    #[test]
    fn test_name() {
        assert_eq!(Mode::Build.name(), "BUILD");
        assert_eq!(Mode::Plan.name(), "PLAN");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Mode::Build), "BUILD");
        assert_eq!(format!("{}", Mode::Plan), "PLAN");
    }

    #[test]
    fn test_partial_eq() {
        assert_eq!(Mode::Build, Mode::Build);
        assert_ne!(Mode::Build, Mode::Plan);
    }

    #[test]
    fn test_build_permissions() {
        let p = Mode::Build.permissions();
        assert!(p.read);
        assert!(p.write);
        assert!(p.execute);
        assert!(p.network);
    }

    #[test]
    fn test_plan_permissions() {
        let p = Mode::Plan.permissions();
        assert!(p.read);
        assert!(!p.write);
        assert!(!p.execute);
        assert!(!p.network);
    }
}
