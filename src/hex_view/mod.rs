use std::{cell::Cell, path::PathBuf};

pub mod buffer;
pub mod hexviewwidget;
pub mod theme;
pub mod undo;

use buffer::DataBuffer;
use cosmic::iced_core::Text;
use cosmic::{
    iced::{
        alignment::{Horizontal, Vertical},
        widget::canvas::Cache,
        window::Event,
        Font, Pixels, Point, Rectangle, Renderer, Size,
    },
    iced_core::{
        self,
        text::{Shaping, Wrapping},
    },
    iced_widget::scrollable::{self, AbsoluteOffset},
    widget::Id,
    Task,
};
use theme::Theme;
use undo::UndoOperation;

use crate::HexResult;

#[derive(Default, PartialEq, Eq, Debug)]
pub enum EditMode {
    #[default]
    Hex,
    Ascii,
}

#[derive(Default)]
pub struct Cursor {
    pub position: usize,
    pub blink: bool,
    pub focus: bool,
    pub in_hex: EditMode,
}

pub struct HexView {
    pub path: PathBuf,
    pub theme: Theme,
    pub cache: Cache,
    pub font: Font,
    pub font_size: f32,
    pub scale_factor: f32,
    pub font_measure: Size<f32>,
    pub viewport: Cell<Rectangle>,

    pub cursor: Cursor,

    pub buffer: Option<DataBuffer>,
    pub last_save: usize,
    pub undo_buffer: Vec<Box<dyn UndoOperation>>,
    pub redo_buffer: Vec<Box<dyn UndoOperation>>,
    pub id: Id,
}

#[derive(Debug, Clone)]
pub enum Message {
    Increment,
    Term(Event),
    Redraw,
    MoveCaret(usize),
    TypeChar(char),
    SetFocus(bool),
    Click(Point),
    SwitchMode,
    PageUp,
    PageDown,
}
type Plain = iced_core::text::paragraph::Plain<<Renderer as iced_core::text::Renderer>::Paragraph>;

impl HexView {
    pub fn redraw(&mut self) {
        self.cache.clear();
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
        self.font_measure = Self::font_measure(self.font_size, self.scale_factor, self.font);
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
        self.font_measure = Self::font_measure(self.font_size, self.scale_factor, self.font);
    }

    fn font_measure(font_size: f32, scale_factor: f32, font: Font) -> Size<f32> {
        let paragraph = Plain::new(Text {
            content: "X",
            font,
            size: Pixels(font_size),
            vertical_alignment: Vertical::Center,
            horizontal_alignment: Horizontal::Center,
            shaping: Shaping::Advanced,
            line_height: cosmic::iced_core::text::LineHeight::Relative(scale_factor),
            bounds: Size::INFINITY,
            wrapping: Wrapping::Glyph,
        });

        paragraph.min_bounds()
    }

    pub fn update_font(&mut self) {
        self.font_measure = Self::font_measure(self.font_size, self.scale_factor, self.font);
    }

    pub(crate) fn new(path: PathBuf, buffer: DataBuffer) -> Self {
        let font_size = 16.0;
        let scale_factor = 1.0;
        let font = Font::MONOSPACE;
        let font_measure = Self::font_measure(font_size, scale_factor, font);
        Self {
            path,
            theme: Theme::default(),
            cache: Cache::default(),
            cursor: Cursor {
                position: 0,
                blink: false,
                focus: true,
                in_hex: EditMode::Hex,
            },
            font,
            font_size,
            scale_factor,
            font_measure,
            buffer: Some(buffer),
            viewport: Cell::new(Rectangle::default()),
            id: Id::unique(),
            last_save: 0,
            undo_buffer: Vec::new(),
            redo_buffer: Vec::new(),
        }
    }

    pub(crate) fn numbers_in_row(&self) -> usize {
        let char_width = self.font_measure.width;
        let width = self.viewport.get().width;

        let offset_margin_width: f32 = self.theme.calc_offset_margin_width(self.font_measure);
        let cell_width = self.theme.calc_cell_width(self.font_measure);

        for i in 2.. {
            if offset_margin_width + (i as f32) * cell_width + self.theme.hex_ascii_spacing() + (i as f32) * char_width > width {
                return i - 1;
            }
        }
        return 1;
    }

