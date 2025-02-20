use std::{cell::RefCell, rc::Rc};

use gtk4::{
    prelude::*, Align, ApplicationWindow, IconTheme, ScrolledWindow, TextBuffer, TextTagTable,
    TextView, WrapMode,
};
use log::info;

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    game::settings::Settings,
    helpers::Capitalize,
    model::{
        ClueWithAddress, Deduction, DeductionKind, Difficulty, Dimensions, GameActionEvent,
        GameBoard, GameStateEvent, GlobalEvent, LayoutConfiguration,
    },
    solver::{
        clue_completion_evaluator::is_clue_fully_completed, deduce_clue, simplify_deductions,
        ConstraintSolver,
    },
    ui::ImageSet,
};

use super::template::TemplateParser;

#[derive(Debug, Clone, PartialEq, Eq)]
enum TutorialStep {
    Disabled,
    HintUsagePhase1,
    HintUsagePhase2(ClueWithAddress),
    HintUsagePhase3(ClueWithAddress, Deduction),
    HintUsagePhase3Oops(ClueWithAddress, Deduction),
    Undo,
    SelectAClue,
    PlayToEnd,
}

impl Default for TutorialStep {
    fn default() -> Self {
        if TutorialStep::skip_beginning() {
            TutorialStep::SelectAClue
        } else {
            TutorialStep::HintUsagePhase1
        }
    }
}
impl TutorialStep {
    fn skip_beginning() -> bool {
        std::env::var("SKIP_TUT").is_ok()
    }
}

pub struct TutorialUI {
    window: Rc<ApplicationWindow>,
    resources: Rc<ImageSet>,
    tutorial_text: TextView,
    pub scrolled_window: ScrolledWindow,
    game_state_subscription: Option<Unsubscriber<GameStateEvent>>,
    game_action_subscription: Option<Unsubscriber<GameActionEvent>>,
    global_event_subscription: Option<Unsubscriber<GlobalEvent>>,
    game_action_emitter: EventEmitter<GameActionEvent>,
    current_step: TutorialStep,
    buffer: TextBuffer,
    settings: Settings,
    current_board: Option<GameBoard>,
    current_clue: Option<ClueWithAddress>,
    layout: Dimensions,
}

impl Destroyable for TutorialUI {
    fn destroy(&mut self) {
        if let Some(subscription) = self.game_state_subscription.take() {
            subscription.unsubscribe();
        }
        if let Some(subscription) = self.game_action_subscription.take() {
            subscription.unsubscribe();
        }
        if let Some(subscription) = self.global_event_subscription.take() {
            subscription.unsubscribe();
        }
    }
}

impl TutorialUI {
    pub fn new(
        game_state_observer: EventObserver<GameStateEvent>,
        game_action_observer: EventObserver<GameActionEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
        game_action_emitter: EventEmitter<GameActionEvent>,
        window: &Rc<ApplicationWindow>,
        resources: &Rc<ImageSet>,
        settings: &Settings,
        layout: &LayoutConfiguration,
    ) -> Rc<RefCell<Self>> {
        let text_tag_table = TextTagTable::new();
        let buffer = TextBuffer::builder().tag_table(&text_tag_table).build();

        let tutorial_text = TextView::builder()
            .name("tutorial-text")
            .visible(true)
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .halign(Align::Start)
            .valign(Align::Start)
            .css_classes(["tutorial-text"])
            .vexpand(true)
            .wrap_mode(WrapMode::Word)
            .build();

        let scrolled_window = ScrolledWindow::builder()
            .name("tutorial-box")
            .child(&tutorial_text)
            .hexpand(true)
            .vexpand(true)
            .build();

        let tutorial_ui = Rc::new(RefCell::new(Self {
            tutorial_text,
            scrolled_window,
            game_state_subscription: None,
            game_action_subscription: None,
            global_event_subscription: None,
            game_action_emitter,
            current_step: TutorialStep::default(),
            buffer,
            window: window.clone(),
            resources: resources.clone(),
            settings: settings.clone(),
            current_board: None,
            current_clue: None,
            layout: layout.tutorial.clone(),
        }));

        TutorialUI::bind_observers(
            Rc::clone(&tutorial_ui),
            game_state_observer,
            game_action_observer,
            global_event_observer,
        );
        tutorial_ui.borrow_mut().sync_tutorial_text();

        tutorial_ui
    }

