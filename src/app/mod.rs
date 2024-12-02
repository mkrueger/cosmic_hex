// SPDX-License-Identifier: {{LICENSE}}

use crate::config::{AppTheme, Config};
use crate::hex_view::buffer::DataBuffer;
use crate::hex_view::hexviewwidget::HexViewWidget;
use crate::hex_view::Message;
use crate::{fl, SYNTAX_SYSTEM};
use cosmic::app::{context_drawer, Core, Task};
use cosmic::cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{event, keyboard, Alignment, Length, Subscription};
use cosmic::iced_wgpu::graphics::text::font_system;
use cosmic::widget::menu::Action as _;
use cosmic::widget::segmented_button::Entity;
use cosmic::widget::{self, button, column, menu, segmented_button};
use cosmic::{cosmic_theme, style, theme, Application, ApplicationExt, Element};
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::{fs, process};
use tab::Tab;

mod key_binds;
mod menu_bar;
mod tab;

const REPOSITORY: &str = "https://github.com/mkrueger/cosmic-hex";
const APP_ICON: &[u8] = include_bytes!("../../res/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    core: Core,
    context_page: ContextPage,
    tab_model: segmented_button::SingleSelectModel,
    dialog_page_opt: Option<DialogPage>,
    key_binds: HashMap<menu::KeyBind, menu_bar::MenuAction>,
    config_handler: Option<cosmic_config::Config>,
    config: Config,
    config_state_handler: Option<cosmic_config::Config>,
    config_state: ConfigState,

    find_search_id: widget::Id,
    find: bool,
    search_pattern: String,
    needle: Vec<u8>,

    modifiers: keyboard::Modifiers,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DialogPage {
    PromptSaveClose(segmented_button::Entity),
    PromptSaveQuit(Vec<segmented_button::Entity>),
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Action {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    ChooseOpenFile,
    OpenFile(PathBuf),
    OpenRecentFile(usize),

    QuitForce,
    TabActivate(Entity),
    TabClose(Option<Entity>),
    HexAction(Message),
    PromptSaveChanges(Entity),
    TabCloseForce(Entity),
    Save(Option<Entity>),
    DialogCancel,
    SaveAll,

    ChangeTheme(AppTheme),
    ChangeSyntaxTheme(usize, bool),
    ChangeFont(usize),
    ChangeFontSize(usize),

    Find,
    Undo,
    Redo,
    SearchPatternChanged(String),
    FindNext,
    FindPrevious,
    SaveAs,

    KeyPressed(keyboard::Modifiers, keyboard::Key),
    ModifiersChanged(keyboard::Modifiers),
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Action;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.CosmicHex";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let (config_handler, config) = match cosmic_config::Config::new(AppModel::APP_ID, crate::config::CONFIG_VERSION) {
            Ok(config_handler) => {
                let config = Config::get_entry(&config_handler).unwrap_or_else(|(errs, config)| {
                    log::info!("errors loading config: {:?}", errs);
                    config
                });
                (Some(config_handler), config)
            }
            Err(err) => {
                log::error!("failed to create config handler: {}", err);
                (None, Config::default())
            }
        };

        let (config_state_handler, config_state) = match cosmic_config::Config::new_state(AppModel::APP_ID, crate::config::CONFIG_VERSION) {
            Ok(config_state_handler) => {
                let config_state = ConfigState::get_entry(&config_state_handler).unwrap_or_else(|(errs, config_state)| {
                    log::info!("errors loading config_state: {:?}", errs);
                    config_state
                });
                (Some(config_state_handler), config_state)
            }
            Err(err) => {
                log::error!("failed to create config_state handler: {}", err);
                (None, ConfigState::default())
            }
        };

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            tab_model: segmented_button::Model::builder().build(),
            key_binds: key_binds::get_key_binds(),
            dialog_page_opt: None,
            // Optional configuration file for an application.
            config_handler,
            config,
            config_state_handler,
            config_state,
            find: false,
            search_pattern: String::new(),
            find_search_id: widget::Id::unique(),
            needle: Vec::new(),

            modifiers: keyboard::Modifiers::default(),
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        vec![self.menu_bar()]
    }

    fn nav_model(&self) -> Option<&widget::nav_bar::Model> {
        None
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }
        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(self.about(), Action::ToggleContextPage(ContextPage::About)).title(fl!("about")),
            ContextPage::Settings => context_drawer::context_drawer(self.settings(), Action::ToggleContextPage(ContextPage::Settings)).title(fl!("settings")),
        })
    }

    fn dialog(&self) -> Option<Element<Self::Message>> {
        let Some(ref dialog) = self.dialog_page_opt else {
            return None;
        };
        match dialog {
            DialogPage::PromptSaveClose(entity) => {
                let save_button = widget::button::suggested(fl!("save")).on_press(Action::Save(Some(*entity)));
                let discard_button = widget::button::destructive(fl!("discard")).on_press(Action::TabCloseForce(*entity));
                let cancel_button = widget::button::text(fl!("cancel")).on_press(Action::DialogCancel);
                let dialog = widget::dialog::Dialog::new()
                    .title(fl!("prompt-save-changes-title"))
                    .body(fl!("prompt-unsaved-changes"))
                    .icon(widget::icon::from_name("dialog-warning-symbolic").size(64))
                    .primary_action(save_button)
                    .secondary_action(discard_button)
                    .tertiary_action(cancel_button);
                Some(dialog.into())
            }

            DialogPage::PromptSaveQuit(entities) => {
                let can_save_all = true;
                let cosmic_theme::Spacing { space_xxs, .. } = self.core().system_theme().cosmic().spacing;
                let mut column = widget::column::with_capacity(entities.len()).spacing(space_xxs);
                for entity in entities.iter() {
                    if let Some(Tab::Editor(tab)) = self.tab_model.data::<Tab>(*entity) {
                        let mut row = widget::row::with_capacity(3).align_y(Alignment::Center);
                        row = row.push(widget::text(tab.title()));
                        row = row.push(widget::horizontal_space());
                        row = row.push(widget::button::standard(fl!("save")).on_press(Action::Save(Some(*entity))));
                        column = column.push(row);
                    }
                }

                let mut save_button = widget::button::suggested(fl!("save-all"));
                if can_save_all {
                    save_button = save_button.on_press(Action::SaveAll);
                }
                let discard_button = widget::button::destructive(fl!("discard")).on_press(Action::QuitForce);
                let cancel_button = widget::button::text(fl!("cancel")).on_press(Action::DialogCancel);
                let dialog = widget::dialog::Dialog::new()
                    .title(fl!("prompt-save-changes-title"))
                    .body(fl!("prompt-unsaved-changes"))
                    .icon(widget::icon::from_name("dialog-warning-symbolic").size(64))
                    .control(column)
                    .primary_action(save_button)
                    .secondary_action(discard_button)
                    .tertiary_action(cancel_button);
                Some(dialog.into())
            }
        }
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let cosmic_theme::Spacing { space_none, space_xxs, .. } = self.core().system_theme().cosmic().spacing;

        let mut tab_column = widget::column::with_capacity(3).padding([space_none, space_xxs]);

        tab_column = tab_column.push(
            widget::row::with_capacity(2).align_y(Alignment::Center).push(
                widget::tab_bar::horizontal(&self.tab_model)
                    .button_height(32)
                    .button_spacing(space_xxs)
                    .close_icon(widget::icon::from_name("window-close-symbolic").size(16).handle().icon())
                    //TODO: this causes issues with small window sizes .minimum_button_width(240)
                    .on_activate(|entity| Action::TabActivate(entity))
                    .on_close(|entity| Action::TabClose(Some(entity)))
                    .width(Length::Shrink),
            ),
        );

        let tab_id = self.tab_model.active();
        match self.tab_model.data::<Tab>(tab_id) {
            Some(Tab::Editor(tab)) => {
                //tab_column = tab_column.push(tab.hex_view.view());
                let widget = HexViewWidget::show(&tab.hex_view);
                let find_widget = widget.map(|msg| Action::HexAction(msg));

                let data_u32 = if let Some(buffer) = tab.hex_view.buffer.as_ref() {
                    buffer.get_u32(tab.hex_view.cursor.position)
                } else {
                    0
                };

                tab_column = tab_column.push(column::with_children(vec![
                    widget::row::with_children(vec![find_widget]).height(Length::Fill).into(),
                    widget::row::with_children(vec![
                        widget::text::body("Offset:").into(),
                        widget::text::body(format!("{:08X}", tab.hex_view.cursor.position)).into(),
                        widget::text::body("\t").into(),
                        widget::text::body("uint:").into(),
                        widget::text::body(format!("{}", data_u32)).into(),
                    ])
                    .height(Length::Shrink)
                    .into(),
                ]));
            }
            _ => {}
        }

        if self.find {
            let find_input = widget::text_input::text_input(fl!("find-placeholder"), &self.search_pattern)
                .id(self.find_search_id.clone())
                .on_input(Action::SearchPatternChanged)
                .on_submit(if self.modifiers.contains(keyboard::Modifiers::SHIFT) {
                    Action::FindPrevious
                } else {
                    Action::FindNext
                })
                .width(Length::Fixed(320.0))
                .trailing_icon(
                    button::custom(widget::icon::from_name("edit-clear-symbolic").size(16).handle().icon())
                        .on_press(Action::SearchPatternChanged(String::new()))
                        .class(style::Button::Icon)
                        .into(),
                );
            let find_widget = widget::row::with_children(vec![
                find_input.into(),
                widget::tooltip(
                    button::custom(widget::icon::from_name("go-up-symbolic").size(16).handle().icon())
                        .on_press(Action::FindPrevious)
                        .padding(space_xxs)
                        .class(style::Button::Icon),
                    widget::text::body(fl!("find-previous")),
                    widget::tooltip::Position::Top,
                )
                .into(),
                widget::tooltip(
                    button::custom(widget::icon::from_name("go-down-symbolic").size(16).handle().icon())
                        .on_press(Action::FindNext)
                        .padding(space_xxs)
                        .class(style::Button::Icon),
                    widget::text::body(fl!("find-next")),
                    widget::tooltip::Position::Top,
                )
                .into(),
                widget::horizontal_space().into(),
                button::custom(widget::icon::from_name("window-close-symbolic").size(16).handle().icon())
                    .on_press(Action::Find)
                    .padding(space_xxs)
                    .class(style::Button::Icon)
                    .into(),
            ])
            .align_y(Alignment::Center)
            .padding(space_xxs)
            .spacing(space_xxs);

            let column = widget::column::with_capacity(3).push(find_widget);
            tab_column = tab_column.push(widget::layer_container(column).layer(cosmic_theme::Layer::Primary));
        }

        let content = tab_column.into();

        // Uncomment to debug layout:
        //content.explain(cosmic::iced::Color::WHITE)
        content
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            event::listen_with(|event, status, _window_id| match event {
                event::Event::Keyboard(keyboard::Event::KeyPressed { modifiers, key, .. }) => match status {
                    event::Status::Ignored => Some(Action::KeyPressed(modifiers, key)),
                    event::Status::Captured => None,
                },
                event::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => Some(Action::ModifiersChanged(modifiers)),
                _ => None,
            }),
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Action::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Action::UpdateConfig(update.config)),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Action::OpenFile(path) => {
                self.open_tab(path);
            }

            Action::ChooseOpenFile => {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    return self.update(Action::OpenFile(path));
                }
            }
            Action::OpenRecentFile(i) => {
                if let Some(path) = self.config_state.recent_files.get(i).cloned() {
                    return self.update(Action::OpenFile(path));
                }
            }

            Action::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }

            Action::SubscriptionChannel => {
                // For example purposes only.
            }

            Action::ToggleContextPage(context_page) => {
                if self.context_page == context_page && self.core.window.show_context {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }

                //self.set_context_title(context_page.title());
            }

            Action::UpdateConfig(config) => {
                self.config = config;
            }

            Action::QuitForce => {
                process::exit(0);
            }

            Action::TabActivate(entity) => {
                self.tab_model.activate(entity);
                return self.update_tab();
            }

            Action::TabClose(entity_opt) => {
                let entity = entity_opt.unwrap_or(self.tab_model.active());
                let tab = self.tab_model.data_mut::<Tab>(entity);
                if let Some(tab) = tab {
                    if tab.is_dirty() {
                        return Task::batch([self.update(Action::TabActivate(entity)), self.update(Action::PromptSaveChanges(entity))]);
                    } else {
                        return self.update(Action::TabCloseForce(entity));
                    }
                }
            }
            Action::TabCloseForce(entity) => {
                if let Some(position) = self.tab_model.position(entity) {
                    if position > 0 {
                        self.tab_model.activate_position(position - 1);
                    } else {
                        self.tab_model.activate_position(position + 1);
                    }
                }
                self.tab_model.remove(entity);
                return self.update_tab();
            }

            Action::PromptSaveChanges(entity) => {
                self.dialog_page_opt = Some(DialogPage::PromptSaveClose(entity));
            }

            Action::HexAction(msg) => {
                let tab_id = self.tab_model.active();
                match self.tab_model.data_mut::<Tab>(tab_id) {
                    Some(Tab::Editor(tab)) => {
                        return tab.hex_view.update(msg).map(|t| cosmic::app::Message::App(Action::HexAction(t)));
                    }
                    _ => {}
                }
            }

            Action::Save(entity_opt) => {
                let tab_id = entity_opt.unwrap_or(self.tab_model.active());
                match self.tab_model.data_mut::<Tab>(tab_id) {
                    Some(Tab::Editor(tab)) => {
                        if let Err(err) = tab.hex_view.save() {
                            log::error!("failed to save tab: {}", err);
                        }
                    }
                    _ => {}
                }
            }

            Action::SaveAs => {
                let tab_id = self.tab_model.active();
                match self.tab_model.data_mut::<Tab>(tab_id) {
                    Some(Tab::Editor(tab)) => {
                        if let Some(file) = rfd::FileDialog::new().save_file() {
                            tab.hex_view.path = file.clone();
                            if let Err(err) = tab.hex_view.save() {
                                log::error!("failed to save tab: {}", err);
                            }
                        }
                    }
                    _ => {}
                }
            }

            Action::SaveAll => {
                let entities: Vec<_> = self.tab_model.iter().collect();
                for entity in entities {
                    if let Some(Tab::Editor(tab)) = self.tab_model.data_mut::<Tab>(entity) {
                        if tab.hex_view.is_dirty() {
                            let _ = tab.hex_view.save();
                        }
                    }
                }
            }

            Action::DialogCancel => {
                self.dialog_page_opt = None;
            }

            Action::ChangeTheme(app_theme) => {
                self.config.app_theme = app_theme;
                return self.save_config();
            }

            Action::ChangeFont(index) => {
                match font_names.get(index) {
                    Some(font_name) => {
                        if font_name != &self.config.font_name {
                            // Update font name from config
                            {
                                let mut font_system = font_system().write().unwrap();
                                font_system.raw().db_mut().set_monospace_family(font_name);
                            }
                            self.config.font_name = font_name.to_string();
                            return self.save_config();
                        }
                    }
                    None => {
                        log::warn!("failed to find font with index {}", index);
                    }
                }
            }

            Action::ChangeFontSize(font_size) => {
                self.config.font_size = font_size;
                return self.save_config();
            }

            Action::ChangeSyntaxTheme(index, dark) => match theme_names.get(index) {
                Some(theme_name) => {
                    if dark {
                        self.config.syntax_theme_dark = theme_name.to_string();
                    } else {
                        self.config.syntax_theme_light = theme_name.to_string();
                    }
                    return self.save_config();
                }
                None => {
                    log::warn!("failed to find syntax theme with index {}", index);
                }
            },

            Action::Find => {
                self.find = !self.find;
            }

            Action::Undo => {
                let tab_id = self.tab_model.active();
                match self.tab_model.data_mut::<Tab>(tab_id) {
                    Some(Tab::Editor(tab)) => {
                        let _ = tab.hex_view.undo();
                        return self.update_tab();
                    }
                    _ => {}
                }
            }

            Action::Redo => {
                let tab_id = self.tab_model.active();
                match self.tab_model.data_mut::<Tab>(tab_id) {
                    Some(Tab::Editor(tab)) => {
                        let _ = tab.hex_view.redo();
                        return self.update_tab();
                    }
                    _ => {}
                }
            }

            Action::SearchPatternChanged(value) => {
                self.search_pattern = value;
                self.needle = self.get_pattern_needle();
            }

            Action::FindNext => {
                let tab_id = self.tab_model.active();

                match self.tab_model.data_mut::<Tab>(tab_id) {
                    Some(Tab::Editor(tab)) => {
                        tab.hex_view.find_next(&self.needle);
                        return self.update_tab();
                    }
                    _ => {}
                }
            }

            Action::FindPrevious => {
                let tab_id = self.tab_model.active();
                match self.tab_model.data_mut::<Tab>(tab_id) {
                    Some(Tab::Editor(tab)) => {
                        tab.hex_view.find_previous(&self.needle);
                        return self.update_tab();
                    }
                    _ => {}
                }
            }

            Action::KeyPressed(modifiers, key) => {
                for (key_bind, action) in self.key_binds.iter() {
                    if key_bind.matches(modifiers, &key) {
                        return self.update(action.message());
                    }
                }
            }

            Action::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
            }
        }
        Task::none()
    }

    fn on_nav_select(&mut self, _id: widget::nav_bar::Id) -> Task<Self::Message> {
        Task::none()
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<Action> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;
        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));
        let title = widget::text::title3(fl!("app-title"));
        let link = widget::button::link(REPOSITORY).on_press(Action::OpenRepositoryUrl).padding(0);
        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<Action> {
        let window_title = fl!("app-title");
        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }

    fn open_tab(&mut self, path: std::path::PathBuf) -> Option<segmented_button::Entity> {
        let canonical = match fs::canonicalize(&path) {
            Ok(path) => path,
            Err(err) => {
                log::error!("failed to canonicalize {:?}: {}", path, err);
                return None;
            }
        };

        let mut activate_opt = None;
        for entity in self.tab_model.iter() {
            if let Some(Tab::Editor(tab)) = self.tab_model.data::<Tab>(entity) {
                if tab.hex_view.path == canonical {
                    activate_opt = Some(entity);
                    break;
                }
            }
        }
        if let Some(entity) = activate_opt {
            self.tab_model.activate(entity);
            return Some(entity);
        }

        let buf = DataBuffer {
            data: match fs::read(&canonical) {
                Ok(data) => data,
                Err(err) => {
                    log::error!("failed to read {:?}: {}", canonical, err);
                    return None;
                }
            },
        };

        self.config_state.recent_files.retain(|x| x != &canonical);
        self.config_state.recent_files.push_front(canonical.to_path_buf());
        self.config_state.recent_files.truncate(10);
        self.save_config_state();

        let mut tab = tab::EditorTab::new(canonical, buf);
        tab.set_config(&self.config);
        Some(
            self.tab_model
                .insert()
                .text(tab.title())
                .icon(tab.icon(16))
                .data::<Tab>(Tab::Editor(tab))
                .closable()
                .activate()
                .id(),
        )
    }

    fn update_tab(&mut self) -> cosmic::Task<cosmic::app::Message<Action>> {
        let tab_id = self.tab_model.active();
        match self.tab_model.data_mut::<Tab>(tab_id) {
            Some(Tab::Editor(tab)) => {
                tab.hex_view.redraw();
            }
            _ => {}
        }
        Task::none()
    }

    fn settings(&self) -> Element<Action> {
        let app_theme_selected = match self.config.app_theme {
            AppTheme::Dark => 1,
            AppTheme::Light => 2,
            AppTheme::System => 0,
        };
        let dark_selected = theme_names.iter().position(|theme_name| theme_name == &self.config.syntax_theme_dark);
        let light_selected = theme_names.iter().position(|theme_name| theme_name == &self.config.syntax_theme_light);
        let font_selected = {
            let mut font_system = font_system().write().unwrap();
            let current_font_name = font_system.raw().db().family_name(&cosmic_text::fontdb::Family::Monospace);
            font_names.iter().position(|font_name| font_name == current_font_name)
        };

        let font_size_selected = font_sizes.iter().position(|font_size| font_size == &self.config.font_size);

        widget::settings::view_column(vec![widget::settings::section()
            .title(fl!("appearance"))
            .add(
                widget::settings::item::builder(fl!("theme")).control(widget::dropdown(&app_themes, Some(app_theme_selected), move |index| {
                    Action::ChangeTheme(match index {
                        1 => AppTheme::Dark,
                        2 => AppTheme::Light,
                        _ => AppTheme::System,
                    })
                })),
            )
            .add(
                widget::settings::item::builder(fl!("syntax-dark")).control(widget::dropdown(&theme_names, dark_selected, move |index| {
                    Action::ChangeSyntaxTheme(index, true)
                })),
            )
            .add(
                widget::settings::item::builder(fl!("syntax-light")).control(widget::dropdown(&theme_names, light_selected, move |index| {
                    Action::ChangeSyntaxTheme(index, false)
                })),
            )
            .add(widget::settings::item::builder(fl!("default-font")).control(widget::dropdown(&font_names, font_selected, Action::ChangeFont)))
            .add(
                widget::settings::item::builder(fl!("default-font-size")).control(widget::dropdown(&font_size_names, font_size_selected, move |index| {
                    Action::ChangeFontSize(font_sizes[index])
                })),
            )
            .into()])
        .into()
    }

    fn save_config(&mut self) -> Task<Action> {
        if let Some(ref config_handler) = self.config_handler {
            if let Err(err) = self.config.write_entry(config_handler) {
                log::error!("failed to save config: {}", err);
            }
        }
        self.update_config()
    }

    fn update_config(&mut self) -> Task<Action> {
        let entities: Vec<_> = self.tab_model.iter().collect();
        for entity in entities {
            if let Some(Tab::Editor(tab)) = self.tab_model.data_mut::<Tab>(entity) {
                tab.set_config(&self.config);
            }
        }
        cosmic::app::command::set_theme(self.config.app_theme.theme())
    }
    fn save_config_state(&mut self) {
        if let Some(ref config_state_handler) = self.config_state_handler {
            if let Err(err) = self.config_state.write_entry(config_state_handler) {
                log::error!("failed to save config_state: {}", err);
            }
        }
    }

    fn get_pattern_needle(&self) -> Vec<u8> {
        let mut res = Vec::new();

        for (i, c) in self.search_pattern.chars().enumerate() {
            let d = c.to_digit(16);
            if let Some(d) = d {
                if i % 2 == 0 {
                    res.push((d as u8) << 4);
                } else {
                    let a = res.pop().unwrap();
                    res.push(a | (d as u8));
                }
            }
        }

        res
    }
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
    Settings,
}

