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
        ClueWithAddress, Deduction, DeductionKind, Difficulty, Dimensions, GameBoard,
        GameBoardChangeReason, GameEngineCommand, GameEngineEvent, LayoutConfiguration,
        LayoutManagerEvent,
    },
    solver::{
        clue_completion_evaluator::is_clue_fully_completed, deduce_clue, simplify_deductions,
        ConstraintSolver,
    },
    ui::{deferred_size_reallocation, ImageSet},
};

use super::template::TemplateParser;
use fluent_i18n::t;

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
    game_state_subscription: Option<Unsubscriber<GameEngineEvent>>,
    game_action_subscription: Option<Unsubscriber<GameEngineCommand>>,
    layout_event_subscription: Option<Unsubscriber<LayoutManagerEvent>>,
    game_engine_command_emitter: EventEmitter<GameEngineCommand>,
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
        if let Some(subscription) = self.layout_event_subscription.take() {
            subscription.unsubscribe();
        }
    }
}

impl TutorialUI {
    pub fn new(
        game_engine_event_observer: EventObserver<GameEngineEvent>,
        game_engine_command_observer: EventObserver<GameEngineCommand>,
        layout_manager_event_observer: EventObserver<LayoutManagerEvent>,
        game_engine_command_emitter: EventEmitter<GameEngineCommand>,
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
            .halign(Align::Fill)
            .valign(Align::Start)
            .css_classes(["tutorial-text"])
            .vexpand(true)
            .hexpand(true)
            .wrap_mode(WrapMode::Word)
            .build();

        let scrolled_window = ScrolledWindow::builder()
            .name("tutorial-box")
            .child(&tutorial_text)
            .hexpand(true)
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .build();

        let tutorial_ui = Rc::new(RefCell::new(Self {
            tutorial_text,
            scrolled_window,
            game_state_subscription: None,
            game_action_subscription: None,
            layout_event_subscription: None,
            game_engine_command_emitter,
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
            game_engine_event_observer,
            game_engine_command_observer,
            layout_manager_event_observer,
        );
        tutorial_ui.borrow_mut().sync_tutorial_text();

        tutorial_ui
    }

    fn bind_observers(
        tutorial_ui: Rc<RefCell<Self>>,
        game_engine_event_observer: EventObserver<GameEngineEvent>,
        game_engine_command_observer: EventObserver<GameEngineCommand>,
        layout_manager_event_observer: EventObserver<LayoutManagerEvent>,
    ) {
        let game_state_subscription = {
            let tutorial_ui = tutorial_ui.clone();
            game_engine_event_observer.subscribe(move |event| {
                tutorial_ui.borrow_mut().handle_game_engine_event(event);
            })
        };

        let game_action_subscription = {
            let tutorial_ui = tutorial_ui.clone();
            game_engine_command_observer.subscribe(move |event| {
                tutorial_ui.borrow_mut().handle_game_action_command(event);
            })
        };

        let layout_event_subscription = {
            let tutorial_ui = tutorial_ui.clone();
            layout_manager_event_observer.subscribe(move |event| {
                tutorial_ui.borrow_mut().handle_layout_event(event);
            })
        };

        tutorial_ui.borrow_mut().game_state_subscription = Some(game_state_subscription);
        tutorial_ui.borrow_mut().game_action_subscription = Some(game_action_subscription);
        tutorial_ui.borrow_mut().layout_event_subscription = Some(layout_event_subscription);
    }

    fn handle_game_engine_event(&mut self, event: &GameEngineEvent) {
        if self.current_step == TutorialStep::Disabled {
            return;
        }
        match event {
            GameEngineEvent::ClueSelected(clue_selection) => {
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
            GameEngineEvent::ClueHintHighlighted(clue_with_address) => {
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
            GameEngineEvent::HintSuggested(deduction) => match &self.current_step {
                TutorialStep::HintUsagePhase2(cwa) => {
                    self.current_step =
                        TutorialStep::HintUsagePhase3(cwa.clone(), deduction.clone());
                    self.sync_tutorial_text();
                }
                _ => {}
            },
            GameEngineEvent::GameBoardUpdated {
                board,
                history_index,
                change_reason,
                ..
            } => {
                self.current_board = Some(board.clone());
                match &self.current_step {
                    TutorialStep::HintUsagePhase3Oops(cwa, deduction)
                        if *history_index == 0 && *change_reason == GameBoardChangeReason::Undo =>
                    {
                        self.current_step =
                            TutorialStep::HintUsagePhase3(cwa.clone(), deduction.clone());
                        self.sync_tutorial_text();
                    }
                    TutorialStep::Undo
                        if *history_index == 0 && *change_reason == GameBoardChangeReason::Undo =>
                    {
                        self.game_engine_command_emitter
                            .emit(GameEngineCommand::ClueFocus(None));
                        self.current_step = TutorialStep::SelectAClue;
                        self.sync_tutorial_text();
                    }
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
            GameEngineEvent::SettingsChanged(settings) => {
                self.settings = settings.clone();
                self.sync_tutorial_text();
            }
            _ => {}
        }
    }

    fn handle_game_action_command(&mut self, event: &GameEngineCommand) {
        // TODO - GameEngineEvent::NewGameStarted(difficulty)
        match event {
            GameEngineCommand::NewGame(difficulty, _) => {
                let difficulty = difficulty.unwrap_or(self.settings.difficulty);
                if difficulty == Difficulty::Tutorial {
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

    fn handle_layout_event(&mut self, event: &LayoutManagerEvent) {
        match event {
            LayoutManagerEvent::LayoutChanged(layout) => {
                self.layout = layout.tutorial.clone();
                self.sync_layout();
            }
            LayoutManagerEvent::ImagesOptimized(image_set) => {
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
        self.scrolled_window.set_height_request(self.layout.height);
        self.scrolled_window.set_width_request(self.layout.width);
    }

    fn reset_tutorial(&mut self) {
        self.current_step = TutorialStep::default();
        self.sync_tutorial_text();
    }

    fn control_text_main(&self) -> String {
        if self.settings.touch_screen_controls {
            t!("control-long-press")
        } else {
            t!("control-left-click")
        }
    }

    fn control_text_alt(&self) -> String {
        if self.settings.touch_screen_controls {
            t!("control-tap")
        } else {
            t!("control-right-click")
        }
    }

    fn get_tutorial_text(&self) -> Option<String> {
        match &self.current_step {
            TutorialStep::HintUsagePhase1 => Some(t!("tutorial-welcome")),

            TutorialStep::HintUsagePhase2(_) => Some(t!("tutorial-phase2")),

            TutorialStep::HintUsagePhase3(_, deduction) => {
                let action = if deduction.tile_assertion.is_positive() {
                    t!("action-selected")
                } else {
                    t!("action-eliminated")
                };

                let control_text = if deduction.tile_assertion.is_positive() {
                    self.control_text_main().capitalize()
                } else {
                    self.control_text_alt().capitalize()
                };

                let prefix = t!("tutorial-phase3-prefix", {
                    "tile" => deduction.tile_assertion.tile.to_string(),
                    "column" => (deduction.column + 1).to_string(),
                    "action" => action.clone()
                });

                let action_text = t!("tutorial-phase3-action", {
                    "control_text" => control_text,
                    "tile" => deduction.tile_assertion.tile.to_string(),
                    "column" => (deduction.column + 1).to_string()
                });

                Some(format!("{}\n\n{}", prefix, action_text))
            }
            TutorialStep::HintUsagePhase3Oops(_, deduction) => {
                let action = if deduction.tile_assertion.is_positive() {
                    t!("action-selected")
                } else {
                    t!("action-eliminated")
                };

                Some(t!("tutorial-phase3-oops", {
                    "tile" => deduction.tile_assertion.tile.to_string(),
                    "column" => (deduction.column + 1).to_string(),
                    "action" => action
                }))
            }
            TutorialStep::Undo => Some(t!("tutorial-undo")),
            TutorialStep::SelectAClue => Some(t!("tutorial-select-clue")),
            TutorialStep::PlayToEnd => Some(self.play_to_end_template()),
            TutorialStep::Disabled => None,
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

            deferred_size_reallocation(&self.tutorial_text);
        }

        info!("Tutorial step: {:?}", self.current_step);
    }

    fn play_to_end_template(&self) -> String {
        if let Some(board) = &self.current_board {
            if board.is_incorrect() {
                return t!("tutorial-mistake");
            } else if board.is_complete() {
                return t!("tutorial-congratulations");
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
                            return t!("tutorial-next-clue");
                        } else {
                            return t!("tutorial-clue-complete", {
                                "action" => self.control_text_alt()
                            });
                        }
                    }
                    return t!("tutorial-no-deduction");
                }

                let first_deduction = deductions.first().unwrap();
                let must_be = if first_deduction.tile_assertion.is_positive() {
                    t!("action-must-be")
                } else {
                    t!("action-cannot-be")
                };

                let converging_note = if first_deduction
                    .deduction_kind
                    .as_ref()
                    .is_some_and(|deduction_kind| deduction_kind == &DeductionKind::Converging)
                {
                    t!("converging-note")
                } else {
                    String::new()
                };

                return t!("tutorial-clue-analysis", {
                    "clue_title" => selection.clue.clue_type.get_title(),
                    "clue_description" => selection.clue.description(),
                    "tile" => first_deduction.tile_assertion.tile.to_string(),
                    "must_be" => must_be,
                    "column" => (first_deduction.column + 1).to_string(),
                    "converging_note" => converging_note
                });
            } else {
                return t!("tutorial-keep-going");
            }
        } else {
            return t!("weird");
        }
    }
}
