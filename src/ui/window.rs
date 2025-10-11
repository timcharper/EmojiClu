use crate::destroyable::Destroyable;
use crate::events::{Channel, EventEmitter, EventHandler, EventObserver};
use crate::game::game_engine::GameEngine;
use crate::game::settings::Settings;
use crate::game::stats_manager::StatsManager;
use crate::model::{
    game_state_snapshot, Difficulty, GameEngineCommand, GameEngineEvent, GameStateSnapshot,
    InputEvent, LayoutManagerEvent, SettingsProjection,
};
use crate::ui::input_translator::InputTranslator;
use crate::ui::seed_dialog::SeedDialog;
use crate::ui::settings_menu_ui::SettingsMenuUI;
use crate::ui::stats_dialog::StatsDialog;
use crate::ui::submit_ui::SubmitUI;
use crate::ui::timer_button_ui::TimerButtonUI;
use crate::ui::top_level_input_event_monitor::TopLevelInputEventMonitor;
use fluent_i18n::t;
use gio::{Menu, SimpleAction};
use gtk4::gdk::{Display, Monitor};
use gtk4::{
    prelude::*, AboutDialog, Application, ApplicationWindow, Button, CssProvider, HeaderBar, Label,
    License, MenuButton, Orientation, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use std::cell::RefCell;
use std::env;
use std::rc::Rc;

use super::clue_panels_ui::CluePanelsUI;
use super::game_info_ui::GameInfoUI;
use super::hint_button_ui::HintButtonUI;
use super::history_controls_ui::HistoryControlsUI;
use super::layout_manager::{ClueStats, LayoutManager};
use super::pause_screen_ui::PauseScreenUI;
use super::puzzle_generation_dialog::PuzzleGenerationDialog;
use super::puzzle_grid_ui::PuzzleGridUI;
use super::resource_manager::ResourceManager;
use super::tutorial_ui::TutorialUI;

const APP_VERSION: &str = env!("APP_VERSION");

pub fn load_settings_and_game_state() -> (Settings, Option<GameStateSnapshot>) {
    let mut initial_settings = Settings::load();
    let saved_game_state = game_state_snapshot::load_game_state_snapshot();
    if let Some(save_state) = &saved_game_state {
        log::info!(target: "window", "Loaded saved game state");
        // if these disagree, then bad things happen.
        initial_settings.difficulty = save_state.board.solution.difficulty;
    } else {
        log::info!(target: "window", "No saved game state found");
    }
    (initial_settings, saved_game_state)
}

struct ChannelPair<T: std::fmt::Debug + 'static> {
    emitter: EventEmitter<T>,
    observer: EventObserver<T>,
}
impl<T: std::fmt::Debug + 'static> ChannelPair<T> {
    fn new() -> Self {
        let (emitter, observer) = Channel::<T>::new();
        Self { emitter, observer }
    }
}

struct Channels {
    game_engine_command: ChannelPair<GameEngineCommand>,
    game_engine_event: ChannelPair<GameEngineEvent>,
    layout_manager: ChannelPair<LayoutManagerEvent>,
    input: ChannelPair<InputEvent>,
}

impl Channels {
    fn new() -> Self {
        Self {
            game_engine_command: ChannelPair::new(),
            game_engine_event: ChannelPair::new(),
            layout_manager: ChannelPair::new(),
            input: ChannelPair::new(),
        }
    }
}

