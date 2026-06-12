use std::fmt;

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Build,
    Plan,
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
            Mode::Build => ratatui::style::Color::Rgb(0, 150, 255),   // Blue
            Mode::Plan => ratatui::style::Color::Rgb(255, 165, 0),    // Orange
        }
    }
    
    pub fn name(&self) -> &str {
        match self {
            Mode::Build => "BUILD",
            Mode::Plan => "PLAN",
        }
    }
    
    pub fn writes_allowed(&self) -> bool {
        match self {
            Mode::Build => true,
            Mode::Plan => false,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
