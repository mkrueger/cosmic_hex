// SPDX-License-Identifier: {{LICENSE}}

use std::sync::OnceLock;

use cosmic_text::SyntaxSystem;

mod app;
mod config;
pub mod hex_view;
mod i18n;
pub type HexResult<T> = anyhow::Result<T>;
pub static SYNTAX_SYSTEM: OnceLock<SyntaxSystem> = OnceLock::new();

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    SYNTAX_SYSTEM.get_or_init(|| {
        let lazy_theme_set = two_face::theme::LazyThemeSet::from(two_face::theme::extra());
        let mut theme_set = syntect::highlighting::ThemeSet::from(&lazy_theme_set);
        // Hardcoded COSMIC themes
        for (theme_name, theme_data) in &[
            ("COSMIC Dark", cosmic_syntax_theme::COSMIC_DARK_TM_THEME),
            ("COSMIC Light", cosmic_syntax_theme::COSMIC_LIGHT_TM_THEME),
        ] {
            let mut cursor = std::io::Cursor::new(theme_data);
            match syntect::highlighting::ThemeSet::load_from_reader(&mut cursor) {
                Ok(mut theme) => {
                    // Use libcosmic theme for background and gutter
                    theme.settings.background = Some(syntect::highlighting::Color { r: 0, g: 0, b: 0, a: 0 });
                    theme.settings.gutter = Some(syntect::highlighting::Color { r: 0, g: 0, b: 0, a: 0 });
                    theme_set.themes.insert(theme_name.to_string(), theme);
                }
                Err(err) => {
                    eprintln!("failed to load {:?} syntax theme: {}", theme_name, err);
                }
            }
        }
        SyntaxSystem {
            //TODO: store newlines in buffer
            syntax_set: two_face::syntax::extra_no_newlines(),
            theme_set,
        }
    });

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(cosmic::iced::Limits::NONE.min_width(360.0).min_height(180.0));

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}