struct Components {
    clue_panels_ui: Rc<RefCell<CluePanelsUI>>,
    resource_manager: Rc<RefCell<ResourceManager>>,
    puzzle_grid_ui: Rc<RefCell<PuzzleGridUI>>,
    game_state: Rc<RefCell<GameEngine>>,
    hint_button_ui: Rc<RefCell<HintButtonUI>>,
    tutorial_ui: Rc<RefCell<TutorialUI>>,
    layout_manager: Rc<RefCell<LayoutManager>>,
    pause_screen_ui: Rc<RefCell<PauseScreenUI>>,
    settings_menu_ui: Rc<RefCell<SettingsMenuUI>>,
    game_info_ui: Rc<RefCell<GameInfoUI>>,
    game_controls: Rc<RefCell<TopLevelInputEventMonitor>>,
    history_controls_ui: Rc<RefCell<HistoryControlsUI>>,
    stats_manager: Rc<RefCell<StatsManager>>,
    submit_ui: Rc<RefCell<SubmitUI>>,
    input_translator: Rc<RefCell<InputTranslator>>,
    timer_button: Rc<RefCell<TimerButtonUI>>,
    seed_dialog: Rc<RefCell<SeedDialog>>,
    puzzle_generation_dialog: Rc<RefCell<PuzzleGenerationDialog>>,
    settings_projection: Rc<RefCell<SettingsProjection>>,
}

impl Components {
    fn new(
        window: Rc<ApplicationWindow>,
        channels: &Channels,
        initial_settings: &Settings,
    ) -> Self {
        let resource_manager = ResourceManager::new(channels.layout_manager.emitter.clone());
        let default_layout = LayoutManager::calculate_layout(
            initial_settings.difficulty,
            Some(ClueStats::default()),
        );
        let image_set = resource_manager.borrow().get_image_set();
        let audio_set = resource_manager.borrow().get_audio_set();
        let clue_panels_ui = CluePanelsUI::new(
            window.clone(),
            channels.input.emitter.clone(),
            &image_set,
            default_layout.clone(),
            initial_settings,
        );
        // Create puzzle grid and clue set UI first
        let puzzle_grid_ui = PuzzleGridUI::new(
            channels.input.emitter.clone(),
            image_set.clone(),
            default_layout.clone(),
            initial_settings,
        );

        // Create game state with UI references
        let game_state = GameEngine::new(
            channels.game_engine_event.emitter.clone(),
            initial_settings.clone(),
        );

        // Create hint button UI
        let hint_button_ui = HintButtonUI::new(
            channels.game_engine_command.emitter.clone(),
            &game_state,
            &audio_set,
            &window,
        );

        // Instantiate TutorialUI
        let tutorial_ui = TutorialUI::new(
            channels.game_engine_command.emitter.clone(),
            &window,
            &image_set,
            initial_settings,
            &default_layout,
        );

        let layout_manager = LayoutManager::new(
            window.clone(),
            channels.layout_manager.emitter.clone(),
            initial_settings.difficulty,
        );

        // Create pause screen UI
        let pause_screen_ui = PauseScreenUI::new();

        // Create Settings submenu
        let settings_menu_ui = SettingsMenuUI::new(
            window.clone(),
            channels.game_engine_command.emitter.clone(),
            initial_settings.clone(),
        );
        let game_info_ui =
            GameInfoUI::new(Rc::new(pause_screen_ui.borrow().pause_screen_box.clone()));
        // Initialize game controls
        let game_controls = TopLevelInputEventMonitor::new(
            window.clone(),
            layout_manager.borrow().scrolled_window.clone(),
            channels.input.emitter.clone(),
        );
        let history_controls_ui = HistoryControlsUI::new();

        // Remove the old button_box since controls are now in header
        let stats_manager = Rc::new(RefCell::new(StatsManager::new()));

        let submit_ui = SubmitUI::new(
            channels.game_engine_command.emitter.clone(),
            &stats_manager,
            &audio_set,
            &window,
        );
        let settings_projection = SettingsProjection::new(&initial_settings);

        // Initialize input translator
        let input_translator = InputTranslator::new(
            channels.game_engine_command.emitter.clone(),
            settings_projection.clone(),
        );
        let timer_button =
            TimerButtonUI::new(&window, channels.game_engine_command.emitter.clone());

        let seed_dialog = SeedDialog::new(&window, channels.game_engine_command.emitter.clone());
        let puzzle_generation_dialog = PuzzleGenerationDialog::new(&window);

        Self {
            clue_panels_ui,
            resource_manager,
            puzzle_grid_ui,
            game_state,
            hint_button_ui,
            tutorial_ui,
            layout_manager,
            pause_screen_ui,
            settings_menu_ui,
            game_info_ui,
            game_controls,
            history_controls_ui,
            stats_manager,
            submit_ui,
            input_translator,
            timer_button,
            seed_dialog,
            puzzle_generation_dialog,
            settings_projection,
        }
    }
}

