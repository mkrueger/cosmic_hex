use std::path::PathBuf;

use crate::fl;
use cosmic::{widget::menu, Element};

use super::{Action, AppModel, ContextPage};

fn format_path(path: &PathBuf) -> String {
    let home_dir_opt = dirs::home_dir();
    if let Some(home_dir) = &home_dir_opt {
        if let Ok(part) = path.strip_prefix(home_dir) {
            return format!("~/{}", part.display());
        }
    }
    path.display().to_string()
}

impl AppModel {
    pub(crate) fn menu_bar(&self) -> Element<Action> {
        let recent_files = self
            .config_state
            .recent_files
            .iter()
            .enumerate()
            .map(|(i, path)| menu::Item::Button(format_path(path), None, MenuAction::OpenRecentFile(i)))
            .collect::<Vec<_>>();

        menu::bar(vec![
            menu::Tree::with_children(
                menu::root(fl!("file")),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button(fl!("open-file"), None, MenuAction::Open),
                        menu::Item::Folder(fl!("open-recent-file"), recent_files),
                        menu::Item::Button(fl!("close-file"), None, MenuAction::CloseFile),
                        menu::Item::Divider,
                        menu::Item::Button(fl!("save"), None, MenuAction::Save),
                        menu::Item::Button(fl!("save-as"), None, MenuAction::SaveAs),
                        menu::Item::Button(fl!("save-all"), None, MenuAction::SaveAll),
                        menu::Item::Divider,
                        menu::Item::Button(fl!("quit"), None, MenuAction::Quit),
                    ],
                ),
            ),
            menu::Tree::with_children(
                menu::root(fl!("edit")),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button(fl!("undo"), None, MenuAction::Undo),
                        menu::Item::Button(fl!("redo"), None, MenuAction::Redo),
                        menu::Item::Divider,
                        menu::Item::Button(fl!("find"), None, MenuAction::Find),
                    ],
                ),
            ),
            menu::Tree::with_children(
                menu::root(fl!("view")),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button(fl!("menu-settings"), None, MenuAction::ShowSettings),
                        menu::Item::Button(fl!("about"), None, MenuAction::About),
                    ],
                ),
            ),
        ])
        .into()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    Open,
    CloseFile,
    About,
    OpenRecentFile(usize),
    Save,
    SaveAs,
    SaveAll,
    Quit,
    ShowSettings,
    Find,
    Undo,
    Redo,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Action;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::Open => {
                return Action::ChooseOpenFile;
            }
            MenuAction::CloseFile => Action::TabClose(None),
            MenuAction::About => Action::ToggleContextPage(ContextPage::About),
            MenuAction::OpenRecentFile(i) => Action::OpenRecentFile(*i),
            MenuAction::Quit => Action::QuitForce,
            MenuAction::ShowSettings => Action::ToggleContextPage(ContextPage::Settings),
            MenuAction::Find => Action::Find,
            MenuAction::Undo => Action::Undo,
            MenuAction::Redo => Action::Redo,
            MenuAction::Save => Action::Save(None),
            MenuAction::SaveAs => Action::SaveAs,
            MenuAction::SaveAll => Action::SaveAll,
        }
    }
}