    fn scroll_to_caret(&self) -> Task<Message> {
        let numbers_in_row = self.numbers_in_row();
        let row = self.cursor.position / (numbers_in_row * 2);
        let row = row as f32;
        let row = row * self.font_measure.height;

        let y = self.viewport.get().y;
        let height = self.viewport.get().height;
        if row < y {
            scrollable::scroll_to::<Message>(self.id.clone(), AbsoluteOffset { x: 0.0, y: row })
        } else if row > y + height {
            scrollable::scroll_to::<Message>(
                self.id.clone(),
                AbsoluteOffset {
                    x: 0.0,
                    y: row + self.font_measure.height - height,
                },
            )
        } else {
            Task::none()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Redraw => {
                self.redraw();
            }

            Message::MoveCaret(position) => {
                self.cursor.position = position.clamp(0, (self.buffer.as_ref().unwrap().len() - 1) * 2);
                self.redraw();
                return self.scroll_to_caret();
            }

            Message::TypeChar(ch) => {
                if let Some(buffer) = &mut self.buffer {
                    let first_char = self.cursor.position % 2 == 0;
                    let pos = self.cursor.position / 2;
                    let old_byte = buffer.data[pos];
                    if self.cursor.in_hex == EditMode::Hex {
                        if ch.is_ascii_hexdigit() {
                            let digit = ch.to_digit(16).unwrap() as u8;
                            let new_byte = if first_char {
                                (old_byte & 0x0F) | (digit << 4)
                            } else {
                                (old_byte & 0xF0) | digit
                            };

                            let operation = undo::UndoChangeByte::new(pos, self.cursor.position, old_byte, self.cursor.position + 1, new_byte);
                            return self.commit_operation(operation);
                        }
                    } else {
                        let new_byte = ch as u8;
                        let operation = undo::UndoChangeByte::new(pos, self.cursor.position, old_byte, pos * 2 + 2, new_byte);
                        return self.commit_operation(operation);
                    }
                }
            }

            Message::SetFocus(focus) => {
                self.cursor.focus = focus;
            }

            Message::Click(point) => {
                let numbers_in_row = self.numbers_in_row();

                let char_width = self.font_measure.width;
                let left_margin: f32 = 9.0 * char_width;
                let x = point.x - left_margin;

                let cell_width = self.theme.calc_cell_width(self.font_measure);
                let numbers_width = (numbers_in_row as f32) * cell_width;
                let text_width = (numbers_in_row as f32) * char_width;

                if x >= 0.0 {
                    if x <= numbers_width {
                        let clicked_cell = (x / cell_width) as usize;
                        let clicked_cell_x = x - (clicked_cell as f32 * cell_width);

                        let mut position = ((point.y / self.font_measure.height) as usize * numbers_in_row + clicked_cell) * 2;

                        if clicked_cell_x > char_width {
                            position += 1;
                        }

                        self.cursor.position = position;
                        self.cursor.in_hex = EditMode::Hex;
                    } else {
                        let x = x - numbers_width;
                        if x < text_width {
                            let number = (x / char_width) as usize;
                            let position = (point.y / self.font_measure.height) as usize * numbers_in_row + number;
                            self.cursor.position = position * 2;
                            self.cursor.in_hex = EditMode::Ascii;
                        }
                    }
                }

                self.redraw();
            }
            Message::SwitchMode => {
                if self.cursor.in_hex == EditMode::Hex {
                    self.cursor.in_hex = EditMode::Ascii;
                } else {
                    self.cursor.in_hex = EditMode::Hex;
                }
                self.redraw();
            }

            Message::PageUp => {
                let numbers_in_row = self.numbers_in_row();
                let height = self.viewport.get().height;
                let line_count = height / self.font_measure.height;

                self.cursor.position = self.cursor.position.saturating_sub(line_count as usize * numbers_in_row * 2);
                self.redraw();

                return scrollable::scroll_to::<Message>(
                    self.id.clone(),
                    AbsoluteOffset {
                        x: 0.0,
                        y: (self.viewport.get().y - height).max(0.0),
                    },
                );
            }

            Message::PageDown => {
                let numbers_in_row = self.numbers_in_row();
                let height = self.viewport.get().height;
                let line_count = height / self.font_measure.height;

                self.cursor.position += line_count as usize * numbers_in_row * 2;
                let _ = self.cursor.position.clamp(0, self.buffer.as_ref().unwrap().len());
                self.redraw();

                return scrollable::scroll_to::<Message>(
                    self.id.clone(),
                    AbsoluteOffset {
                        x: 0.0,
                        y: self.viewport.get().y + height,
                    },
                );
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.undo_buffer.len() != self.last_save
    }

    fn commit_operation(&mut self, operation: undo::UndoChangeByte) -> Task<Message> {
        let _ = operation.redo(self);
        self.redo_buffer.clear();
        self.undo_buffer.push(Box::new(operation));
        self.redraw();
        self.scroll_to_caret()
    }

    pub(crate) fn save(&mut self) -> HexResult<()> {
        if let Some(data) = &self.buffer {
            self.last_save = self.undo_buffer.len();
            std::fs::write(&self.path, &data.data)?;
        }
        Ok(())
    }

    pub(crate) fn undo(&mut self) -> HexResult<()> {
        if let Some(undo) = self.undo_buffer.pop() {
            undo.undo(self)?;
            self.redo_buffer.push(undo);
        }
        Ok(())
    }

    pub fn redo(&mut self) -> HexResult<()> {
        if let Some(redo) = self.redo_buffer.pop() {
            redo.redo(self)?;
            self.undo_buffer.push(redo);
        }
        Ok(())
    }

    pub(crate) fn find_next(&mut self, needle: &[u8]) -> bool {
        for i in self.cursor.position / 2..self.buffer.as_ref().unwrap().len() {
            if self.buffer.as_ref().unwrap().data[i..].starts_with(needle) {
                self.cursor.position = i * 2;
                return true;
            }
        }
        false
    }

    pub(crate) fn find_previous(&mut self, needle: &[u8]) -> bool {
        for i in (0..self.cursor.position / 2).rev() {
            if self.buffer.as_ref().unwrap().data[i..].starts_with(needle) {
                self.cursor.position = i * 2;
                return true;
            }
        }
        false
    }
}
