#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    P240,
    P360,
    P480,
    P720,
    P1080,
    P1440,
    P2160,
}

impl Resolution {
    pub const ALL: &[Resolution] = &[
        Resolution::P240,
        Resolution::P360,
        Resolution::P480,
        Resolution::P720,
        Resolution::P1080,
        Resolution::P1440,
        Resolution::P2160,
    ];

    pub fn height(self) -> u32 {
        match self {
            Resolution::P240 => 240,
            Resolution::P360 => 360,
            Resolution::P480 => 480,
            Resolution::P720 => 720,
            Resolution::P1080 => 1080,
            Resolution::P1440 => 1440,
            Resolution::P2160 => 2160,
        }
    }

    pub fn available_for(source_height: u32) -> Vec<Resolution> {
        Self::ALL
            .iter()
            .filter(|r| r.height() < source_height)
            .copied()
            .collect()
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}p", self.height())
    }
}
