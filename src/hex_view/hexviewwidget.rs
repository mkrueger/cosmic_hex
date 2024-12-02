use cosmic::iced_core::{
    keyboard,
    text::{LineHeight, Shaping},
    widget::Tree,
    Widget,
};
use cosmic::{
    iced::{
        self, event,
        keyboard::Key,
        mouse::{self, Cursor},
        touch,
        widget::scrollable,
        Element, Event, Length, Point, Rectangle, Renderer, Size, Vector,
    },
    iced_core::{
        self,
        widget::{operation, tree},
    },
    widget::canvas::{Path, Stroke, Text},
    Theme,
};

use crate::hex_view::EditMode;

use super::{HexView, Message};

pub struct HexViewWidget<'a> {
    pub hex_view: &'a HexView,
}

impl<'a> HexViewWidget<'a> {
    pub fn show(hex_view: &'a HexView) -> Element<'a, Message, Theme, cosmic::iced::Renderer> {
        let scroll_properties: scrollable::Scrollbar = scrollable::Scrollbar::default();
        let id = hex_view.id.clone();
        scrollable(HexViewWidget { hex_view })
            .id(id.into())
            .on_scroll(|_viewport| Message::Redraw)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(scroll_properties))
            .into()
    }
}
impl<'a> Widget<Message, Theme, Renderer> for HexViewWidget<'a> {
    fn size(&self) -> Size<Length> {
        let numbers_in_row = self.hex_view.numbers_in_row();
        let height = if let Some(buffer) = &self.hex_view.buffer {
            let lines = buffer.len() / numbers_in_row + 1;
            lines as f32
        } else {
            1.0
        };
        Size {
            width: Length::Fill,
            height: Length::Fixed(height * self.hex_view.font_measure.height),
        }
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

    fn layout(&self, _tree: &mut Tree, _renderer: &Renderer, limits: &iced_core::layout::Limits) -> iced_core::layout::Node {
        let numbers_in_row = self.hex_view.numbers_in_row();
        let height = if let Some(buffer) = &self.hex_view.buffer {
            let lines = buffer.len() / numbers_in_row + 1;
            lines as f32
        } else {
            1.0
        };

        let size = limits.resolve(Length::Fill, Length::Fixed(height * self.hex_view.font_measure.height), Size::ZERO);
        iced::advanced::layout::Node::new(size)
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout,
        _cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let Some(buffer) = &self.hex_view.buffer else {
            return;
        };
        let bounds = layout.bounds();
        let mut vp = viewport.clone();
        vp.y -= bounds.y;
        vp.x -= bounds.x;

        self.hex_view.viewport.set(vp);

        let geometry = self.hex_view.cache.draw(renderer, viewport.size(), |frame| {
            let rect = Path::rectangle(Point::ORIGIN, viewport.size());
            frame.fill(&rect, self.hex_view.theme.background);

            let numbers_in_row = self.hex_view.numbers_in_row();

            let y = viewport.y - bounds.y;
            let mut line = (y / self.hex_view.font_measure.height.max(16.0)).floor();

            let mut offset = line as usize * numbers_in_row;
            let cell_size = self.hex_view.theme.calc_cell_width(self.hex_view.font_measure);
            let offset_margin_width = self.hex_view.theme.calc_offset_margin_width(self.hex_view.font_measure);

            let last_x = offset_margin_width + (numbers_in_row as f32) * cell_size + self.hex_view.theme.hex_ascii_spacing();
            while offset < buffer.len() {
                let line_y = line * self.hex_view.font_measure.height - y;
                if line_y > viewport.height {
                    break;
                }
                let text = Text {
                    font: self.hex_view.font,
                    size: iced::Pixels(self.hex_view.font_size),
                    color: self.hex_view.theme.offset_number,
                    content: format!("{:08X} ", offset),
                    position: iced::Point::new(0.0, line_y),
                    line_height: LineHeight::Relative(1.0),
                    horizontal_alignment: iced::alignment::Horizontal::Left,
                    vertical_alignment: iced::alignment::Vertical::Top,
                    shaping: Shaping::Advanced,
                };

                frame.fill_text(text);

                for i in 0..numbers_in_row {
                    let o = offset + i;
                    if o >= buffer.len() {
                        break;
                    }
                    let x = i as f32 * cell_size + offset_margin_width;
                    let text = Text {
                        font: self.hex_view.font,
                        size: iced::Pixels(self.hex_view.font_size),
                        color: self.hex_view.theme.hex,
                        content: format!("{:02X} ", buffer.get_byte(o)),
                        position: iced::Point::new(x, line_y),
                        line_height: LineHeight::Relative(1.0),
                        horizontal_alignment: iced::alignment::Horizontal::Left,
                        vertical_alignment: iced::alignment::Vertical::Top,
                        shaping: Shaping::Advanced,
                    };
                    frame.fill_text(text);

                    let x = i as f32 * self.hex_view.font_measure.width + last_x;
                    let ch = buffer.get_byte(o) as char;
                    let text = Text {
                        font: self.hex_view.font,
                        size: iced::Pixels(self.hex_view.font_size),
                        color: self.hex_view.theme.ascii,
                        content: format!("{} ", if char::is_ascii_control(&ch) { '.' } else { ch }),
                        position: iced::Point::new(x, line_y),
                        line_height: LineHeight::Relative(1.0),
                        horizontal_alignment: iced::alignment::Horizontal::Left,
                        vertical_alignment: iced::alignment::Vertical::Top,
                        shaping: Shaping::Advanced,
                    };
                    frame.fill_text(text);
                }
                line += 1.0;
                offset += numbers_in_row;
            }
            let caret_line = self.hex_view.cursor.position / (numbers_in_row * 2);
            let caret_line_offset = self.hex_view.cursor.position % (numbers_in_row * 2);

            let caret_cell = caret_line_offset / 2;

            let y = caret_line as f32 * self.hex_view.font_measure.height - y;
            let mut x = caret_cell as f32 * cell_size + offset_margin_width;
            let c = self.hex_view.theme.caret;
            if self.hex_view.cursor.in_hex == EditMode::Hex {
                if caret_line_offset % 2 != 0 {
                    x += self.hex_view.font_measure.width;
                }
                frame.fill_rectangle(Point::new(x, y), self.hex_view.font_measure, c);
            } else {
                frame.stroke_rectangle(
                    Point::new(x, y),
                    Size::new(self.hex_view.font_measure.width * 2.0, self.hex_view.font_measure.height),
                    Stroke::default().with_color(c),
                );
            }

            let x: f32 = last_x + caret_cell as f32 * self.hex_view.font_measure.width;
            if self.hex_view.cursor.in_hex == EditMode::Hex {
                frame.stroke_rectangle(Point::new(x, y), self.hex_view.font_measure, Stroke::default().with_color(c));
            } else {
                frame.fill_rectangle(Point::new(x, y), self.hex_view.font_measure, c);
            }
        });

        use iced::advanced::Renderer as _;
        renderer.with_translation(Vector::new(bounds.x, viewport.y), |renderer| {
            use iced::advanced::graphics::geometry::Renderer as _;
            renderer.draw_geometry(geometry);
        });
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        _event: iced::Event,
        layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced_core::Clipboard,
        shell: &mut iced_core::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<State>();

        let bounds = layout.bounds();
        match _event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if state.is_focused {
                    match key {
                        Key::Named(keyboard::key::Named::ArrowDown) => {
                            let numbers_in_row = self.hex_view.numbers_in_row() * 2;
                            shell.publish(Message::MoveCaret(self.hex_view.cursor.position + numbers_in_row));
                        }
                        Key::Named(keyboard::key::Named::ArrowUp) => {
                            let numbers_in_row = self.hex_view.numbers_in_row() * 2;
                            shell.publish(Message::MoveCaret(self.hex_view.cursor.position.saturating_sub(numbers_in_row)));
                        }
                        Key::Named(keyboard::key::Named::ArrowLeft) => {
                            shell.publish(Message::MoveCaret(self.hex_view.cursor.position.saturating_sub(1)));
                        }
                        Key::Named(keyboard::key::Named::ArrowRight) => {
                            shell.publish(Message::MoveCaret(self.hex_view.cursor.position + 1));
                        }
                        Key::Named(keyboard::key::Named::Home) => {
                            if modifiers.control() || modifiers.macos_command() {
                                shell.publish(Message::MoveCaret(0));
                            } else {
                                let numbers_in_row = self.hex_view.numbers_in_row() * 2;
                                shell.publish(Message::MoveCaret(
                                    self.hex_view.cursor.position - self.hex_view.cursor.position % numbers_in_row,
                                ));
                            }
                        }
                        Key::Named(keyboard::key::Named::End) => {
                            if modifiers.control() || modifiers.macos_command() {
                                if let Some(buffer) = &self.hex_view.buffer {
                                    shell.publish(Message::MoveCaret(buffer.len().saturating_sub(1) * 2));
                                }
                            } else {
                                let numbers_in_row = self.hex_view.numbers_in_row() * 2;
                                let pos = self.hex_view.cursor.position - self.hex_view.cursor.position % numbers_in_row + numbers_in_row - 2;
                                shell.publish(Message::MoveCaret(pos));
                            }
                        }
                        Key::Named(keyboard::key::Named::Tab) => {
                            shell.publish(Message::SwitchMode);
                        }
                        Key::Named(keyboard::key::Named::PageUp) => {
                            shell.publish(Message::PageUp);
                        }
                        Key::Named(keyboard::key::Named::PageDown) => {
                            shell.publish(Message::PageDown);
                        }
                        Key::Character(ch) => {
                            let str = ch.to_string();
                            if str.len() == 1 {
                                let ch = str.chars().next().unwrap();
                                shell.publish(Message::TypeChar(ch));
                            }
                        }
                        _ => {}
                    }
                }
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if cursor.is_over(bounds) {
                    state.is_focused = true;
                    shell.publish(Message::SetFocus(true));
                    if let Some(mut pos) = cursor.position() {
                        println!("pos: {:?} bounds:{:?}", pos, bounds);

                        pos.x -= bounds.x;
                        pos.y -= bounds.y;

                        shell.publish(Message::Click(pos));
                    }
                } else {
                    state.is_focused = false;

                    shell.publish(Message::SetFocus(false));
                }
            }
            _ => {}
        }
        event::Status::Ignored
    }
}

impl<'a> From<HexViewWidget<'a>> for Element<'a, Message, Theme, iced::Renderer> {
    fn from(widget: HexViewWidget<'a>) -> Self {
        Self::new(widget)
    }
}

pub struct State {
    pub is_focused: bool,
}

impl State {
    pub fn new() -> State {
        State { is_focused: false }
    }
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn focus(&mut self) {
        self.is_focused = true;
    }

    fn unfocus(&mut self) {
        self.is_focused = false;
    }
}
