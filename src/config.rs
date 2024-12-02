// SPDX-License-Identifier: {{LICENSE}}

use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry},
    theme,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AppTheme {
    Dark,
    Light,
    System,
}

impl AppTheme {
    pub fn theme(&self) -> theme::Theme {
        match self {
            Self::Dark => {
                let mut t = theme::system_dark();
                t.theme_type.prefer_dark(Some(true));
                t
            }
            Self::Light => {
                let mut t = theme::system_light();
                t.theme_type.prefer_dark(Some(false));
                t
            }
            Self::System => theme::system_preference(),
        }
    }
}

pub const CONFIG_VERSION: u64 = 1;

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    pub app_theme: AppTheme,
    pub font_name: String,
    pub font_size: usize,
    pub syntax_theme_dark: String,
    pub syntax_theme_light: String,
}

impl Config {
    pub fn syntax_theme(&self) -> &str {
        let dark = self.app_theme.theme().theme_type.is_dark();
        if dark {
            &self.syntax_theme_dark
        } else {
            &self.syntax_theme_light
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_theme: AppTheme::System,

            font_name: "Fira Mono".to_string(),
            font_size: 20,

            syntax_theme_dark: "COSMIC Dark".to_string(),
            syntax_theme_light: "COSMIC Light".to_string(),
        }
    }
}
