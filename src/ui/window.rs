use crate::destroyable::Destroyable;
use crate::events::{Channel, EventEmitter};
use crate::game::game_state::GameState;
use crate::game::settings::Settings;
use crate::game::stats_manager::StatsManager;
use crate::model::{
    game_state_snapshot, Difficulty, GameActionEvent, GameStateEvent, GlobalEvent, InputEvent,
};
use crate::ui::input_translator::InputTranslator;
use crate::ui::seed_dialog::SeedDialog;
use crate::ui::settings_menu_ui::SettingsMenuUI;
use crate::ui::stats_dialog::StatsDialog;
use crate::ui::submit_ui::SubmitUI;
use crate::ui::timer_button_ui::TimerButtonUI;
use crate::ui::top_level_input_event_monitor::TopLevelInputEventMonitor;
use crate::ui::NotQuiteRightDialog;
use gio::{Menu, SimpleAction};
use glib::timeout_add_local_once;
use gtk4::gdk::{Display, Monitor};
use gtk4::{
    prelude::*, AboutDialog, Application, ApplicationWindow, Button, CssProvider, HeaderBar, Label,
    License, MenuButton, Orientation, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::time::Duration;

use super::audio_set::AudioSet;
use super::clue_panels_ui::CluePanelsUI;
use super::game_info_ui::GameInfoUI;
use super::history_controls_ui::HistoryControlsUI;
use super::layout_manager::{ClueStats, LayoutManager};
use super::puzzle_grid_ui::PuzzleGridUI;
use super::resource_manager::ResourceManager;
use super::tutorial_ui::TutorialUI;

const APP_VERSION: &str = env!("APP_VERSION");

fn pause_screen() -> Rc<gtk4::Box> {
    let pause_label = Label::builder()
        .name("pause-label")
        .label("PAUSED")
        .css_classes(["pause-label"])
        .visible(true)
        .hexpand(true)
        .vexpand(true)
        .build();
    let pause_screen_box = gtk4::Box::builder()
        .name("pause-screen")
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();
    pause_screen_box.append(&pause_label);
    Rc::new(pause_screen_box)
}

fn hint_button_handler(
    game_action_emitter: EventEmitter<GameActionEvent>,
    game_state: &Rc<RefCell<GameState>>,
    audio_set: &Rc<AudioSet>,
    window: &Rc<ApplicationWindow>,
) -> impl Fn(&Button) {
    let game_state = Rc::clone(&game_state);
    let audio_set_hint = Rc::clone(&audio_set);
    let window = Rc::clone(&window);

    move |button| {
        let board_is_incorrect = game_state.borrow().current_board.is_incorrect();
        log::trace!(target: "window", "Handling hint button click");
        if board_is_incorrect {
            log::trace!(target: "window", "Board is incorrect, showing rewind dialog");
            let media = audio_set_hint.random_lose_sound();
            media.play();
            NotQuiteRightDialog::new(&window, game_action_emitter.clone()).show();
        } else {
            log::trace!(target: "window", "Board is correct, showing hint");
            game_action_emitter.emit(GameActionEvent::ShowHint);
            button.set_sensitive(false);
            let button = button.clone();
            timeout_add_local_once(Duration::from_secs(4), move || {
                log::trace!(target: "window", "Re-enabling hint button");
                button.set_sensitive(true);
            });
        }
    }
}

pub fn build_ui(app: &Application) {
    let (game_action_emitter, game_action_observer) = Channel::<GameActionEvent>::new();
    let (game_state_emitter, game_state_observer) = Channel::<GameStateEvent>::new();
    let (global_event_emitter, global_event_observer) = Channel::<GlobalEvent>::new();
    let (input_event_emitter, input_event_observer) = Channel::<InputEvent>::new();

    let settings = Rc::new(RefCell::new(Settings::load()));
    let saved_game_state = game_state_snapshot::load_game_state_snapshot();
    if let Some(save_state) = &saved_game_state {
        log::info!(target: "window", "Loaded saved game state");
        // if these disagree, then bad things happen.
        settings.borrow_mut().difficulty = save_state.board.solution.difficulty;
    } else {
        log::info!(target: "window", "No saved game state found");
    }

    let resource_manager =
        ResourceManager::new(global_event_observer.clone(), global_event_emitter.clone());
    let image_set = resource_manager.borrow().get_image_set();
    let audio_set = resource_manager.borrow().get_audio_set();

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
            .title("Mind Hunt")
            .icon_name("org.timcharper.MindHunt")
            .resizable(true)
            .decorated(true)
            .default_height(desired_height as i32)
            .default_width(desired_width.min(max_desired_width) as i32)
            .build(),
    );

    let scrolled_window = gtk4::ScrolledWindow::builder()
        .hexpand_set(true)
        .vexpand_set(true)
        .build();

    let pause_screen = pause_screen();
    // Create game area with puzzle and horizontal clues side by side
    let game_box = Rc::new(
        gtk4::Box::builder()
            .name("game-box")
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .halign(gtk4::Align::Center)
            .hexpand(true)
            .margin_start(10)
            .margin_end(10)
            .build(),
    );

    let layout_manager = LayoutManager::new(
        window.clone(),
        global_event_emitter.clone(),
        game_action_observer.clone(),
        game_state_observer.clone(),
        scrolled_window.clone(),
        settings.borrow().difficulty,
    );

    // Set up keyboard shortcuts
    app.set_accels_for_action("win.undo", &["<Control>z"]);
    app.set_accels_for_action("win.redo", &["<Control><Shift>z"]);
    app.set_accels_for_action("win.new-game", &["<Control>n"]);
    app.set_accels_for_action("win.pause", &["space"]);
    app.set_accels_for_action("win.restart", &["<Control>r"]);

    // Create menu model for hamburger menu
    let menu = Menu::new();

    // Create Settings submenu
    let settings_menu_ui = SettingsMenuUI::new(
        window.clone(),
        global_event_emitter.clone(),
        settings.clone(),
    );

    // Add all menu items
    menu.append(Some("New Game"), Some("win.new-game"));
    menu.append(Some("Restart"), Some("win.restart"));
    menu.append(Some("Statistics"), Some("win.statistics"));
    menu.append(Some("Seed"), Some("win.seed"));
    menu.append_submenu(Some("Settings"), settings_menu_ui.borrow().get_menu());
    menu.append(Some("About"), Some("win.about"));

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

    difficulty_selector.set_tooltip_text(Some("Select Difficulty"));
    difficulty_box.append(&difficulty_selector);

    // Set initial selection based on current settings
    let current_difficulty = settings.borrow().difficulty;
    difficulty_selector.set_selected(current_difficulty.index() as u32);

    // Handle difficulty changes
    let settings_ref = Rc::clone(&settings);
    let game_action_emitter_new_game = game_action_emitter.clone();
    difficulty_selector.connect_selected_notify(move |selector| {
        let new_difficulty = Difficulty::from_index(selector.selected() as usize);
        settings_ref.borrow_mut().difficulty = new_difficulty;
        let _ = settings_ref.borrow().save();
        game_action_emitter_new_game.emit(GameActionEvent::NewGame(new_difficulty, None));
    });

    header_bar.pack_start(&difficulty_box);

    let history_controls_ui = HistoryControlsUI::new(game_state_observer.clone());

    let game_info_ui = GameInfoUI::new(
        game_state_observer.clone(),
        game_box.clone(),
        pause_screen.clone(),
    );

    let solve_button = Button::with_label("Solve");
    let hint_button = Button::from_icon_name("view-reveal-symbolic");

    // Add tooltips
    hint_button.set_tooltip_text(Some("Show Hint"));

    let default_layout =
        LayoutManager::calculate_layout(settings.borrow().difficulty, Some(ClueStats::default()));

    // Create puzzle grid and clue set UI first
    let puzzle_grid_ui = PuzzleGridUI::new(
        input_event_emitter.clone(),
        game_state_observer.clone(),
        global_event_observer.clone(),
        image_set.clone(),
        default_layout.clone(),
        &settings.borrow(),
    );

    let clue_set_ui = CluePanelsUI::new(
        input_event_emitter.clone(),
        game_state_observer.clone(),
        global_event_observer.clone(),
        &image_set,
        default_layout.clone(),
        &settings.borrow(),
    );

    // Create game state with UI references
    let game_state = GameState::new(
        game_action_observer.clone(),
        game_state_emitter.clone(),
        global_event_observer.clone(),
        settings.borrow().clone(),
    );

    // Remove the old button_box since controls are now in header
    let stats_manager = Rc::new(RefCell::new(StatsManager::new()));

    let submit_ui = SubmitUI::new(
        game_state_observer.clone(),
        game_action_emitter.clone(),
        &stats_manager,
        &audio_set,
        &window,
    );

    // Create left side box for timer and hints
    let left_box = gtk4::Box::builder()
        .name("left-box")
        .orientation(Orientation::Horizontal)
        .spacing(10) // Slightly larger spacing between groups
        .build();

    // Create pause button
    let timer_button = TimerButtonUI::new(&window, game_action_emitter.clone());
    left_box.append(&timer_button.borrow().button);
    left_box.append(&game_info_ui.borrow().timer_label);
    let hints_label = Label::new(Some("Hints: "));
    hints_label.set_css_classes(&["hints-label"]);
    left_box.append(&hints_label);
    left_box.append(&game_info_ui.borrow().hints_label);

    header_bar.pack_start(&left_box);

    // Create right side box for controls
    let right_box = gtk4::Box::builder()
        .name("right-box")
        .orientation(Orientation::Horizontal)
        .spacing(5)
        .css_classes(["menu-box"])
        .build();

    // Create buttons first
    right_box.append(history_controls_ui.borrow().undo_button.as_ref());
    right_box.append(history_controls_ui.borrow().redo_button.as_ref());
    if Settings::is_debug_mode() {
        right_box.append(&solve_button);
    }
    right_box.append(&hint_button);

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

    let game_action_emitter_solve = game_action_emitter.clone();
    solve_button.connect_clicked(move |_| {
        game_action_emitter_solve.emit(GameActionEvent::Solve);
    });

    // Connect hint button
    hint_button.connect_clicked(hint_button_handler(
        game_action_emitter.clone(),
        &game_state,
        &audio_set,
        &window,
    ));

    // Add CSS for selected cells
    let provider = CssProvider::new();
    provider.load_from_resource("/org/mindhunt/style.css");

    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let puzzle_background = gtk4::Frame::builder()
        .name("puzzle-mat-board")
        .css_classes(["puzzle-mat-board"])
        .child(&puzzle_grid_ui.borrow().grid)
        .build();

    // Instantiate TutorialUI
    let tutorial_ui = TutorialUI::new(
        game_state_observer.clone(),
        game_action_observer.clone(),
        global_event_observer.clone(),
        game_action_emitter.clone(),
        &window,
        &image_set,
        &settings.borrow(),
        &default_layout,
    );

    // Assemble the UI
    puzzle_vertical_box.append(&puzzle_background);
    puzzle_vertical_box.append(&tutorial_ui.borrow().scrolled_window);
    puzzle_vertical_box.append(&clue_set_ui.borrow().vertical_grid);
    puzzle_vertical_box.set_hexpand(false);

    game_box.append(&puzzle_vertical_box);
    game_box.append(&clue_set_ui.borrow().horizontal_grid);

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
    top_level_box.append(pause_screen.as_ref());

    scrolled_window.set_child(Some(&top_level_box));
    // window.set_child(Some(&top_level_box));
    window.set_child(Some(&scrolled_window));

    window.present();

    // Add actions for keyboard shortcuts and menu items
    let action_undo = SimpleAction::new("undo", None);
    let game_action_emitter_undo = game_action_emitter.clone();
    action_undo.connect_activate(move |_, _| {
        game_action_emitter_undo.emit(GameActionEvent::Undo);
    });
    window.add_action(&action_undo);

    let action_redo = SimpleAction::new("redo", None);
    let game_action_emitter_redo = game_action_emitter.clone();
    action_redo.connect_activate(move |_, _| {
        game_action_emitter_redo.emit(GameActionEvent::Redo);
    });
    window.add_action(&action_redo);

    // Add new game action that uses current difficulty
    let action_new_game = SimpleAction::new("new-game", None);
    let settings_ref: Rc<RefCell<Settings>> = Rc::clone(&settings);
    action_new_game.connect_activate({
        let game_action_emitter = game_action_emitter.clone();
        move |_, _| {
            let difficulty = settings_ref.borrow().difficulty;
            game_action_emitter.emit(GameActionEvent::NewGame(difficulty, None));
        }
    });
    window.add_action(&action_new_game);

    let action_statistics = SimpleAction::new("statistics", None);
    let stats_manager_stats = Rc::clone(&stats_manager);

    action_statistics.connect_activate({
        let settings = Rc::clone(&settings);
        let window = window.clone();
        move |_, _| {
            StatsDialog::show(
                &window,
                settings.borrow().difficulty,
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
            .program_name("Mind Hunt")
            .version(APP_VERSION)
            .authors(vec!["Tim Harper"])
            .website("https://github.com/timcharper/mindhunt")
            .website_label("GitHub Repository")
            .license_type(License::MitX11)
            .build();
        dialog.present();
    });
    window.add_action(&action_about);

    let seed_dialog = SeedDialog::new(
        &window,
        game_action_emitter.clone(),
        game_state_observer.clone(),
    );

    // Initialize game controls
    let game_controls = TopLevelInputEventMonitor::new(
        window.clone(),
        scrolled_window.clone(),
        input_event_emitter.clone(),
        global_event_observer.clone(),
        &settings.borrow(),
    );

    // Initialize game with saved difficulty
    match saved_game_state {
        Some(save_state) => {
            game_action_emitter.emit(GameActionEvent::LoadState(save_state));
        }
        None => {
            game_action_emitter.emit(GameActionEvent::NewGame(
                settings.borrow().difficulty,
                Settings::seed_from_env(),
            ));
        }
    }
    global_event_emitter.emit(GlobalEvent::SettingsChanged(settings.borrow().clone()));

    // Add seed action
    let action_seed = SimpleAction::new("seed", None);
    let seed_dialog_ref = seed_dialog.clone();
    action_seed.connect_activate(move |_, _| {
        seed_dialog_ref.borrow().show_seed();
    });
    window.add_action(&action_seed);

    // Add restart action
    let action_restart = SimpleAction::new("restart", None);
    let game_action_emitter_restart = game_action_emitter.clone();
    action_restart.connect_activate(move |_, _| {
        game_action_emitter_restart.emit(GameActionEvent::Restart);
    });
    window.add_action(&action_restart);

    // Initialize input translator
    let input_translator = InputTranslator::new(
        game_action_emitter.clone(),
        input_event_observer.clone(),
        global_event_observer.clone(),
        &settings.borrow(),
    );

    window.connect_close_request(move |_| {
        println!("Destroying window");
        if !game_state.borrow_mut().get_game_save_state().save() {
            log::error!(target: "window", "Failed to save game state");
        }
        history_controls_ui.borrow_mut().destroy();
        game_state.borrow_mut().destroy();
        game_info_ui.borrow_mut().destroy();
        submit_ui.borrow_mut().destroy();
        puzzle_grid_ui.borrow_mut().destroy();
        clue_set_ui.borrow_mut().destroy();
        timer_button.borrow_mut().destroy();
        layout_manager.borrow_mut().destroy();
        seed_dialog.borrow_mut().destroy();
        settings_menu_ui.borrow_mut().destroy();
        game_controls.borrow_mut().destroy();
        input_translator.borrow_mut().destroy();
        resource_manager.borrow_mut().destroy();

        // save game here
        glib::signal::Propagation::Proceed
    });
}
