use cosmic::{
    iced::keyboard::Key,
    widget::menu::{key_bind::Modifier, KeyBind},
};
use std::collections::HashMap;

use super::menu_bar::MenuAction;

fn bind_key(key: char) -> KeyBind {
    KeyBind {
        key: Key::Character(key.to_string().into()),
        modifiers: vec![Modifier::Ctrl],
    }
}

fn bind_key_ctrl_shift(key: char) -> KeyBind {
    KeyBind {
        key: Key::Character(key.to_string().into()),
        modifiers: vec![Modifier::Ctrl, Modifier::Shift],
    }
}

pub fn get_key_binds() -> HashMap<KeyBind, MenuAction> {
    HashMap::from([
        // File
        (bind_key('o'), MenuAction::Open),
        (bind_key('q'), MenuAction::Quit),
        (bind_key('s'), MenuAction::Save),
        (bind_key('w'), MenuAction::CloseFile),
        // Edit
        (bind_key('z'), MenuAction::Undo),
        (bind_key_ctrl_shift('z'), MenuAction::Redo),
        (bind_key('f'), MenuAction::Find),
    ])
}