impl ContextPage {
    fn title(&self) -> String {
        match self {
            Self::About => String::new(),
            Self::Settings => fl!("settings"),
        }
    }
}

lazy_static::lazy_static! {
    static ref font_size_names: Vec<String> = (4..=32).map(|font_size| format!("{}px", font_size)).collect();
    static ref font_sizes: Vec<usize> = (4..=32).collect();
    static ref app_themes: Vec<String> = vec![fl!("match-desktop"), fl!("dark"), fl!("light")];
    static ref theme_names: Vec<String> = SYNTAX_SYSTEM.get().unwrap().theme_set.themes.iter().map(|(theme_name, _theme)| theme_name.to_string()).collect();
    static ref font_names: Vec<String> = {
        let mut res = Vec::new();
        let mut font_system = font_system().write().unwrap();
        let attrs = cosmic_text::Attrs::new().family(cosmic_text::fontdb::Family::Monospace);
        for face in font_system.raw().db().faces() {
            if attrs.matches(face) && face.monospaced {
                let font_name = face
                    .families
                    .first()
                    .map_or_else(|| face.post_script_name.to_string(), |x| x.0.to_string());
                if !res.contains(&font_name) {
                    res.push(font_name);
                }
            }
        }
        res.sort();
        res
    };
}

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConfigState {
    pub recent_files: VecDeque<PathBuf>,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self { recent_files: VecDeque::new() }
    }
}