    fn bind_observers(
        tutorial_ui: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
        game_action_observer: EventObserver<GameActionEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
    ) {
        let game_state_subscription = {
            let tutorial_ui = tutorial_ui.clone();
            game_state_observer.subscribe(move |event| {
                tutorial_ui.borrow_mut().handle_game_state_event(event);
            })
        };

        let game_action_subscription = {
            let tutorial_ui = tutorial_ui.clone();
            game_action_observer.subscribe(move |event| {
                tutorial_ui.borrow_mut().handle_game_action_event(event);
            })
        };

        let global_event_subscription = {
            let tutorial_ui = tutorial_ui.clone();
            global_event_observer.subscribe(move |event| {
                tutorial_ui.borrow_mut().handle_global_event(event);
            })
        };

        tutorial_ui.borrow_mut().game_state_subscription = Some(game_state_subscription);
        tutorial_ui.borrow_mut().game_action_subscription = Some(game_action_subscription);
        tutorial_ui.borrow_mut().global_event_subscription = Some(global_event_subscription);
    }

    fn handle_game_state_event(&mut self, event: &GameStateEvent) {
        if self.current_step == TutorialStep::Disabled {
            return;
        }
        match event {
            GameStateEvent::ClueSelected(clue_selection) => {
                self.current_clue = clue_selection
                    .as_ref()
                    .map(|clue_selection| clue_selection.clue.clone());
                let is_focused = clue_selection
                    .as_ref()
                    .map(|cs| cs.is_focused)
                    .unwrap_or(false);
                match &self.current_step {
                    TutorialStep::SelectAClue if is_focused => {
                        self.current_step = TutorialStep::PlayToEnd;
                        self.sync_tutorial_text();
                    }
                    TutorialStep::PlayToEnd => {
                        self.sync_tutorial_text();
                    }
                    _ => {}
                }
            }
            GameStateEvent::ClueHintHighlight(clue_with_address) => {
                if let Some(cwa) = clue_with_address {
                    match &self.current_step {
                        TutorialStep::HintUsagePhase1 => {
                            self.current_step = TutorialStep::HintUsagePhase2(cwa.clone());
                        }
                        _ => {}
                    }
                    self.sync_tutorial_text();
                }
            }
            GameStateEvent::CellHintHighlight(deduction) => match &self.current_step {
                TutorialStep::HintUsagePhase2(cwa) => {
                    self.current_step =
                        TutorialStep::HintUsagePhase3(cwa.clone(), deduction.clone());
                    self.sync_tutorial_text();
                }
                _ => {}
            },
            GameStateEvent::GridUpdate(board) => {
                self.current_board = Some(board.clone());
                match &self.current_step {
                    TutorialStep::HintUsagePhase3(cwa, deduction) => {
                        let task_completed = if deduction.is_positive() {
                            board.is_selected_in_column(
                                &deduction.tile_assertion.tile,
                                deduction.column,
                            )
                        } else {
                            !board.is_candidate_available(
                                deduction.tile_assertion.tile.row,
                                deduction.column,
                                deduction.tile_assertion.tile.variant,
                            )
                        };
                        if task_completed {
                            self.current_step = TutorialStep::Undo;
                        } else {
                            self.current_step =
                                TutorialStep::HintUsagePhase3Oops(cwa.clone(), deduction.clone());
                        }
                        self.sync_tutorial_text();
                    }
                    TutorialStep::PlayToEnd => {
                        self.sync_tutorial_text();
                    }
                    _ => {}
                }
            }
            GameStateEvent::HistoryChanged { history_index, .. } => match &self.current_step {
                TutorialStep::HintUsagePhase3Oops(cwa, deduction) if *history_index == 0 => {
                    self.current_step =
                        TutorialStep::HintUsagePhase3(cwa.clone(), deduction.clone());
                    self.sync_tutorial_text();
                }
                TutorialStep::Undo if *history_index == 0 => {
                    self.game_action_emitter
                        .emit(GameActionEvent::ClueFocus(None));
                    self.current_step = TutorialStep::SelectAClue;
                    self.sync_tutorial_text();
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_game_action_event(&mut self, event: &GameActionEvent) {
        match event {
            GameActionEvent::NewGame(difficulty, _) => {
                if *difficulty == Difficulty::Tutorial {
                    self.reset_tutorial();
                } else {
                    self.current_step = TutorialStep::Disabled;
                    self.sync_tutorial_text();
                }
            }
            _ => {}
        }
        // Handle game action events and update tutorial text
    }

    fn handle_global_event(&mut self, event: &GlobalEvent) {
        match event {
            GlobalEvent::LayoutChanged(layout) => {
                self.layout = layout.tutorial.clone();
                self.sync_layout();
            }
            GlobalEvent::SettingsChanged(settings) => {
                self.settings = settings.clone();
                self.sync_tutorial_text();
            }
            GlobalEvent::ImagesOptimized(image_set) => {
                self.resources = image_set.clone();
                self.sync_tutorial_text();
            }
            _ => {}
        }
    }

    fn sync_layout(&mut self) {
        if self.layout.height > 0 {
            self.scrolled_window.set_visible(true);
        } else {
            self.scrolled_window.set_visible(false);
        }
        self.tutorial_text.set_width_request(self.layout.width);
        self.scrolled_window.set_height_request(self.layout.height);
        self.scrolled_window.set_width_request(self.layout.width);
    }

    fn reset_tutorial(&mut self) {
        self.current_step = TutorialStep::default();
        self.sync_tutorial_text();
    }

    fn control_text_main(&self) -> String {
        if self.settings.touch_screen_controls {
            "long press".to_string()
        } else {
            "left click".to_string()
        }
    }

    fn control_text_alt(&self) -> String {
        if self.settings.touch_screen_controls {
            "tap".to_string()
        } else {
            "right click".to_string()
        }
    }

    fn get_tutorial_text(&self) -> Option<String> {
        match &self.current_step {
            TutorialStep::HintUsagePhase1 =>
                 Some("<b>Welcome to MindHunt</b>, a logical deduction puzzle game.

Above this text is the puzzle grid, to the right and bottom are clues. Your goal \
is to figure out the location of various tiles making deductions with the clues.

First, let's start off using the hint system. Press the {icon:view-reveal-symbolic} button (in the top-right corner) now.".to_string()),


            TutorialStep::HintUsagePhase2(_) =>
                Some("Great! The game selected and highlighted a clue you should look at.

<b>Hover over the selected clue</b> to see a tooltip explaining what the clue means.

Pressing {icon:view-reveal-symbolic} a second time gives you additional help.

Press the {icon:view-reveal-symbolic} button a second time, now.".to_string()),

            TutorialStep::HintUsagePhase3(_, deduction) => {
                let prefix = format!(
                    "The second time we pressed the hint button, the game highlighted a tile that is one of the deductions you can make from the clue.

We can deduce here from the clue that tile {{tile:{}}} in column {} should be {}.\n\n",
                    deduction.tile_assertion.tile.to_string(),
                    deduction.column + 1,
                    if deduction.tile_assertion.is_positive() {
                        "selected"
                    } else {
                        "eliminated"
                    }
                );

                let template2 = format!(
                    "{} the tile {{tile:{}}} in column {} now.",
                    if deduction.tile_assertion.is_positive() {
                        self.control_text_main().capitalize()
                    } else {
                        self.control_text_alt().capitalize()
                    },
                    deduction.tile_assertion.tile.to_string(),
                    deduction.column + 1
                );
                Some(format!("{} {}", prefix, template2))
            }
            TutorialStep::HintUsagePhase3Oops(_, deduction) => {
                let template = format!(
                    "Oops! That wasn't quite right. Tile {{tile:{}}} in column {} is not {}.",
                    deduction.tile_assertion.tile.to_string(),
                    deduction.column + 1,
                    if deduction.tile_assertion.is_positive() {
                        "selected"
                    } else {
                        "eliminated"
                    }
                );

                let template2 = format!(
                    "Press the {{icon:edit-undo-symbolic}} button repeatedly until no further undos are possible.",
                );
                Some(format!("{}{}", template, template2))
            }
            TutorialStep::Undo => {
                Some(format!(
                    "Great!
                    
Now, at any time, you can undo any moves you make with the undo button, or by pressing <tt>Ctrl+Z</tt>.

Let's get the game back to the start. Press the {{icon:edit-undo-symbolic}} button repeatedly until no further undos are possible.",
                ))
            }
            TutorialStep::SelectAClue => {
                Some(format!(
                    "Great! Now, let's use the clue selection system.
                    
Selecting a clue helps you track what you're currently working on. You can select a clue either by clicking on it, or navigating to it using the keys <tt>A</tt> or <tt>D</tt>.

Let's select a clue now.",
                ))
            }
            TutorialStep::PlayToEnd => {
                Some(self.play_to_end_template())
            }
            TutorialStep::Disabled => {
                None
            }
        }
    }

    fn sync_tutorial_text(&mut self) {
        let buffer = self.buffer.clone();
        let mut start = buffer.start_iter();
        let mut end = buffer.end_iter();
        buffer.delete(&mut start, &mut end);

        if let Some(template) = self.get_tutorial_text() {
            let display = WidgetExt::display(self.window.as_ref());
            let theme = IconTheme::for_display(&display);
            let parser = TemplateParser::new(self.resources.clone(), Some(Rc::new(theme)));
            parser.append_to_text_buffer(&self.tutorial_text, &mut end, &template);
        }

        info!("Tutorial step: {:?}", self.current_step);
    }

    fn play_to_end_template(&self) -> String {
        if let Some(board) = &self.current_board {
            if board.is_incorrect() {
                return format!(
                    "<b>Oops!</b> You've made a mistake. Let's try again.

Press the {{icon:edit-undo-symbolic}} button.",
                );
            } else if board.is_complete() {
                return "<b>Congratulations!</b>

You've completed the tutorial! You can try an easy puzzle by selecting <tt>'Easy'</tt> from the top-left difficulty selector.

Or, press <tt>Ctrl+N</tt> to restart this tutorial."
                    .to_string();
            } else if let Some(selection) = &self.current_clue {
                let selected_clue_marked_completed = board.is_clue_completed(&selection.address());
                let deductions = simplify_deductions(
                    board,
                    ConstraintSolver::deduce_clue(board, &selection.clue),
                    &selection.clue,
                );

                let deductions = if deductions.is_empty() {
                    simplify_deductions(board, deduce_clue(board, &selection.clue), &selection.clue)
                } else {
                    deductions
                };

                if deductions.is_empty() {
                    if is_clue_fully_completed(&selection.clue, board) {
                        if selected_clue_marked_completed {
                            return "Let's move on to the next clue.".to_string();
                        } else {
                            return format!(
                                "<b>Clue complete!</b>
                    
This clue is fully encoded in the board. Mark it as completed by pressing <tt>'C'</tt>, or by {}ing the clue.",
                                self.control_text_alt()
                            );
                        }
                    }
                    return format!("We can't deduce anything more from this clue at this time, <i>but it is not complete</i>. Move on to next clue.");
                }

                let first_deduction = deductions.first().unwrap();
                let template = format!(
                    "<big>{}</big>:\n\n{}\n\nSo, {{tile:{}}} <b>{}</b> in column <big><tt>{}</tt></big>{}.\n\n",
                    selection.clue.clue_type.get_title(),
                    selection.clue.description(),
                    first_deduction.tile_assertion.tile.to_string(),
                    if first_deduction.tile_assertion.is_positive() {
                        "must be"
                    } else {
                        "cannot be"
                    },
                    first_deduction.column + 1,
                    if first_deduction.deduction_kind.as_ref().is_some_and(|deduction_kind| deduction_kind == &DeductionKind::Converging) {
                        " (<i>all possible solutions for this clue overlap this cell, so it can only be one of the clue values</i>)"
                    } else {
                        ""
                    }
                );
                return template;
            } else {
                return "Let's keep going. Select a clue.".to_string();
            }
        } else {
            return "Weird".to_string();
        }
    }
}