impl Destroyable for Components {
    fn destroy(&mut self) {
        self.history_controls_ui.borrow_mut().destroy();
        self.game_state.borrow_mut().destroy();
        self.game_info_ui.borrow_mut().destroy();
        self.hint_button_ui.borrow_mut().destroy();
        self.pause_screen_ui.borrow_mut().destroy();
        self.submit_ui.borrow_mut().destroy();
        self.puzzle_grid_ui.borrow_mut().destroy();
        self.clue_panels_ui.borrow_mut().destroy();
        self.timer_button.borrow_mut().destroy();
        self.layout_manager.borrow_mut().destroy();
        self.seed_dialog.borrow_mut().destroy();
        self.puzzle_generation_dialog.borrow_mut().destroy();
        self.settings_menu_ui.borrow_mut().destroy();
        self.game_controls.borrow_mut().destroy();
        self.input_translator.borrow_mut().destroy();
        self.resource_manager.borrow_mut().destroy();
    }
}

fn wire_event_observers(channels: &Channels, components: &Components) {
    type EHGameEvent = Rc<RefCell<dyn EventHandler<GameEngineEvent>>>;
    type EHLayoutEvent = Rc<RefCell<dyn EventHandler<LayoutManagerEvent>>>;
    type EHGameCommand = Rc<RefCell<dyn EventHandler<GameEngineCommand>>>;

    let game_engine_event_observer = &channels.game_engine_event.observer;
    let layout_event_observer = &channels.layout_manager.observer;
    let game_engine_command_observer = &channels.game_engine_command.observer;

    // Subscribe GameEngine to its own commands
    game_engine_command_observer
        .subscribe_component(&(components.game_state.clone() as EHGameCommand));

    game_engine_event_observer
        .subscribe_component(&(components.clue_panels_ui.clone() as EHGameEvent));
    layout_event_observer
        .subscribe_component(&(components.clue_panels_ui.clone() as EHLayoutEvent));

    game_engine_event_observer
        .subscribe_component(&(components.puzzle_grid_ui.clone() as EHGameEvent));
    layout_event_observer
        .subscribe_component(&(components.puzzle_grid_ui.clone() as EHLayoutEvent));

    game_engine_event_observer
        .subscribe_component(&(components.tutorial_ui.clone() as EHGameEvent));
    layout_event_observer.subscribe_component(&(components.tutorial_ui.clone() as EHLayoutEvent));

    // Subscribe layout manager to GameEngineEvent
    game_engine_event_observer
        .subscribe_component(&(components.layout_manager.clone() as EHGameEvent));

    // Subscribe GameInfoUI (uses EventHandler<GameEngineEvent>) via centralized subscription
    game_engine_event_observer
        .subscribe_component(&(components.game_info_ui.clone() as EHGameEvent));

    // Subscribe PauseScreenUI to GameEngineEvent
    game_engine_event_observer
        .subscribe_component(&(components.pause_screen_ui.clone() as EHGameEvent));

    // Subscribe HistoryControlsUI to GameEngineEvent
    game_engine_event_observer
        .subscribe_component(&(components.history_controls_ui.clone() as EHGameEvent));

    layout_event_observer
        .subscribe_component(&(components.resource_manager.clone() as EHLayoutEvent));

    // Subscribe SubmitUI to GameEngineEvent via centralized subscription
    game_engine_event_observer.subscribe_component(&(components.submit_ui.clone() as EHGameEvent));

    // New centralized subscriptions for components refactored to EventHandler
    game_engine_event_observer
        .subscribe_component(&(components.seed_dialog.clone() as EHGameEvent));
    game_engine_event_observer
        .subscribe_component(&(components.puzzle_generation_dialog.clone() as EHGameEvent));

    // InputTranslator handles InputEvent
    type EHInputEvent = Rc<RefCell<dyn EventHandler<InputEvent>>>;
    let input_event_observer = &channels.input.observer;
    input_event_observer
        .subscribe_component(&(components.input_translator.clone() as EHInputEvent));

    // SettingsProjection listens for GameEngineEvent (SettingsChanged)
    game_engine_event_observer
        .subscribe_component(&(components.settings_projection.clone() as EHGameEvent));
}

