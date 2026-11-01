//! Light/dark theme preference. `System` follows the OS (`prefers-color-scheme`).

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

pub const STORAGE_KEY: &str = "theme";

impl Theme {
    /// Parse the value stored in localStorage; anything unknown means `System`.
    pub fn from_stored(s: Option<&str>) -> Self {
        match s {
            Some("light") => Theme::Light,
            Some("dark") => Theme::Dark,
            _ => Theme::System,
        }
    }

    /// Value for `data-theme` / localStorage; `None` for `System`.
    pub fn stored(self) -> Option<&'static str> {
        match self {
            Theme::System => None,
            Theme::Light => Some("light"),
            Theme::Dark => Some("dark"),
        }
    }

    /// Cycle order: System -> Light -> Dark -> System.
    pub fn next(self) -> Self {
        match self {
            Theme::System => Theme::Light,
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::System,
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Theme::System => "\u{25D0}", // ◐
            Theme::Light => "\u{2600}",  // ☀
            Theme::Dark => "\u{263E}",   // ☾
        }
    }

    pub fn label(self) -> String {
        let cur = match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        };
        let nxt = match self.next() {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        };
        format!("Theme: {cur}. Switch to {nxt} theme")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_through_all_three() {
        assert_eq!(Theme::System.next(), Theme::Light);
        assert_eq!(Theme::Light.next(), Theme::Dark);
        assert_eq!(Theme::Dark.next(), Theme::System);
    }

    #[test]
    fn stored_round_trips_and_garbage_is_system() {
        for t in [Theme::System, Theme::Light, Theme::Dark] {
            assert_eq!(Theme::from_stored(t.stored()), t);
        }
        assert_eq!(Theme::from_stored(Some("purple")), Theme::System);
        assert_eq!(Theme::from_stored(None), Theme::System);
    }

    #[test]
    fn label_names_current_and_next() {
        assert_eq!(Theme::Dark.label(), "Theme: dark. Switch to system theme");
    }
}
