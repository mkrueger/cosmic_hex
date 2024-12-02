use crate::{hex_view::HexView, SYNTAX_SYSTEM};
use cosmic::{iced::Point, widget::Icon};
use std::path::PathBuf;

pub enum Tab {
    Editor(EditorTab),
}

impl Tab {
    pub fn is_dirty(&self) -> bool {
        match self {
            Tab::Editor(tab) => tab.hex_view.is_dirty(),
            //  _ => false
        }
    }
}

pub struct EditorTab {
    pub hex_view: HexView,
    pub _context_menu: Option<Point>,
}

impl EditorTab {
    pub(crate) fn new(path: PathBuf, buf: crate::hex_view::buffer::DataBuffer) -> Self {
        Self {
            hex_view: HexView::new(path, buf),
            _context_menu: None,
        }
    }

    pub(crate) fn title(&self) -> String {
        self.hex_view.path.file_name().unwrap().to_string_lossy().to_string()
    }

    pub(crate) fn icon(&self, _size: u16) -> Icon {
        cosmic::widget::icon::from_name("applications-science-symbolic").handle().icon()
        // TODO:
        // cosmic::widget::icon::icon(mime_icon(mime_for_path(path), size)).size(size)
    }

    pub(crate) fn set_config(&mut self, config: &crate::config::Config) {
        if let Some(theme) = SYNTAX_SYSTEM.get().unwrap().theme_set.themes.get(config.syntax_theme()) {
            self.hex_view.theme.caret = convert_color(theme.settings.caret);
            self.hex_view.theme.background = convert_color(theme.settings.background);
            self.hex_view.theme.offset_number = convert_color(theme.settings.gutter_foreground);
            self.hex_view.theme.hex = convert_color(theme.settings.foreground);
            self.hex_view.theme.ascii = convert_color(theme.settings.foreground);
        }

        self.hex_view.font_size = config.font_size as f32;
        self.hex_view.update_font();
        self.hex_view.redraw();
    }
}

fn convert_color(background: Option<syntect::highlighting::Color>) -> cosmic::iced::Color {
    if let Some(color) = background {
        cosmic::iced::Color::from_rgba8(color.r, color.g, color.b, color.a as f32 / 255.0)
    } else {
        cosmic::iced::Color::WHITE
    }
}