pub fn build_ui(app: &Application) {
    let (initial_settings, saved_game_state) = load_settings_and_game_state();

    let display = Display::default().expect("Could not connect to a display.");
    let monitor = display
        .monitors()
        .item(0)
        .and_then(|m| m.downcast::<Monitor>().ok())
        .expect("No monitors found");
    let monitor_geometry = monitor.geometry();
    let monitor_width = monitor_geometry.width();
    let monitor_height = monitor_geometry.height();
    let desired_height = (monitor_height * 8) / 10;
    let desired_width = (monitor_height * 4) / 3;
    let max_desired_width = (monitor_width * 8) / 10;

    let window = Rc::new(
        ApplicationWindow::builder()
            .application(app)
            .title(&t!("app-title"))
            .icon_name("io.github.timcharper.EmojiClu")
            .resizable(true)
            .decorated(true)
            .default_height(desired_height as i32)
            .default_width(desired_width.min(max_desired_width) as i32)
            .build(),
    );
    let channels = Channels::new();
    let components = Components::new(window.clone(), &channels, &initial_settings);

    wire_event_observers(&channels, &components);

    let game_engine_command_emitter = channels.game_engine_command.emitter.clone();

    // Set up keyboard shortcuts
    app.set_accels_for_action("win.undo", &["<Control>z"]);
    app.set_accels_for_action("win.redo", &["<Control><Shift>z"]);
    app.set_accels_for_action("win.new-game", &["<Control>n"]);
    app.set_accels_for_action("win.pause", &["space"]);
    app.set_accels_for_action("win.restart", &["<Control>r"]);
    app.set_accels_for_action("win.toggle-fullscreen", &["F11"]);

    // Create menu model for hamburger menu
    let menu = Menu::new();

    // Add all menu items
    menu.append(Some(&t!("menu-new-game")), Some("win.new-game"));
    menu.append(Some(&t!("menu-restart")), Some("win.restart"));
    menu.append(Some(&t!("menu-statistics")), Some("win.statistics"));
    menu.append(Some(&t!("menu-seed")), Some("win.seed"));
    menu.append_submenu(
        Some("Settings"),
        components.settings_menu_ui.borrow().get_menu(),
    );
    menu.append(Some(&t!("menu-about")), Some("win.about"));

    // Add menu button to header bar
    let header_bar = HeaderBar::new();

    // Create difficulty selector dropdown with label
    let difficulty_box = gtk4::Box::builder()
        .name("difficulty-box")
        .orientation(Orientation::Horizontal)
        .spacing(5)
        .build();

    let difficulty_label = gtk4::Label::new(Some("Difficulty:"));
    difficulty_box.append(&difficulty_label);

    let all_difficulties = Difficulty::all()
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<String>>();

    let difficulty_selector = gtk4::DropDown::from_strings(
        all_difficulties
            .iter()
            .map(|d| d.as_str())
            .collect::<Vec<&str>>()
            .as_slice(),
    );

    difficulty_selector.set_tooltip_text(Some(&t!("select-difficulty")));
    difficulty_box.append(&difficulty_selector);

    // Set initial selection based on current settings
    let current_difficulty = initial_settings.difficulty;
    difficulty_selector.set_selected(current_difficulty.index() as u32);

    // Handle difficulty changes
    let game_engine_command_emitter_new_game = game_engine_command_emitter.clone();
    difficulty_selector.connect_selected_notify(move |selector| {
        let new_difficulty = Difficulty::from_index(selector.selected() as usize);
        game_engine_command_emitter_new_game
            .emit(GameEngineCommand::NewGame(Some(new_difficulty), None));
    });

    header_bar.pack_start(&difficulty_box);

    let solve_button = Button::with_label(&t!("solve-button"));

    // Create left side box for timer and hints
    let left_box = gtk4::Box::builder()
        .name("left-box")
        .orientation(Orientation::Horizontal)
        .spacing(10) // Slightly larger spacing between groups
        .build();

    // Create pause button
    left_box.append(&components.timer_button.borrow().button);
    left_box.append(&components.game_info_ui.borrow().timer_label);
    left_box.append(&components.hint_button_ui.borrow().hint_button);
    let hints_label = Label::new(Some(&t!("hints-label")));
    hints_label.set_css_classes(&["hints-label"]);
    left_box.append(&hints_label);
    left_box.append(&components.game_info_ui.borrow().hints_label);

    header_bar.pack_start(&left_box);

    // Create right side box for controls
    let right_box = gtk4::Box::builder()
        .name("right-box")
        .orientation(Orientation::Horizontal)
        .spacing(5)
        .css_classes(["menu-box"])
        .build();

    // Create buttons first
    right_box.append(components.history_controls_ui.borrow().undo_button.as_ref());
    right_box.append(components.history_controls_ui.borrow().redo_button.as_ref());
    if Settings::is_debug_mode() {
        right_box.append(&solve_button);
    }

    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();

    // Pack the controls on the right
    header_bar.pack_end(&menu_button); // Hamburger menu goes last
    header_bar.pack_end(&right_box); // Controls go before hamburger menu

    window.set_titlebar(Some(&header_bar));

    // Create a vertical box for puzzle grid and vertical clues
    let puzzle_vertical_box = gtk4::Box::builder()
        .name("puzzle-vertical-box")
        .orientation(Orientation::Vertical)
        .build();

    let game_engine_command_emitter_solve = game_engine_command_emitter.clone();
    solve_button.connect_clicked(move |_| {
        game_engine_command_emitter_solve.emit(GameEngineCommand::Solve);
    });

    // Add CSS for selected cells
    let provider = CssProvider::new();
    provider.load_from_resource("/org/emojiclu/style.css");

    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let puzzle_background = gtk4::Frame::builder()
        .name("puzzle-mat-board")
        .css_classes(["puzzle-mat-board"])
        .child(&components.puzzle_grid_ui.borrow().grid)
        .build();

    let scrolled_window = components.layout_manager.borrow().scrolled_window.clone();

    // Assemble the UI
    puzzle_vertical_box.append(&puzzle_background);
    puzzle_vertical_box.append(&components.tutorial_ui.borrow().scrolled_window);
    puzzle_vertical_box.append(&components.clue_panels_ui.borrow().vertical_grid);
    puzzle_vertical_box.set_hexpand(false);

    let game_box = components.game_info_ui.borrow().game_box.clone();
    game_box.append(&puzzle_vertical_box);
    game_box.append(&components.clue_panels_ui.borrow().horizontal_grid);

    let top_level_box = gtk4::Box::builder()
        .name("top-level-box")
        .orientation(Orientation::Vertical)
        .visible(true)
        .hexpand(true)
        .vexpand(true)
        .halign(gtk4::Align::Fill)
        .valign(gtk4::Align::Center)
        .css_classes(["app-background"])
        .build();

    top_level_box.append(game_box.as_ref());
    top_level_box.append(&components.pause_screen_ui.borrow().pause_screen_box);

    scrolled_window.set_child(Some(&top_level_box));
    // window.set_child(Some(&top_level_box));
    window.set_child(Some(&scrolled_window));

    window.present();

    // Add actions for keyboard shortcuts and menu items
    let action_undo = SimpleAction::new("undo", None);
    let game_engine_command_emitter_undo = game_engine_command_emitter.clone();
    action_undo.connect_activate(move |_, _| {
        game_engine_command_emitter_undo.emit(GameEngineCommand::Undo);
    });
    window.add_action(&action_undo);

    let action_redo = SimpleAction::new("redo", None);
    let game_engine_command_emitter_redo = game_engine_command_emitter.clone();
    action_redo.connect_activate(move |_, _| {
        game_engine_command_emitter_redo.emit(GameEngineCommand::Redo);
    });
    window.add_action(&action_redo);

    // Add new game action that uses current difficulty
    let action_new_game = SimpleAction::new("new-game", None);
    action_new_game.connect_activate({
        let game_engine_command_emitter = game_engine_command_emitter.clone();
        move |_, _| {
            game_engine_command_emitter.emit(GameEngineCommand::NewGame(None, None));
        }
    });
    window.add_action(&action_new_game);

    let action_statistics = SimpleAction::new("statistics", None);
    let stats_manager_stats = Rc::clone(&components.stats_manager);

    action_statistics.connect_activate({
        let settings = components.settings_projection.clone();
        let window = window.clone();
        move |_, _| {
            StatsDialog::show(
                &window,
                settings.borrow().current_settings().difficulty,
                &stats_manager_stats.borrow_mut(),
                None,
                || {},
            );
        }
    });

    window.add_action(&action_statistics);

    let action_about = SimpleAction::new("about", None);
    action_about.connect_activate(move |_, _| {
        let dialog = AboutDialog::builder()
            .program_name(&t!("app-title"))
            .version(APP_VERSION)
            .authors(vec![t!("about-author").as_str()])
            .website(&t!("about-website"))
            .website_label(&t!("about-website-label"))
            .license_type(License::MitX11)
            .build();
        dialog.present();
    });
    window.add_action(&action_about);
    // Initialize game with saved difficulty
    match saved_game_state {
        Some(save_state) => {
            game_engine_command_emitter.emit(GameEngineCommand::LoadState(save_state));
        }
        None => {
            game_engine_command_emitter.emit(GameEngineCommand::NewGame(
                Some(initial_settings.difficulty),
                Settings::seed_from_env(),
            ));
        }
    }
    // delete me
    // game_engine_command_emitter.emit(GameEngineCommand::ChangeSettings(settings.borrow().clone()));

    // Add seed action
    let action_seed = SimpleAction::new("seed", None);
    action_seed.connect_activate({
        let seed_dialog_ref = components.seed_dialog.clone();
        move |_, _| {
            seed_dialog_ref.borrow().show_seed();
        }
    });
    window.add_action(&action_seed);

    // Add restart action
    let action_restart = SimpleAction::new("restart", None);
    action_restart.connect_activate({
        let game_engine_command_emitter = game_engine_command_emitter.clone();
        move |_, _| {
            game_engine_command_emitter.emit(GameEngineCommand::Restart);
        }
    });
    window.add_action(&action_restart);

    // Add fullscreen toggle action
    let action_toggle_fullscreen = SimpleAction::new("toggle-fullscreen", None);
    let window_for_fullscreen = window.clone();
    action_toggle_fullscreen.connect_activate(move |_, _| {
        if window_for_fullscreen.is_fullscreen() {
            window_for_fullscreen.unfullscreen();
        } else {
            window_for_fullscreen.fullscreen();
        }
    });
    window.add_action(&action_toggle_fullscreen);

    window.connect_close_request({
        let components = Rc::new(RefCell::new(components));
        move |_| {
            log::info!(target: "window", "{}", t!("destroying-window"));
            if !components
                .borrow()
                .game_state
                .borrow_mut()
                .get_game_save_state()
                .save()
            {
                log::error!(target: "window", "Failed to save game state");
            }
            components.borrow_mut().destroy();
            // save game here
            glib::signal::Propagation::Proceed
        }
    });
}
