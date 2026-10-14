mod analysis;
mod analysis_browser;
mod analysis_pages;
mod board;
mod large_collection;
mod pgn_projection;
mod presentation;
mod progress;
mod puzzle;

use analysis::NodeHandle;
use analysis_browser::AnalysisBrowser;
use analysis_pages::{link_action, paginate, AnalysisPage};
use board::{Board, Color, Piece, PieceKind, TapResult};
use kobo_sdk::{
    action_id, ActionId, BandAlign, BannerLevel, Context, ControlState, Glyph, KoboApp,
    ParagraphPresentation, RichTextSpan, Screen, ScreenBuilder, SlotWidth, StoreResult,
    TextPresentation,
};
use large_collection::{CollectionReader, ReadMode, ReadProgress};
use pgn_projection::project_analysis;
use presentation::{analysis_text, solution_text, StatusMode};
use progress::Progress;
use puzzle::{parse_puzzle_file, Puzzle, PuzzleCollection};
use std::{collections::VecDeque, process::ExitCode};

const SIDE: usize = 8;
const CELLS: usize = SIDE * SIDE;
const RESET: &str = "reset";
const FLIP: &str = "flip";
const EXIT: &str = "exit";
const MODE_TOGGLE: &str = "mode-toggle";
const ORIENTATION_LOCK: &str = "orientation-lock";
const PUZZLE_PICKER: &str = "puzzle-picker";
const ANALYSIS_TOGGLE: &str = "analysis-toggle";
const ANALYSIS_PREVIOUS: &str = "analysis-previous";
const ANALYSIS_NEXT: &str = "analysis-next";
const ANALYSIS_CLOSE: &str = "analysis-close";
const PUZZLE_PICKER_CANCEL: &str = "puzzle-picker-cancel";
const COLLECTION_ACTION_PREFIX: &str = "collection-";
const PROMOTE_QUEEN: &str = "promote-queen";
const PROMOTE_ROOK: &str = "promote-rook";
const PROMOTE_BISHOP: &str = "promote-bishop";
const PROMOTE_KNIGHT: &str = "promote-knight";
const PROMOTION_CANCEL: &str = "promotion-cancel";
const PUZZLES_FILE: &str = "puzzles.json";
const PROGRESS_KEY: &str = "progress.v1";
const EXAMPLE_PUZZLES: &[u8] = include_bytes!("../examples/puzzles.json");

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum BoardMode {
    #[default]
    Solution,
    FreeBoard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SolutionFeedback {
    Correct,
    Wrong,
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingPromotion {
    before: Board,
    from: usize,
    to: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UciMove {
    from: usize,
    to: usize,
    promotion: Option<PieceKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CollectionEntry {
    key: String,
    title: Option<String>,
    valid: bool,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CollectionLoadKind {
    Metadata,
    Initial,
    Selection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CollectionLoad {
    key: String,
    kind: CollectionLoadKind,
}

#[derive(Default)]
struct ChessBoardApp {
    board: Board,
    puzzles: Vec<Puzzle>,
    puzzle_index: usize,
    solution_ply: usize,
    solution_feedback: Option<SolutionFeedback>,
    mode: BoardMode,
    pending_promotion: Option<PendingPromotion>,
    analysis_browser: AnalysisBrowser,
    collections: Vec<CollectionEntry>,
    active_collection: Option<String>,
    collection_load: Option<CollectionLoad>,
    collection_reader: Option<CollectionReader>,
    metadata_queue: VecDeque<String>,
    picker_open: bool,
    creating_default: bool,
    progress: Progress,
    progress_ready: bool,
    progress_durable: bool,
    progress_dirty: bool,
    progress_generation: u64,
    progress_saving: Option<u64>,
    progress_warning: Option<String>,
    file_error: Option<String>,
    sleeping: bool,
    flipped: bool,
    orientation_locked: bool,
    description_visible: bool,
}

impl ChessBoardApp {
    fn activate_collection(&mut self, key: Option<String>, collection: PuzzleCollection) {
        self.puzzles = collection.puzzles;
        self.active_collection = key;
        self.file_error = None;
        let index = self
            .active_collection
            .as_deref()
            .and_then(|key| self.progress.file(key))
            .and_then(|file| file.current_puzzle_id.as_deref())
            .and_then(|id| self.puzzles.iter().position(|puzzle| puzzle.id == id))
            .unwrap_or(0);
        self.select_puzzle(index);
    }

    fn show_examples_fallback(&mut self, error: impl Into<String>) {
        let collection =
            parse_puzzle_file(EXAMPLE_PUZZLES).expect("bundled example puzzles are valid");
        self.activate_collection(None, collection);
        self.file_error = Some(error.into());
    }

    fn begin_collection_discovery(&mut self, context: &mut Context, keys: Vec<String>) {
        let mut keys = keys
            .into_iter()
            .filter(|key| is_puzzle_collection_key(key))
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();

        self.collections = keys
            .iter()
            .map(|key| CollectionEntry {
                key: key.clone(),
                title: None,
                valid: false,
                error: None,
            })
            .collect();
        self.metadata_queue = keys.into();
        self.collection_load = None;
        self.collection_reader = None;

        if self.collections.is_empty() {
            self.creating_default = true;
            context.store().save(PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec());
        } else {
            self.load_next_metadata(context);
        }
    }

    fn load_next_metadata(&mut self, context: &mut Context) {
        if let Some(key) = self.metadata_queue.pop_front() {
            self.collection_load = Some(CollectionLoad {
                key: key.clone(),
                kind: CollectionLoadKind::Metadata,
            });
            context.store().load(key);
        } else {
            self.load_initial_collection(context);
        }
    }

    fn load_initial_collection(&mut self, context: &mut Context) {
        let remembered = self.progress.active_file.as_deref().and_then(|key| {
            self.collections
                .iter()
                .find(|entry| entry.key == key && entry.valid)
        });
        let preferred = remembered
            .or_else(|| {
                self.collections
                    .iter()
                    .find(|entry| entry.key == PUZZLES_FILE && entry.valid)
            })
            .or_else(|| self.collections.iter().find(|entry| entry.valid))
            .map(|entry| entry.key.clone());

        let Some(key) = preferred else {
            self.collection_load = None;
            self.show_examples_fallback(
                "No valid puzzle collection could be loaded; showing bundled examples.",
            );
            self.show(context);
            return;
        };

        self.collection_load = Some(CollectionLoad {
            key: key.clone(),
            kind: CollectionLoadKind::Initial,
        });
        context.store().load(key);
    }

    fn request_collection(&mut self, context: &mut Context, key: String) {
        self.analysis_browser.close();
        self.picker_open = false;
        if self.active_collection.as_deref() == Some(key.as_str()) {
            self.show(context);
            return;
        }
        if self.collection_load.is_some() {
            return;
        }

        self.file_error = None;
        self.collection_load = Some(CollectionLoad {
            key: key.clone(),
            kind: CollectionLoadKind::Selection,
        });
        context.store().load(key);
        self.show(context);
    }

    // Publish only a fully parsed collection, leaving active board/progress unchanged on errors.
    fn finish_collection_load(
        &mut self,
        context: &mut Context,
        key: &str,
        parsed: Result<PuzzleCollection, String>,
    ) {
        let Some(load) = self.collection_load.take() else {
            return;
        };
        if load.key != key {
            self.file_error = Some(format!("{key}: unexpected collection response."));
            self.show(context);
            return;
        }
        match load.kind {
            CollectionLoadKind::Metadata => {
                self.update_collection_metadata(key, &parsed);
                self.load_next_metadata(context);
            }
            CollectionLoadKind::Initial => match parsed {
                Ok(collection) => {
                    let title = collection.title.clone();
                    if let Some(entry) = self.collections.iter_mut().find(|entry| entry.key == key)
                    {
                        entry.title = title;
                        entry.valid = true;
                        entry.error = None;
                    }
                    self.activate_collection(Some(key.to_owned()), collection);
                    self.show(context);
                }
                Err(error) => {
                    if let Some(entry) = self.collections.iter_mut().find(|entry| entry.key == key)
                    {
                        entry.valid = false;
                        entry.error = Some(error.clone());
                    }
                    // Try another valid neighbor rather than losing the entire library.
                    self.load_initial_collection(context);
                }
            },
            CollectionLoadKind::Selection => match parsed {
                Ok(collection) => {
                    self.remember_current_puzzle();
                    let title = collection.title.clone();
                    if let Some(entry) = self.collections.iter_mut().find(|entry| entry.key == key)
                    {
                        entry.title = title;
                        entry.valid = true;
                        entry.error = None;
                    }
                    self.activate_collection(Some(key.to_owned()), collection);
                    self.remember_active_collection();
                    self.save_progress(context);
                    self.show(context);
                }
                Err(error) => {
                    if let Some(entry) = self.collections.iter_mut().find(|entry| entry.key == key)
                    {
                        entry.valid = false;
                        entry.error = Some(error.clone());
                    }
                    self.file_error = Some(error);
                    self.show(context);
                }
            },
        }
    }

    fn finish_metadata_probe(&mut self, context: &mut Context, key: &str) {
        if !self
            .collection_load
            .as_ref()
            .is_some_and(|load| load.key == key && load.kind == CollectionLoadKind::Metadata)
        {
            return;
        }
        self.collection_load = None;
        if let Some(entry) = self.collections.iter_mut().find(|entry| entry.key == key) {
            // A first chunk proves accessibility and a bounded size, but is not
            // full JSON validation. Selection fully validates before activation.
            entry.valid = true;
            entry.error = None;
        }
        self.load_next_metadata(context);
    }

    fn update_collection_metadata(&mut self, key: &str, parsed: &Result<PuzzleCollection, String>) {
        let Some(entry) = self.collections.iter_mut().find(|entry| entry.key == key) else {
            return;
        };
        match parsed {
            Ok(collection) => {
                entry.title = collection.title.clone();
                entry.valid = true;
                entry.error = None;
            }
            Err(error) => {
                entry.title = None;
                entry.valid = false;
                entry.error = Some(error.clone());
            }
        }
    }

    fn select_puzzle(&mut self, index: usize) {
        self.analysis_browser.close();
        let puzzle = &self.puzzles[index];
        self.board = Board::from_fen(&puzzle.fen).expect("validated puzzle FEN");
        if !self.orientation_locked {
            self.flipped = puzzle.side_to_move() == Color::Black;
        }
        self.puzzle_index = index;
        self.reset_attempt();
    }

    fn reset_attempt(&mut self) {
        self.description_visible = false;
        self.solution_ply = 0;
        self.solution_feedback = None;
        self.pending_promotion = None;
    }

    /// The reader is independent of the solve position and progress.
    /// The reader never owns or changes the solving board.
    fn open_analysis(&mut self) -> bool {
        if self.mode != BoardMode::Solution
            || self.pending_promotion.is_some()
            || self.picker_open
            || self.sleeping
            || self.collection_load.is_some()
        {
            return false;
        }
        self.puzzles
            .get(self.puzzle_index)
            .is_some_and(|puzzle| self.analysis_browser.open(puzzle))
    }

    fn close_analysis(&mut self) -> bool {
        self.analysis_browser.close()
    }

    fn select_analysis_node(&mut self, node: NodeHandle) -> bool {
        if self.pending_promotion.is_some()
            || self.picker_open
            || self.sleeping
            || self.mode != BoardMode::Solution
        {
            return false;
        }
        let tree = self
            .puzzles
            .get(self.puzzle_index)
            .and_then(|puzzle| puzzle.analysis.as_ref());
        self.analysis_browser.select(tree, node)
    }

    /// Page count comes from the same projection used by the visible pane.
    fn turn_analysis_page(&mut self, forward: bool, page_count: usize) -> bool {
        if self.pending_promotion.is_some() || self.picker_open || self.sleeping {
            return false;
        }
        self.analysis_browser.turn_page(forward, page_count)
    }

    fn analysis_pages(&self) -> Vec<AnalysisPage> {
        self.puzzles
            .get(self.puzzle_index)
            .map(|puzzle| paginate(&project_analysis(puzzle)))
            .unwrap_or_default()
    }

    fn analysis_available(&self) -> bool {
        self.mode == BoardMode::Solution
            && self.puzzles.get(self.puzzle_index)
                .is_some_and(|puzzle| analysis_text(puzzle).available())
    }

    fn analysis_page(screen: ScreenBuilder, page: &AnalysisPage, selected: Option<NodeHandle>) -> ScreenBuilder {
        let mut text = String::new();
        let mut spans = Vec::new();
        let mut links = Vec::new();
        for fragment in &page.fragments {
            let start = text.len();
            text.push_str(&fragment.text);
            if let Some(target) = fragment.target {
                let end = text.len();
                spans.push(RichTextSpan {
                    start,
                    end,
                    presentation: TextPresentation {
                        strong: true,
                        underline: true,
                        highlighted: selected == Some(target),
                        ..TextPresentation::default()
                    },
                });
                links.push((link_action(fragment.source_run), start, end));
            }
        }
        // PAGE_LINKS (3) is below SDK MAX_TEXT_LINKS (16); styling one
        // span per link also stays well below MAX_RICH_TEXT_SPANS (256).
        screen.rich_text_linking(text, spans, ParagraphPresentation::default(), links)
    }

    fn displayed_board(&self) -> &Board {
        self.analysis_browser.preview_board().unwrap_or(&self.board)
    }

    fn note_progress_change(&mut self) {
        if self.progress_ready && self.progress_durable {
            self.progress_generation = self.progress_generation.wrapping_add(1);
            self.progress_dirty = true;
        }
    }

    fn remember_current_puzzle(&mut self) {
        let (Some(key), Some(puzzle)) = (
            self.active_collection.clone(),
            self.puzzles.get(self.puzzle_index),
        ) else {
            return;
        };
        let puzzle_id = puzzle.id.clone();
        if self.progress.remember_puzzle(&key, &puzzle_id) {
            self.note_progress_change();
        }
    }

    fn remember_active_collection(&mut self) {
        let Some(key) = self.active_collection.clone() else {
            return;
        };
        let changed_active = self.progress.set_active_file(&key);
        let puzzle_id = self
            .puzzles
            .get(self.puzzle_index)
            .map(|puzzle| puzzle.id.clone());
        let changed_puzzle = puzzle_id
            .as_deref()
            .is_some_and(|id| self.progress.remember_puzzle(&key, id));
        if changed_active || changed_puzzle {
            self.note_progress_change();
        }
    }

    fn mark_current_solved(&mut self) {
        let (Some(key), Some(puzzle)) = (
            self.active_collection.clone(),
            self.puzzles.get(self.puzzle_index),
        ) else {
            return;
        };
        let puzzle_id = puzzle.id.clone();
        if self.progress.mark_solved(&key, &puzzle_id) {
            self.note_progress_change();
        }
    }

    fn current_is_solved(&self) -> bool {
        let (Some(key), Some(puzzle)) = (
            self.active_collection.as_deref(),
            self.puzzles.get(self.puzzle_index),
        ) else {
            return false;
        };
        self.progress.is_solved(key, &puzzle.id)
    }

    fn save_progress(&mut self, context: &mut Context) {
        if !self.progress_ready
            || !self.progress_durable
            || !self.progress_dirty
            || self.progress_saving.is_some()
        {
            return;
        }
        match self.progress.to_bytes() {
            Ok(bytes) => {
                let generation = self.progress_generation;
                self.progress_saving = Some(generation);
                context.store().save(PROGRESS_KEY, bytes);
            }
            Err(_) => {
                self.progress_warning = Some("Progress not saved".into());
            }
        }
    }

    fn handle_progress_load(&mut self, result: StoreResult) {
        self.progress_ready = true;
        match result {
            StoreResult::Loaded {
                value: Some(bytes), ..
            } => match Progress::parse(&bytes) {
                Ok(progress) => {
                    self.progress = progress;
                    self.progress_durable = true;
                    self.progress_warning = None;
                }
                Err(error) => {
                    self.progress = Progress::new();
                    self.progress_durable = false;
                    self.progress_warning = Some(format!("Progress file needs fixing: {error}"));
                }
            },
            StoreResult::Loaded { value: None, .. } => {
                self.progress = Progress::new();
                self.progress_durable = true;
                self.progress_warning = None;
            }
            StoreResult::Denied(error) => {
                self.progress = Progress::new();
                self.progress_durable = false;
                self.progress_warning = Some(format!(
                    "Progress could not be read ({error}); progress will not be saved."
                ));
            }
            _ => {
                self.progress = Progress::new();
                self.progress_durable = false;
                self.progress_warning =
                    Some("Progress could not be read; progress will not be saved.".into());
            }
        }
    }

    fn handle_progress_save(&mut self, context: &mut Context, result: StoreResult) {
        let Some(saved_generation) = self.progress_saving.take() else {
            return;
        };
        match result {
            StoreResult::Saved { .. } => {
                if saved_generation == self.progress_generation {
                    self.progress_dirty = false;
                    self.progress_warning = None;
                } else {
                    self.progress_dirty = true;
                    self.save_progress(context);
                }
            }
            StoreResult::Denied(_)
            | StoreResult::Loaded { .. }
            | StoreResult::Forgotten { .. }
            | StoreResult::Keys(_)
            | StoreResult::ShelfWritten { .. }
            | StoreResult::ShelfRead { .. }
            | StoreResult::ShelfRemoved { .. }
            | StoreResult::Shelf(_) => {
                self.progress_dirty = true;
                self.progress_warning = Some("Progress not saved".into());
            }
        }
    }

    fn show(&self, context: &mut Context) {
        context.set_screen(self.screen());
    }

    fn screen(&self) -> Screen {
        let displayed_board = self.displayed_board();
        let selected = displayed_board.selected();
        let feedback_glyph = if self.mode == BoardMode::Solution {
            match self.solution_feedback {
                Some(SolutionFeedback::Complete) => Some(Glyph::ThumbUp),
                Some(SolutionFeedback::Wrong) => Some(Glyph::ThumbDown),
                _ => None,
            }
        } else {
            None
        };

        let cells = (0..CELLS).map(|display_square| {
            let square = board_square(display_square, self.flipped);
            let piece = displayed_board.piece_at(square);
            (
                square_action(square),
                square_label(square, piece),
                if display_square == 27 {
                    feedback_glyph.or_else(|| piece.map(piece_glyph))
                } else {
                    piece.map(piece_glyph)
                },
                selected == Some(square),
            )
        });

        let title = if self.sleeping {
            "Sleeping - press power to wake".to_owned()
        } else if self.file_error.is_some() {
            "Puzzle file needs fixing".to_owned()
        } else if let Some(puzzle) = self.puzzles.get(self.puzzle_index) {
            let difficulty = puzzle
                .difficulty
                .as_ref()
                .map(|value| format!(" ({value})"))
                .unwrap_or_default();
            format!(
                "{}/{} {}{difficulty}",
                self.puzzle_index + 1,
                self.puzzles.len(),
                puzzle.id
            )
        } else {
            "0/0".to_owned()
        };
        let free_board = self.mode == BoardMode::FreeBoard;
        let orientation_locked = self.orientation_locked;
        let analysis_open = self.analysis_browser.is_open();
        let analysis_available = self.analysis_available();
        let analysis_pages = if analysis_open { self.analysis_pages() } else { Vec::new() };
        let analysis_position = self.analysis_browser.page().min(analysis_pages.len().saturating_sub(1));
        let title = if analysis_open {
            format!("{title} · Analysis {}/{}", analysis_position + 1, analysis_pages.len())
        } else { title };
        let mut screen = ScreenBuilder::new("chessboard")
            .top_bar(title)
            .top_bar_glyph(EXIT, "Return to Kobo reader", Glyph::Close)
            .top_bar_action(PUZZLE_PICKER, "Puzzles")
            .board_with_selection(SIDE as u8, cells)
            .band(
                BandAlign::Middle,
                [
                    (
                        SlotWidth::Fill,
                        // Use the *existing* vacant slot, keeping original
                        // four controls flush with the outer board border.
                        // Navigation lives at the same height as the toolbar;
                        // nothing extra consumes board height below it.
                        Box::new(move |slot: ScreenBuilder| {
                            if analysis_open {
                                slot.controls_with_selection(
                                    4,
                                    [
                                        (ANALYSIS_PREVIOUS, "Previous analysis page", Glyph::Previous, false),
                                        (ANALYSIS_TOGGLE, "Analysis active", Glyph::Book, true),
                                        (ANALYSIS_NEXT, "Next analysis page", Glyph::Next, false),
                                        (ANALYSIS_CLOSE, "Close analysis", Glyph::Close, false),
                                    ],
                                )
                            } else if analysis_available {
                                slot.controls_with_selection(
                                    1,
                                    [(ANALYSIS_TOGGLE, "Open Analysis", Glyph::Book, false)],
                                )
                            } else {
                                slot.button_with_state(
                                    ANALYSIS_TOGGLE, "Analysis", ControlState::Disabled
                                )
                            }
                        }) as Box<dyn FnOnce(ScreenBuilder) -> ScreenBuilder>,
                    ),
                    (
                        // Four original 10 mm controls and right alignment.
                        SlotWidth::Fixed(430),
                        Box::new(move |slot| {
                            slot.controls_with_selection(
                                4,
                                [
                                    (MODE_TOGGLE, "Toggle free board", Glyph::Grid, free_board),
                                    (
                                        ORIENTATION_LOCK,
                                        "Toggle orientation lock",
                                        Glyph::Key,
                                        orientation_locked,
                                    ),
                                    (RESET, "Reset position", Glyph::Refresh, false),
                                    (FLIP, "Flip board", Glyph::SwapVertical, false),
                                ],
                            )
                        }),
                    ),
                ],
            );

        if let Some(puzzle) = self.puzzles.get(self.puzzle_index) {
            let view = solution_text(
                puzzle,
                if self.mode == BoardMode::Solution {
                    StatusMode::Solution
                } else {
                    StatusMode::FreeBoard
                },
                self.solution_feedback == Some(SolutionFeedback::Correct),
                self.current_is_solved(),
                self.description_visible,
            );
            if analysis_open {
                if let Some(content) = analysis_pages.get(analysis_position) {
                    screen = Self::analysis_page(
                        screen, content, self.analysis_browser.selected_node()
                    );
                }
            } else {
                screen = screen.secondary(view.status);
                if let Some(description) = view.revealed_description {
                    screen = screen.text(description);
                }
            }
        }
        if let Some(error) = self.file_error.as_ref() {
            screen = screen.text(error.clone());
        }
        if let Some(warning) = self.progress_warning.as_ref() {
            screen = screen.banner(BannerLevel::Attention, warning.clone());
        }

        if self.pending_promotion.is_some() {
            screen = screen.modal("Promote pawn", |modal| {
                modal
                    .choose(
                        "Choose promotion piece",
                        [
                            (PROMOTE_QUEEN, "Queen"),
                            (PROMOTE_ROOK, "Rook"),
                            (PROMOTE_BISHOP, "Bishop"),
                            (PROMOTE_KNIGHT, "Knight"),
                        ],
                    )
                    .button(PROMOTION_CANCEL, "Cancel")
            });
        }

        if self.picker_open {
            screen = screen.modal("Puzzle collections", |modal| {
                modal
                    .rows(self.collections.iter().enumerate().map(|(index, entry)| {
                        (
                            collection_action(index),
                            collection_display_name(entry),
                            collection_summary(entry, self.active_collection.as_deref()),
                            Glyph::Grid,
                        )
                    }))
                    .button(PUZZLE_PICKER_CANCEL, "Cancel")
            });
        }

        screen.build()
    }

    fn turn_puzzle(&mut self, context: &mut Context, forward: bool) {
        let next = if forward {
            self.puzzle_index
                .checked_add(1)
                .filter(|&index| index < self.puzzles.len())
        } else {
            self.puzzle_index.checked_sub(1)
        };
        let Some(next) = next else {
            return;
        };
        self.select_puzzle(next);
        self.remember_current_puzzle();
        self.save_progress(context);
        self.show(context);
    }

    fn handle_square_tap(&mut self, square: usize) -> bool {
        if self.analysis_browser.is_open() {
            return false;
        }
        if (self.mode == BoardMode::Solution
            && self.solution_feedback == Some(SolutionFeedback::Complete))
            || self.pending_promotion.is_some()
        {
            return false;
        }

        let before = self.board.clone();
        match self.board.tap(square) {
            TapResult::NoChange => false,
            TapResult::SelectionChanged => true,
            TapResult::Moved { from, to } => {
                if self.mode == BoardMode::Solution {
                    let attempted = uci_move(from, to, None);
                    self.check_solver_move(before, &attempted);
                }
                true
            }
            TapResult::Promotion { from, to, .. } => {
                self.pending_promotion = Some(PendingPromotion { before, from, to });
                true
            }
        }
    }

    fn finish_promotion(&mut self, kind: PieceKind) {
        let Some(pending) = self.pending_promotion.take() else {
            return;
        };
        let before = pending.before;
        self.board = before.clone();
        self.board.clear_selection();
        if !self.board.promote_pawn(pending.from, pending.to, kind) {
            self.board = before;
            self.board.clear_selection();
            self.file_error = Some("The staged pawn promotion could not be applied.".into());
            return;
        }

        if self.mode == BoardMode::Solution {
            let attempted = uci_move(pending.from, pending.to, Some(kind));
            self.check_solver_move(before, &attempted);
        }
    }

    fn set_mode(&mut self, mode: BoardMode) {
        if self.mode == mode {
            return;
        }

        self.analysis_browser.close();
        self.pending_promotion = None;
        self.description_visible = false;
        self.board.clear_selection();
        match mode {
            BoardMode::FreeBoard => {
                self.mode = BoardMode::FreeBoard;
                self.solution_feedback = None;
            }
            BoardMode::Solution => {
                self.board.reset();
                self.reset_attempt();
                self.mode = BoardMode::Solution;
            }
        }
    }

    fn cancel_promotion(&mut self) {
        let Some(pending) = self.pending_promotion.take() else {
            return;
        };
        self.board = pending.before;
        self.board.clear_selection();
    }

    fn check_solver_move(&mut self, before: Board, attempted: &str) {
        let original_ply = self.solution_ply;
        let expected = self
            .puzzles
            .get(self.puzzle_index)
            .and_then(|puzzle| puzzle.solution.get(original_ply))
            .cloned();

        let Some(expected) = expected else {
            self.restore_wrong_move(before);
            return;
        };

        if attempted != expected {
            self.restore_wrong_move(before);
            return;
        }

        self.solution_ply += 1;

        let reply = self
            .puzzles
            .get(self.puzzle_index)
            .and_then(|puzzle| puzzle.solution.get(self.solution_ply))
            .cloned();

        if let Some(reply) = reply {
            if !self.apply_stored_move(&reply) {
                self.restore_invalid_reply(before, original_ply, &reply);
                return;
            }
            self.solution_ply += 1;
        }

        let solution_len = self
            .puzzles
            .get(self.puzzle_index)
            .map_or(0, |puzzle| puzzle.solution.len());
        let complete = self.solution_ply >= solution_len;
        self.solution_feedback = Some(if complete {
            SolutionFeedback::Complete
        } else {
            SolutionFeedback::Correct
        });
        if complete {
            self.description_visible = true;
            self.mark_current_solved();
        }
    }

    fn apply_stored_move(&mut self, movement: &str) -> bool {
        let Some(movement) = parse_uci_move(movement) else {
            return false;
        };
        match movement.promotion {
            Some(kind) => self.board.promote_pawn(movement.from, movement.to, kind),
            None => self.board.move_piece(movement.from, movement.to),
        }
    }

    fn restore_wrong_move(&mut self, before: Board) {
        self.board = before;
        self.board.clear_selection();
        self.solution_feedback = Some(SolutionFeedback::Wrong);
    }

    fn restore_invalid_reply(&mut self, before: Board, original_ply: usize, reply: &str) {
        self.board = before;
        self.board.clear_selection();
        self.solution_ply = original_ply;
        self.solution_feedback = None;
        let puzzle_id = self
            .puzzles
            .get(self.puzzle_index)
            .map_or("unknown", |puzzle| puzzle.id.as_str());
        self.file_error = Some(format!(
            "Puzzle {puzzle_id}: stored reply {reply} could not be applied."
        ));
    }
}

impl KoboApp for ChessBoardApp {
    fn on_start(&mut self, context: &mut Context) {
        context.store().list();
        context.store().load(PROGRESS_KEY);
    }

    fn on_store(&mut self, context: &mut Context, result: StoreResult) {
        match result {
            StoreResult::Keys(keys) => self.begin_collection_discovery(context, keys),
            StoreResult::Denied(error) => {
                self.show_examples_fallback(format!(
                    "Puzzle collections could not be listed ({error}); showing bundled examples."
                ));
                self.show(context);
            }
            _ => {}
        }
    }

    fn on_save(&mut self, context: &mut Context, key: &str, result: StoreResult) {
        if key == PROGRESS_KEY {
            self.handle_progress_save(context, result);
            self.show(context);
            return;
        }
        if key != PUZZLES_FILE || !self.creating_default {
            return;
        }
        self.creating_default = false;
        match result {
            StoreResult::Saved { .. } => context.store().list(),
            StoreResult::Denied(error) => {
                self.show_examples_fallback(format!(
                    "The default puzzle collection could not be created ({error}); showing bundled examples."
                ));
                self.show(context);
            }
            _ => {
                self.show_examples_fallback(
                    "The default puzzle collection could not be created; showing bundled examples.",
                );
                self.show(context);
            }
        }
    }

    fn on_load(&mut self, context: &mut Context, key: &str, result: StoreResult) {
        if key == PROGRESS_KEY {
            self.handle_progress_load(result);
            return;
        }
        let Some(load) = self.collection_load.as_ref() else {
            return;
        };
        if load.key != key {
            self.file_error = Some(format!("{key}: unexpected collection response."));
            self.show(context);
            return;
        }
        if matches!(result, StoreResult::Denied(kobo_sdk::StoreError::TooFull)) {
            let mode = if load.kind == CollectionLoadKind::Metadata {
                ReadMode::Metadata
            } else {
                ReadMode::Complete
            };
            match CollectionReader::begin(context, key, mode) {
                Ok(reader) => self.collection_reader = Some(reader),
                Err(error) => self.finish_collection_load(context, key, Err(error)),
            }
            return;
        }
        let parsed = match result {
            StoreResult::Loaded {
                value: Some(bytes), ..
            } => parse_puzzle_file(&bytes).map_err(|error| format!("{key}: {error}")),
            StoreResult::Loaded { value: None, .. } => {
                Err(format!("{key}: the puzzle collection no longer exists."))
            }
            StoreResult::Denied(error) => Err(format!("{key}: could not be read ({error}).")),
            _ => Err(format!("{key}: unexpected store response.")),
        };
        self.finish_collection_load(context, key, parsed);
    }

    fn on_shelf(&mut self, context: &mut Context, name: &str, result: StoreResult) {
        let Some(mut reader) = self.collection_reader.take() else {
            return;
        };
        if reader.wire_name() != name {
            self.collection_reader = Some(reader);
            return;
        }
        let key = reader.key().to_owned();
        match reader.advance(context, result) {
            ReadProgress::Pending => self.collection_reader = Some(reader),
            ReadProgress::Metadata => self.finish_metadata_probe(context, &key),
            ReadProgress::Complete(bytes) => {
                let parsed = parse_puzzle_file(&bytes).map_err(|error| format!("{key}: {error}"));
                self.finish_collection_load(context, &key, parsed);
            }
            ReadProgress::Failed(error) => {
                self.finish_collection_load(context, &key, Err(error));
            }
        }
    }

    fn on_action(&mut self, context: &mut Context, action: ActionId) {
        if self.pending_promotion.is_some() {
            if action == action_id(PROMOTION_CANCEL) {
                self.cancel_promotion();
                self.show(context);
            } else if let Some(kind) = promotion_kind_for_action(action) {
                self.finish_promotion(kind);
                self.save_progress(context);
                self.show(context);
            }
            return;
        }

        if self.picker_open {
            if action == action_id(PUZZLE_PICKER_CANCEL) {
                self.picker_open = false;
                self.show(context);
                return;
            }
            for index in 0..self.collections.len() {
                if action == action_id(&collection_action(index)) {
                    let key = self.collections[index].key.clone();
                    self.request_collection(context, key);
                    return;
                }
            }
            return;
        }

        if self.analysis_browser.is_open() {
            if action == action_id(ANALYSIS_CLOSE) {
                self.close_analysis();
                self.show(context);
                return;
            }
            if action == action_id(ANALYSIS_PREVIOUS) || action == action_id(ANALYSIS_NEXT) {
                let count = self.analysis_pages().len();
                if self.turn_analysis_page(action == action_id(ANALYSIS_NEXT), count) {
                    self.show(context);
                }
                return;
            }
            if let Some(puzzle) = self.puzzles.get(self.puzzle_index) {
                let document = project_analysis(puzzle);
                for (index, run) in document.runs.iter().enumerate() {
                    if let Some(target) = run.target {
                        if action == action_id(&link_action(index)) {
                            if self.select_analysis_node(target) {
                                self.show(context);
                            }
                            return;
                        }
                    }
                }
            }
        }

        if action == action_id(ANALYSIS_TOGGLE) {
            if !self.close_analysis() {
                self.open_analysis();
            }
            self.show(context);
            return;
        }

        if action == action_id(PUZZLE_PICKER) {
            if self.collection_load.is_none() && !self.collections.is_empty() {
                self.analysis_browser.close();
                self.picker_open = true;
                self.show(context);
            }
            return;
        }

        if action == action_id(EXIT) {
            context.exit();
            return;
        }

        if action == action_id(MODE_TOGGLE) {
            self.set_mode(match self.mode {
                BoardMode::Solution => BoardMode::FreeBoard,
                BoardMode::FreeBoard => BoardMode::Solution,
            });
            self.show(context);
            return;
        }

        if action == action_id(ORIENTATION_LOCK) {
            self.orientation_locked = !self.orientation_locked;
            self.show(context);
            return;
        }

        if action == action_id(RESET) {
            self.analysis_browser.close();
            self.board.reset();
            self.reset_attempt();
            self.show(context);
            return;
        }

        if action == action_id(FLIP) {
            self.flipped = !self.flipped;
            self.show(context);
            return;
        }

        for square in 0..CELLS {
            if action == action_id(&square_action(square)) {
                // Avoid an e-ink refresh when a tap changed nothing.
                if self.handle_square_tap(square) {
                    self.save_progress(context);
                    self.show(context);
                }
                return;
            }
        }
    }

    fn on_page_turn(&mut self, context: &mut Context, forward: bool) {
        if self.pending_promotion.is_some() || self.picker_open {
            return;
        }
        if self.analysis_browser.is_open() {
            if self.turn_analysis_page(forward, self.analysis_pages().len()) {
                self.show(context);
            }
        } else {
            self.turn_puzzle(context, forward);
        }
    }

    fn on_suspend(&mut self, context: &mut Context) {
        self.analysis_browser.close();
        self.save_progress(context);
        self.sleeping = true;
        self.show(context);
    }

    fn on_background(&mut self, context: &mut Context) {
        self.analysis_browser.close();
        self.save_progress(context);
    }

    fn on_exit(&mut self, context: &mut Context) {
        self.save_progress(context);
    }

    fn on_resume(&mut self, context: &mut Context) {
        self.sleeping = false;
        self.show(context);
    }
}

fn is_puzzle_collection_key(key: &str) -> bool {
    key == PUZZLES_FILE
        || key
            .strip_prefix("puzzles-")
            .and_then(|name| name.strip_suffix(".json"))
            .is_some_and(|name| !name.is_empty())
}

fn collection_action(index: usize) -> String {
    format!("{COLLECTION_ACTION_PREFIX}{index}")
}

fn collection_display_name(entry: &CollectionEntry) -> String {
    entry
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(&entry.key)
        .to_owned()
}

fn collection_summary(entry: &CollectionEntry, active: Option<&str>) -> String {
    let current = active == Some(entry.key.as_str());
    if current && entry.title.is_some() {
        format!("{} · current", entry.key)
    } else if current {
        "Current collection".into()
    } else if !entry.valid {
        format!("{} · could not be read", entry.key)
    } else if entry.title.is_some() {
        entry.key.clone()
    } else {
        String::new()
    }
}

fn square_action(square: usize) -> String {
    format!("square-{square}")
}

const fn board_square(display_square: usize, flipped: bool) -> usize {
    if flipped {
        CELLS - 1 - display_square
    } else {
        display_square
    }
}

fn square_name(square: usize) -> String {
    debug_assert!(square < CELLS);
    let file = (b'a' + (square % SIDE) as u8) as char;
    let rank = (b'8' - (square / SIDE) as u8) as char;
    format!("{file}{rank}")
}

fn square_from_name(name: &str) -> Option<usize> {
    let bytes = name.as_bytes();
    if bytes.len() != 2 || !(b'a'..=b'h').contains(&bytes[0]) || !(b'1'..=b'8').contains(&bytes[1])
    {
        return None;
    }
    let file = usize::from(bytes[0] - b'a');
    let rank_from_top = usize::from(b'8' - bytes[1]);
    Some(rank_from_top * SIDE + file)
}

fn uci_move(from: usize, to: usize, promotion: Option<PieceKind>) -> String {
    let mut movement = format!("{}{}", square_name(from), square_name(to));
    if let Some(kind) = promotion {
        movement.push(promotion_suffix(kind).expect("only promotable piece kinds reach UCI"));
    }
    movement
}

fn parse_uci_move(movement: &str) -> Option<UciMove> {
    let bytes = movement.as_bytes();
    if bytes.len() != 4 && bytes.len() != 5 {
        return None;
    }
    let from = std::str::from_utf8(&bytes[..2])
        .ok()
        .and_then(square_from_name)?;
    let to = std::str::from_utf8(&bytes[2..4])
        .ok()
        .and_then(square_from_name)?;
    let promotion = if bytes.len() == 5 {
        Some(promotion_kind(bytes[4])?)
    } else {
        None
    };
    Some(UciMove {
        from,
        to,
        promotion,
    })
}

const fn promotion_kind(suffix: u8) -> Option<PieceKind> {
    match suffix {
        b'q' => Some(PieceKind::Queen),
        b'r' => Some(PieceKind::Rook),
        b'b' => Some(PieceKind::Bishop),
        b'n' => Some(PieceKind::Knight),
        _ => None,
    }
}

const fn promotion_suffix(kind: PieceKind) -> Option<char> {
    match kind {
        PieceKind::Queen => Some('q'),
        PieceKind::Rook => Some('r'),
        PieceKind::Bishop => Some('b'),
        PieceKind::Knight => Some('n'),
        PieceKind::Pawn | PieceKind::King => None,
    }
}

fn promotion_kind_for_action(action: ActionId) -> Option<PieceKind> {
    if action == action_id(PROMOTE_QUEEN) {
        Some(PieceKind::Queen)
    } else if action == action_id(PROMOTE_ROOK) {
        Some(PieceKind::Rook)
    } else if action == action_id(PROMOTE_BISHOP) {
        Some(PieceKind::Bishop)
    } else if action == action_id(PROMOTE_KNIGHT) {
        Some(PieceKind::Knight)
    } else {
        None
    }
}

fn square_label(square: usize, piece: Option<Piece>) -> String {
    let name = square_name(square);
    match piece {
        Some(piece) => format!("{name} {}", piece_label(piece)),
        None => name,
    }
}

const fn piece_label(piece: Piece) -> &'static str {
    match (piece.color, piece.kind) {
        (Color::White, PieceKind::King) => "K",
        (Color::White, PieceKind::Queen) => "Q",
        (Color::White, PieceKind::Rook) => "R",
        (Color::White, PieceKind::Bishop) => "B",
        (Color::White, PieceKind::Knight) => "N",
        (Color::White, PieceKind::Pawn) => "P",
        (Color::Black, PieceKind::King) => "k",
        (Color::Black, PieceKind::Queen) => "q",
        (Color::Black, PieceKind::Rook) => "r",
        (Color::Black, PieceKind::Bishop) => "b",
        (Color::Black, PieceKind::Knight) => "n",
        (Color::Black, PieceKind::Pawn) => "p",
    }
}

const fn piece_glyph(piece: Piece) -> Glyph {
    match (piece.color, piece.kind) {
        (Color::White, PieceKind::King) => Glyph::ChessWhiteKing,
        (Color::White, PieceKind::Queen) => Glyph::ChessWhiteQueen,
        (Color::White, PieceKind::Rook) => Glyph::ChessWhiteRook,
        (Color::White, PieceKind::Bishop) => Glyph::ChessWhiteBishop,
        (Color::White, PieceKind::Knight) => Glyph::ChessWhiteKnight,
        (Color::White, PieceKind::Pawn) => Glyph::ChessWhitePawn,
        (Color::Black, PieceKind::King) => Glyph::ChessBlackKing,
        (Color::Black, PieceKind::Queen) => Glyph::ChessBlackQueen,
        (Color::Black, PieceKind::Rook) => Glyph::ChessBlackRook,
        (Color::Black, PieceKind::Bishop) => Glyph::ChessBlackBishop,
        (Color::Black, PieceKind::Knight) => Glyph::ChessBlackKnight,
        (Color::Black, PieceKind::Pawn) => Glyph::ChessBlackPawn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kobo_sdk::{AppRunner, Chrome, Command, DisplayMetrics, StoreRequest};

    const PROMOTION_PUZZLES: &[u8] = include_bytes!("../examples/promotion-puzzles.json");
    const ALT_PUZZLES: &[u8] = br#"{
      "version": 1,
      "title": "Endgames",
      "puzzles": [
        {
          "id": "end-1",
          "fen": "7k/8/5KQ1/8/8/8/8/8 w - - 0 1",
          "description": "Endgame one.",
          "solution": ["g6g7"]
        },
        {
          "id": "end-2",
          "fen": "8/8/8/8/8/5kq1/8/7K b - - 0 1",
          "description": "Endgame two.",
          "solution": ["g3g2"]
        }
      ]
    }"#;

    fn new_runner() -> AppRunner<ChessBoardApp> {
        AppRunner::with_metrics(
            ChessBoardApp::default(),
            DisplayMetrics {
                width: 1264,
                height: 1680,
                pixels_per_inch: 300,
                ..DisplayMetrics::default()
            },
        )
    }

    fn runner(bytes: Option<Vec<u8>>) -> (AppRunner<ChessBoardApp>, Vec<Command>) {
        match bytes {
            Some(bytes) => runner_with_collections(vec![(PUZZLES_FILE, bytes)]),
            None => {
                let mut runner = new_runner();
                let mut commands = runner.start();
                assert!(commands
                    .iter()
                    .any(|command| matches!(command, Command::Store(StoreRequest::List))));
                assert!(commands.iter().any(|command| matches!(
                    command,
                    Command::Store(StoreRequest::Load { key }) if key == PROGRESS_KEY
                )));

                let next = runner.store_result(StoreResult::Keys(Vec::new()));
                assert!(next.iter().any(|command| matches!(
                    command,
                    Command::Store(StoreRequest::Save { key, value })
                        if key == PUZZLES_FILE && value.as_slice() == EXAMPLE_PUZZLES
                )));
                commands.extend(next);

                let next = runner.store_result(StoreResult::Loaded {
                    key: PROGRESS_KEY.into(),
                    value: None,
                });
                commands.extend(next);

                let next = runner.store_result(StoreResult::Saved {
                    key: PUZZLES_FILE.into(),
                });
                assert!(next
                    .iter()
                    .any(|command| matches!(command, Command::Store(StoreRequest::List))));
                commands.extend(next);

                let next = runner.store_result(StoreResult::Keys(vec![PUZZLES_FILE.to_owned()]));
                commands.extend(next);

                let next = runner.store_result(StoreResult::Loaded {
                    key: PUZZLES_FILE.into(),
                    value: Some(EXAMPLE_PUZZLES.to_vec()),
                });
                commands.extend(next);

                let next = runner.store_result(StoreResult::Loaded {
                    key: PUZZLES_FILE.into(),
                    value: Some(EXAMPLE_PUZZLES.to_vec()),
                });
                commands.extend(next);

                (runner, commands)
            }
        }
    }

    fn runner_with_collections(
        files: Vec<(&str, Vec<u8>)>,
    ) -> (AppRunner<ChessBoardApp>, Vec<Command>) {
        runner_with_state(files, None)
    }

    fn runner_with_state(
        files: Vec<(&str, Vec<u8>)>,
        progress: Option<Vec<u8>>,
    ) -> (AppRunner<ChessBoardApp>, Vec<Command>) {
        let mut runner = new_runner();
        let mut commands = runner.start();
        assert!(commands
            .iter()
            .any(|command| matches!(command, Command::Store(StoreRequest::List))));
        assert!(commands.iter().any(|command| matches!(
            command,
            Command::Store(StoreRequest::Load { key }) if key == PROGRESS_KEY
        )));

        let keys = files.iter().map(|(key, _)| (*key).to_owned()).collect();
        let next = runner.store_result(StoreResult::Keys(keys));
        commands.extend(next);

        let next = runner.store_result(StoreResult::Loaded {
            key: PROGRESS_KEY.into(),
            value: progress,
        });
        commands.extend(next);

        let mut metadata_keys = files
            .iter()
            .map(|(key, _)| (*key).to_owned())
            .filter(|key| is_puzzle_collection_key(key))
            .collect::<Vec<_>>();
        metadata_keys.sort();
        metadata_keys.dedup();

        for key in metadata_keys {
            let bytes = files
                .iter()
                .find(|(candidate, _)| *candidate == key)
                .map(|(_, bytes)| bytes.clone())
                .expect("metadata key has bytes");
            let next = runner.store_result(StoreResult::Loaded {
                key,
                value: Some(bytes),
            });
            commands.extend(next);
        }

        if let Some(CollectionLoad {
            key,
            kind: CollectionLoadKind::Initial,
        }) = runner.app().collection_load.clone()
        {
            let bytes = files
                .iter()
                .find(|(candidate, _)| *candidate == key)
                .map(|(_, bytes)| bytes.clone())
                .expect("initial collection has bytes");
            let next = runner.store_result(StoreResult::Loaded {
                key,
                value: Some(bytes),
            });
            commands.extend(next);
        }

        (runner, commands)
    }

    fn progress_bytes(commands: &[Command]) -> Option<Vec<u8>> {
        commands.iter().find_map(|command| match command {
            Command::Store(StoreRequest::Save { key, value }) if key == PROGRESS_KEY => {
                Some(value.clone())
            }
            _ => None,
        })
    }

    fn ack_progress_save(runner: &mut AppRunner<ChessBoardApp>) -> Vec<Command> {
        if runner.app().progress_saving.is_some() {
            runner.store_result(StoreResult::Saved {
                key: PROGRESS_KEY.into(),
            })
        } else {
            Vec::new()
        }
    }

    fn switch_collection(
        runner: &mut AppRunner<ChessBoardApp>,
        key: &str,
        bytes: &[u8],
    ) -> Vec<Command> {
        if runner.app().progress_saving.is_some() {
            ack_progress_save(runner);
        }
        let index = runner
            .app()
            .collections
            .iter()
            .position(|entry| entry.key == key)
            .expect("collection is discovered");
        runner.action(action_id(PUZZLE_PICKER));
        let mut commands = runner.action(action_id(&collection_action(index)));
        assert!(commands.iter().any(|command| matches!(
            command,
            Command::Store(StoreRequest::Load { key: requested }) if requested == key
        )));
        commands.extend(runner.store_result(StoreResult::Loaded {
            key: key.to_owned(),
            value: Some(bytes.to_vec()),
        }));
        commands
    }

    fn play(runner: &mut AppRunner<ChessBoardApp>, movement: &str) -> Vec<Command> {
        let movement = parse_uci_move(movement).expect("valid test UCI");
        let mut commands = runner.action(action_id(&square_action(movement.from)));
        commands.extend(runner.action(action_id(&square_action(movement.to))));
        if let Some(kind) = movement.promotion {
            commands.extend(runner.action(action_id(promotion_action(kind))));
        }
        commands
    }

    fn promotion_action(kind: PieceKind) -> &'static str {
        match kind {
            PieceKind::Queen => PROMOTE_QUEEN,
            PieceKind::Rook => PROMOTE_ROOK,
            PieceKind::Bishop => PROMOTE_BISHOP,
            PieceKind::Knight => PROMOTE_KNIGHT,
            PieceKind::Pawn | PieceKind::King => panic!("not a promotion piece"),
        }
    }

    fn select_with_range_reply(
        runner: &mut AppRunner<ChessBoardApp>,
        key: &str,
        data: &[u8],
        response_error: Option<kobo_sdk::StoreError>,
    ) -> Vec<Command> {
        let index = runner
            .app()
            .collections
            .iter()
            .position(|entry| entry.key == key)
            .unwrap();
        runner.action(action_id(PUZZLE_PICKER));
        let commands = runner.action(action_id(&collection_action(index)));
        assert!(commands.iter().any(|command| matches!(command,
            Command::Store(StoreRequest::Load { key: name }) if name == key)));
        let mut pending = runner.store_result(StoreResult::Denied(kobo_sdk::StoreError::TooFull));
        let mut seen = 0;
        let mut issued = Vec::new();
        while !pending.is_empty() {
            let mut next = Vec::new();
            for command in pending {
                if let Command::Store(StoreRequest::ShelfRead {
                    name,
                    offset,
                    length,
                }) = &command
                {
                    seen += 1;
                    assert!(seen <= 258, "two passes of 8 MiB at 64 KiB each");
                    assert_eq!(name, &format!("{}{}", large_collection::READ_PREFIX, key));
                    assert!(*length as usize <= large_collection::READ_CHUNK);
                    let answer = match response_error {
                        Some(error) => StoreResult::Denied(error),
                        None => {
                            let from = *offset as usize;
                            let end = (from + *length as usize).min(data.len());
                            StoreResult::ShelfRead {
                                name: name.clone(),
                                offset: *offset,
                                bytes: data[from..end].to_vec(),
                                size: data.len() as u32,
                            }
                        }
                    };
                    next.extend(runner.store_result(answer));
                }
                issued.push(command);
            }
            pending = next;
        }
        assert!(seen > 0, "large file uses range transport");
        issued
    }

    #[test]
    fn selected_large_collection_is_fully_validated_before_atomic_activation() {
        let collection = serde_json::json!({
            "version": 1, "title": "Large book",
            "puzzles": [{
                "id": "large-1",
                "fen": "7k/8/5KQ1/8/8/8/8/8 w - - 0 1",
                "description": "x".repeat(1024 * 1024),
                "solution": ["g6g7"]
            }]
        });
        let bytes = serde_json::to_vec(&collection).unwrap();
        assert!(bytes.len() > 256 * 1024);
        let key = "puzzles-large.json";
        let (mut runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
            (key, ALT_PUZZLES.to_vec()),
        ]);
        let commands = select_with_range_reply(&mut runner, key, &bytes, None);
        assert_eq!(runner.app().active_collection.as_deref(), Some(key));
        assert_eq!(runner.app().puzzles[0].id, "large-1");
        assert!(commands.iter().any(|command| matches!(command,
            Command::Store(StoreRequest::Save { key, .. }) if key == PROGRESS_KEY)));
    }

    #[test]
    fn failed_large_selection_never_changes_board_file_or_progress() {
        let key = "puzzles-large.json";
        for payload in [vec![0xff; 256 * 1024 + 1], vec![b'{'; 256 * 1024 + 1]] {
            let (mut runner, _) = runner_with_collections(vec![
                (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
                (key, ALT_PUZZLES.to_vec()),
            ]);
            let old_board = runner.app().board.clone();
            let old_progress = runner.app().progress.clone();
            let commands = select_with_range_reply(&mut runner, key, &payload, None);
            assert_eq!(runner.app().board, old_board);
            assert_eq!(runner.app().progress, old_progress);
            assert_eq!(
                runner.app().active_collection.as_deref(),
                Some(PUZZLES_FILE)
            );
            assert!(runner.app().file_error.as_deref().unwrap().contains(key));
            assert!(!commands.iter().any(|command| matches!(command,
                Command::Store(StoreRequest::Save { key, .. }) if key == PROGRESS_KEY)));
        }

        let (mut runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
            (key, ALT_PUZZLES.to_vec()),
        ]);
        let old_board = runner.app().board.clone();
        let old_progress = runner.app().progress.clone();
        select_with_range_reply(
            &mut runner,
            key,
            ALT_PUZZLES,
            Some(kobo_sdk::StoreError::Missing),
        );
        assert_eq!(runner.app().board, old_board);
        assert_eq!(runner.app().progress, old_progress);
        assert_eq!(
            runner.app().active_collection.as_deref(),
            Some(PUZZLES_FILE)
        );
        assert!(runner
            .app()
            .file_error
            .as_deref()
            .unwrap()
            .contains("runtime lacks"));
    }

    #[test]
    fn flipped_display_reverses_board_but_not_uci_coordinates() {
        assert_eq!(board_square(0, false), 0);
        assert_eq!(board_square(CELLS - 1, false), CELLS - 1);
        assert_eq!(board_square(0, true), CELLS - 1);
        assert_eq!(board_square(CELLS - 1, true), 0);
        assert_eq!(square_label(0, None), "a8");
        assert_eq!(square_label(CELLS - 1, None), "h1");

        let movement = parse_uci_move("e2e4").unwrap();
        assert_eq!(
            uci_move(movement.from, movement.to, movement.promotion),
            "e2e4"
        );
        let flipped_from = (0..CELLS)
            .find(|&display| board_square(display, true) == movement.from)
            .unwrap();
        let flipped_to = (0..CELLS)
            .find(|&display| board_square(display, true) == movement.to)
            .unwrap();
        assert_ne!(flipped_from, movement.from);
        assert_ne!(flipped_to, movement.to);
        assert_eq!(
            uci_move(
                board_square(flipped_from, true),
                board_square(flipped_to, true),
                None
            ),
            "e2e4"
        );
    }

    #[test]
    fn promotion_uci_supports_all_standard_suffixes() {
        let from = square_from_name("a7").unwrap();
        let to = square_from_name("a8").unwrap();
        for (kind, expected) in [
            (PieceKind::Queen, "a7a8q"),
            (PieceKind::Rook, "a7a8r"),
            (PieceKind::Bishop, "a7a8b"),
            (PieceKind::Knight, "a7a8n"),
        ] {
            assert_eq!(uci_move(from, to, Some(kind)), expected);
            assert_eq!(
                parse_uci_move(expected),
                Some(UciMove {
                    from,
                    to,
                    promotion: Some(kind),
                })
            );
        }
    }

    #[test]
    fn wrong_move_is_rejected_without_advancing_solution() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let initial = runner.app().board.clone();

        play(&mut runner, "d7d8");

        assert_eq!(runner.app().board, initial);
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Wrong)
        );
    }

    #[test]
    fn description_is_revealed_below_controls_only_after_completing_solution() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        runner.app_mut().select_puzzle(2);
        let description = runner.app().puzzles[2].description.clone().unwrap();
        let has_description = |app: &ChessBoardApp| {
            app.screen().nodes.iter().any(
                |node| matches!(node, kobo_sdk::Node::Text { text, .. } if text == &description),
            )
        };
        assert!(!has_description(runner.app()));
        play(&mut runner, "e2e5");
        assert!(!has_description(runner.app()));
        play(&mut runner, "e2e6");
        assert!(!has_description(runner.app()));
        play(&mut runner, "e6f7");
        assert!(has_description(runner.app()));

        let screen = runner.app().screen();
        let diagnostics =
            screen.diagnostics(&runner.context().metrics(), &Chrome::measuring(false));
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.issues);
        let text = diagnostics
            .layout
            .nodes
            .iter()
            .find(|node| {
                node.kind == kobo_sdk::LayoutKind::Text
                    && node.text_lines.first()
                        == description.lines().next().map(str::to_owned).as_ref()
            })
            .expect("description is laid out");
        assert_eq!(text.text_lines.len(), 3);
        for action in [MODE_TOGGLE, RESET, FLIP] {
            let control = diagnostics
                .layout
                .nodes
                .iter()
                .find(|node| node.kind.acts_on() == Some(action_id(action)))
                .expect("toolbar control");
            assert!(text.rect.y >= control.rect.y + control.rect.height);
        }

        runner.action(action_id(MODE_TOGGLE));
        assert!(!has_description(runner.app()));
        runner.action(action_id(MODE_TOGGLE));
        play(&mut runner, "e2e6");
        play(&mut runner, "e6f7");
        assert!(has_description(runner.app()));
        runner.action(action_id(RESET));
        assert!(runner.app().current_is_solved());
        assert!(!has_description(runner.app()));
        play(&mut runner, "e2e6");
        play(&mut runner, "e6f7");
        runner.page_turn(true);
        assert!(!has_description(runner.app()));
    }


    #[test]
    fn visible_analysis_has_selected_state_and_preserves_the_original_board() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let metrics = runner.context().metrics();
        let before = runner.app().screen().diagnostics(&metrics, &Chrome::measuring(false));
        let old_board = before.layout.nodes.iter()
            .find(|node| matches!(node.kind, kobo_sdk::LayoutKind::ChessFrame { .. }))
            .unwrap().rect;
        for name in [ANALYSIS_TOGGLE, MODE_TOGGLE, ORIENTATION_LOCK, RESET, FLIP] {
            assert!(before.layout.nodes.iter().any(|node| node.kind.acts_on() == Some(action_id(name))));
        }
        assert!(!before.layout.nodes.iter().any(|node| {
            node.kind.acts_on() == Some(action_id("description-toggle"))
        }));
        runner.action(action_id(ANALYSIS_TOGGLE));
        assert!(runner.app().analysis_browser.is_open());
        let opened = runner.app().screen();
        let layout = opened.diagnostics(&metrics, &Chrome::measuring(false));
        assert_eq!(layout.layout.nodes.iter()
            .find(|node| matches!(node.kind, kobo_sdk::LayoutKind::ChessFrame { .. }))
            .unwrap().rect, old_board);
        assert!(layout.layout.nodes.iter().any(|node| matches!(
            node.kind, kobo_sdk::LayoutKind::Cell(id, _, true) if id == action_id(ANALYSIS_TOGGLE)
        )));
        assert!(opened.nodes.iter().any(|node| matches!(node, kobo_sdk::Node::RichText { .. })));
        for name in [ANALYSIS_PREVIOUS, ANALYSIS_NEXT, ANALYSIS_CLOSE] {
            assert!(layout.layout.nodes.iter().any(|node| node.kind.acts_on() == Some(action_id(name))));
        }
        runner.action(action_id(ANALYSIS_CLOSE));
        assert!(!runner.app().analysis_browser.is_open());
        let closed = runner.app().screen().diagnostics(&metrics, &Chrome::measuring(false));
        assert_eq!(closed.layout.nodes.iter()
            .find(|node| matches!(node.kind, kobo_sdk::LayoutKind::ChessFrame { .. }))
            .unwrap().rect, old_board);
        assert!(closed.layout.nodes.iter().any(|node| matches!(
            node.kind, kobo_sdk::LayoutKind::Cell(id, _, false) if id == action_id(ANALYSIS_TOGGLE)
        )));
    }

    #[test]
    fn blank_analysis_is_disabled_and_legacy_description_has_its_own_pane() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        runner.app_mut().puzzles[0].analysis = None;
        runner.app_mut().puzzles[0].description = None;
        runner.app_mut().puzzles[0].description_content.clear();
        let d = runner.app().screen().diagnostics(&runner.context().metrics(), &Chrome::measuring(false));
        assert!(d.layout.nodes.iter().any(|node| matches!(
            node.kind, kobo_sdk::LayoutKind::Button(id, ControlState::Disabled, _)
            if id == action_id(ANALYSIS_TOGGLE)
        )));
        runner.action(action_id(ANALYSIS_TOGGLE));
        assert!(!runner.app().analysis_browser.is_open());
        runner.app_mut().puzzles[0].description_content =
            vec![crate::analysis::ContentSpan::Text("Legacy description".into())];
        runner.action(action_id(ANALYSIS_TOGGLE));
        assert!(runner.app().analysis_browser.is_open());
        assert!(runner.app().analysis_browser.preview_board().is_none());
        assert!(runner.app().screen().nodes.iter().any(|node| matches!(
            node, kobo_sdk::Node::RichText { text, .. } if text.contains("Legacy description")
        )));
        runner.action(action_id(ANALYSIS_CLOSE));
        assert!(!runner.app().analysis_browser.is_open());
    }

    #[test]
    fn rich_first_middle_last_pages_have_bounded_links_and_do_not_grade_moves() {
        let (mut runner, _) = runner(Some(RICH_PUZZLES.to_vec()));
        let before = attempt_snapshot(runner.app());
        let document = project_analysis(&runner.app().puzzles[0]);
        let pages = paginate(&document);
        runner.action(action_id(ANALYSIS_TOGGLE));
        assert!(runner.app().analysis_browser.is_open());
        let metrics = runner.context().metrics();
        let libra_h2o = DisplayMetrics {
            width: 1080,
            height: 1440,
            pixels_per_inch: 265,
            ..DisplayMetrics::default()
        };
        for destination in [0, pages.len() / 2, pages.len() - 1] {
            while runner.app().analysis_browser.page() < destination {
                runner.action(action_id(ANALYSIS_NEXT));
            }
            while runner.app().analysis_browser.page() > destination {
                runner.action(action_id(ANALYSIS_PREVIOUS));
            }
            let screen = runner.app().screen();
            for metrics in [metrics, libra_h2o] {
                let diagnostics = screen.diagnostics(&metrics, &Chrome::measuring(false));
                assert!(!diagnostics.has_errors(), "page {destination}, {metrics:?}: {:?}", diagnostics.issues);
            }
            assert!(screen.nodes.len() <= 512);
            for node in &screen.nodes {
                if let kobo_sdk::Node::RichText { spans, links, .. } = node {
                    assert!(spans.len() <= 256 && links.len() <= 16);
                    if let Some(link) = links.first() {
                        assert!(node_as_rich_text_has_utf8_boundaries(node, link.start, link.end));
                    }
                }
            }
        }
        let (run_index, target) = document.runs.iter().enumerate()
            .find_map(|(index, run)| run.target.map(|target| (index, target))).unwrap();
        let target_page = pages.iter().position(|page| page.fragments.iter().any(|fragment| {
            fragment.source_run == run_index && fragment.target == Some(target)
        })).unwrap();
        while runner.app().analysis_browser.page() > target_page {
            runner.action(action_id(ANALYSIS_PREVIOUS));
        }
        while runner.app().analysis_browser.page() < target_page {
            runner.action(action_id(ANALYSIS_NEXT));
        }
        runner.action(action_id(&link_action(run_index)));
        assert_eq!(runner.app().analysis_browser.selected_node(), Some(target));
        assert!(runner.app().screen().nodes.iter().any(|node| matches!(
            node, kobo_sdk::Node::RichText { spans, .. }
                if spans.iter().any(|span| span.presentation.highlighted)
        )));
        runner.action(action_id(ANALYSIS_CLOSE));
        assert_eq!(attempt_snapshot(runner.app()), before);
    }

    fn node_as_rich_text_has_utf8_boundaries(node: &kobo_sdk::Node, start: usize, end: usize) -> bool {
        matches!(node, kobo_sdk::Node::RichText { text, .. }
            if start < end && end <= text.len()
                && text.is_char_boundary(start) && text.is_char_boundary(end))
    }

    #[test]
    fn note_control_is_hidden_but_description_and_progress_survive() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let description = runner.app().puzzles[0].description.clone().unwrap();
        let initial_board = runner.app().board.clone();
        let initial_generation = runner.app().progress_generation;
        let preface = crate::presentation::analysis_text(&runner.app().puzzles[0]);
        assert!(preface.available());
        assert_eq!(
            preface.preface,
            &[crate::analysis::ContentSpan::Text(description.clone())]
        );
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().progress_generation, initial_generation);
        assert_eq!(runner.app().board, initial_board);

        let screen = runner.app().screen();
        let diagnostics =
            screen.diagnostics(&runner.context().metrics(), &Chrome::measuring(false));
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.issues);
        assert!(!diagnostics
            .layout
            .nodes
            .iter()
            .any(|node| { node.kind.acts_on() == Some(action_id("description-toggle")) }));
        assert!(
            !screen.nodes.iter().any(
                |node| matches!(node, kobo_sdk::Node::Text { text, .. } if text == &description),
            )
        );

        play(&mut runner, "d7e8");
        assert!(
            runner.app().screen().nodes.iter().any(
                |node| matches!(node, kobo_sdk::Node::Text { text, .. } if text == &description),
            )
        );
        assert!(runner.app().current_is_solved());
        runner.action(action_id(RESET));
        assert!(runner.app().current_is_solved());
        assert!(!runner.app().description_visible);
        runner.action(action_id(MODE_TOGGLE));
        assert_eq!(runner.app().mode, BoardMode::FreeBoard);
        assert!(!runner.app().description_visible);
    }

    #[test]
    fn topic_replaces_side_to_move_in_solution_status_not_free_board() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        runner.app_mut().puzzles[0].topic = Some("Мат в один ход\nEndgame".into());
        let status = format!("{:?}", runner.app().screen().nodes);
        assert!(status.contains("Мат в один ход"));
        assert!(!status.contains("White to move"));
        assert!(!status.contains("Black to move"));

        runner.action(action_id(MODE_TOGGLE));
        let status = format!("{:?}", runner.app().screen().nodes);
        assert!(status.contains("Free board"));
        assert!(!status.contains("Мат в один ход"));
    }

    #[test]
    fn completed_puzzles_allow_missing_or_blank_descriptions() {
        for description in [None, Some(String::new()), Some(" \n ".into())] {
            let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
            let puzzle = &mut runner.app_mut().puzzles[0];
            puzzle.description = description.clone();
            puzzle.description_content = description
                .into_iter()
                .map(crate::analysis::ContentSpan::Text)
                .collect();
            play(&mut runner, "d7e8");
            assert!(!runner
                .app()
                .screen()
                .nodes
                .iter()
                .any(|node| { matches!(node, kobo_sdk::Node::Text { .. }) }));
        }
    }

    #[test]
    fn one_move_solution_completes_and_locks_board() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));

        play(&mut runner, "d7e8");

        assert_eq!(runner.app().solution_ply, 1);
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Complete)
        );
        let solved = runner.app().board.clone();
        let e8 = square_from_name("e8").unwrap();
        runner.action(action_id(&square_action(e8)));
        assert_eq!(runner.app().board, solved);
    }

    #[test]
    fn mate_in_two_auto_plays_reply_then_completes_and_resets() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        runner.page_turn(true);
        runner.page_turn(true);
        assert_eq!(runner.app().puzzle_index, 2);

        let initial = runner.app().board.clone();
        play(&mut runner, "e2e6");

        assert_eq!(runner.app().solution_ply, 2);
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Correct)
        );
        assert!(runner
            .app()
            .board
            .piece_at(square_from_name("f7").unwrap())
            .is_none());
        assert_eq!(
            runner.app().board.piece_at(square_from_name("f8").unwrap()),
            Some(Piece {
                color: Color::Black,
                kind: PieceKind::King,
            })
        );

        play(&mut runner, "e6f7");
        assert_eq!(runner.app().solution_ply, 3);
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Complete)
        );

        runner.action(action_id(RESET));
        assert_eq!(runner.app().board, initial);
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().solution_feedback, None);
    }

    #[test]
    fn cancelling_promotion_leaves_board_and_attempt_unchanged() {
        let (mut runner, _) = runner(Some(PROMOTION_PUZZLES.to_vec()));
        let initial = runner.app().board.clone();

        let movement = parse_uci_move("a7a8q").unwrap();
        runner.action(action_id(&square_action(movement.from)));
        runner.action(action_id(&square_action(movement.to)));
        assert!(runner.app().pending_promotion.is_some());
        assert_eq!(
            runner.app().board.piece_at(movement.from),
            initial.piece_at(movement.from)
        );
        assert_eq!(
            runner.app().board.piece_at(movement.to),
            initial.piece_at(movement.to)
        );

        runner.action(action_id(PROMOTION_CANCEL));

        assert_eq!(runner.app().board, initial);
        assert!(runner.app().pending_promotion.is_none());
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().solution_feedback, None);
    }

    #[test]
    fn wrong_underpromotion_is_rejected_and_correct_one_completes() {
        let (mut runner, _) = runner(Some(PROMOTION_PUZZLES.to_vec()));
        runner.page_turn(true);
        let initial = runner.app().board.clone();

        let movement = parse_uci_move("b7b8n").unwrap();
        runner.action(action_id(&square_action(movement.from)));
        runner.action(action_id(&square_action(movement.to)));
        runner.action(action_id(PROMOTE_QUEEN));

        assert_eq!(runner.app().board, initial);
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Wrong)
        );

        play(&mut runner, "b7b8n");
        assert_eq!(
            runner.app().board.piece_at(square_from_name("b8").unwrap()),
            Some(Piece {
                color: Color::White,
                kind: PieceKind::Knight,
            })
        );
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Complete)
        );
    }

    #[test]
    fn black_pawn_can_promote_to_an_underpromotion_piece() {
        let (mut runner, _) = runner(Some(PROMOTION_PUZZLES.to_vec()));
        runner.page_turn(true);
        runner.page_turn(true);

        play(&mut runner, "h2h1r");

        assert_eq!(
            runner.app().board.piece_at(square_from_name("h1").unwrap()),
            Some(Piece {
                color: Color::Black,
                kind: PieceKind::Rook,
            })
        );
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Complete)
        );
    }

    #[test]
    fn automatic_opponent_promotion_applies_without_opening_modal() {
        let (mut runner, _) = runner(Some(PROMOTION_PUZZLES.to_vec()));
        for _ in 0..3 {
            runner.page_turn(true);
        }

        play(&mut runner, "h2h3");

        assert!(runner.app().pending_promotion.is_none());
        assert_eq!(runner.app().solution_ply, 2);
        assert_eq!(
            runner.app().board.piece_at(square_from_name("a1").unwrap()),
            Some(Piece {
                color: Color::Black,
                kind: PieceKind::Queen,
            })
        );
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Correct)
        );
    }

    #[test]
    fn changing_puzzles_clears_transient_solution_feedback_and_promotion() {
        let (mut runner, _) = runner(Some(PROMOTION_PUZZLES.to_vec()));
        let movement = parse_uci_move("a7a8q").unwrap();
        runner.action(action_id(&square_action(movement.from)));
        runner.action(action_id(&square_action(movement.to)));
        assert!(runner.app().pending_promotion.is_some());

        runner.action(action_id(PROMOTION_CANCEL));
        play(&mut runner, "a7a8q");
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Complete)
        );

        runner.page_turn(true);

        assert_eq!(runner.app().puzzle_index, 1);
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().solution_feedback, None);
        assert!(runner.app().pending_promotion.is_none());
    }

    #[test]
    fn free_board_accepts_wrong_moves_without_advancing_solution() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let initial = runner.app().board.clone();

        runner.action(action_id(MODE_TOGGLE));
        assert_eq!(runner.app().mode, BoardMode::FreeBoard);
        play(&mut runner, "d7d8");

        assert_ne!(runner.app().board, initial);
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().solution_feedback, None);
    }

    #[test]
    fn entering_free_board_keeps_the_current_solved_line_position() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        runner.page_turn(true);
        runner.page_turn(true);
        play(&mut runner, "e2e6");
        assert_eq!(runner.app().solution_ply, 2);
        let current = runner.app().board.clone();

        runner.action(action_id(MODE_TOGGLE));

        assert_eq!(runner.app().mode, BoardMode::FreeBoard);
        assert_eq!(runner.app().board, current);
        assert_eq!(runner.app().solution_ply, 2);
        assert_eq!(runner.app().solution_feedback, None);

        play(&mut runner, "e6e5");
        assert_eq!(runner.app().solution_ply, 2);
    }

    #[test]
    fn returning_to_solution_resets_position_attempt_and_keeps_flip() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let initial = runner.app().board.clone();

        runner.action(action_id(MODE_TOGGLE));
        play(&mut runner, "d7d8");
        runner.action(action_id(FLIP));
        assert!(runner.app().flipped);

        runner.action(action_id(MODE_TOGGLE));

        assert_eq!(runner.app().mode, BoardMode::Solution);
        assert_eq!(runner.app().board, initial);
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().solution_feedback, None);
        assert!(runner.app().flipped);
    }

    #[test]
    fn reset_and_navigation_preserve_free_board_mode() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        runner.action(action_id(MODE_TOGGLE));
        let initial = runner.app().board.clone();

        play(&mut runner, "d7d8");
        assert_ne!(runner.app().board, initial);

        runner.action(action_id(RESET));
        assert_eq!(runner.app().board, initial);
        assert_eq!(runner.app().mode, BoardMode::FreeBoard);

        runner.page_turn(true);
        assert_eq!(runner.app().puzzle_index, 1);
        assert_eq!(runner.app().mode, BoardMode::FreeBoard);
        runner.page_turn(false);
        assert_eq!(runner.app().puzzle_index, 0);
        assert_eq!(runner.app().mode, BoardMode::FreeBoard);
    }

    #[test]
    fn promotion_works_without_grading_in_free_board() {
        let (mut runner, _) = runner(Some(PROMOTION_PUZZLES.to_vec()));
        runner.action(action_id(MODE_TOGGLE));

        play(&mut runner, "a7a8r");

        assert_eq!(runner.app().mode, BoardMode::FreeBoard);
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().solution_feedback, None);
        assert_eq!(
            runner.app().board.piece_at(square_from_name("a8").unwrap()),
            Some(Piece {
                color: Color::White,
                kind: PieceKind::Rook,
            })
        );
    }

    #[test]
    fn orientation_lock_keeps_manual_orientation_across_puzzles_and_collections() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        assert!(!runner.app().flipped);
        runner.action(action_id(ORIENTATION_LOCK));
        assert!(runner.app().orientation_locked);
        runner.page_turn(true);
        assert_eq!(runner.app().puzzles[1].side_to_move(), Color::Black);
        assert!(!runner.app().flipped, "lock keeps White at the bottom");

        let screen = runner.app().screen();
        let diagnostics =
            screen.diagnostics(&runner.context().metrics(), &Chrome::measuring(false));
        assert!(!diagnostics.has_errors(), "{:?}", diagnostics.issues);
        assert!(diagnostics.layout.nodes.iter().any(|node| {
            matches!(node.kind, kobo_sdk::LayoutKind::Cell(action, _, true)
                if action == action_id(ORIENTATION_LOCK))
        }));

        runner.action(action_id(FLIP));
        assert!(runner.app().flipped, "manual flip works while locked");
        runner.page_turn(true);
        assert_eq!(runner.app().puzzles[2].side_to_move(), Color::White);
        assert!(
            runner.app().flipped,
            "lock keeps the manually flipped orientation"
        );
        runner.action(action_id(RESET));
        runner.action(action_id(MODE_TOGGLE));
        runner.action(action_id(MODE_TOGGLE));
        assert!(runner.app().orientation_locked);
        assert!(runner.app().flipped);

        runner.app_mut().activate_collection(
            Some("puzzles-endgames.json".into()),
            parse_puzzle_file(ALT_PUZZLES).unwrap(),
        );
        assert!(runner.app().orientation_locked);
        assert!(
            runner.app().flipped,
            "collection changes also keep orientation"
        );
        runner.action(action_id(ORIENTATION_LOCK));
        assert!(!runner.app().orientation_locked);
        assert!(
            runner.app().flipped,
            "unlocking does not jump the current board"
        );
        runner.page_turn(true);
        runner.page_turn(false);
        assert!(
            !runner.app().flipped,
            "automatic orientation resumes on navigation"
        );
    }

    #[test]
    fn puzzle_navigation_sets_orientation_and_keeps_board_controls() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        assert_eq!(runner.app().puzzles.len(), 10);
        assert!(!runner.app().flipped);
        let initial = runner.app().board.clone();
        let occupied = (0..CELLS)
            .find(|&square| initial.piece_at(square).is_some())
            .unwrap();
        runner.action(action_id(&square_action(occupied)));
        assert_ne!(runner.app().board, initial);
        runner.action(action_id(FLIP));
        assert!(runner.app().flipped);
        runner.action(action_id(RESET));
        assert_eq!(runner.app().board, initial);
        assert!(runner.app().flipped, "reset should preserve a manual flip");
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().solution_feedback, None);
        runner.page_turn(true);
        assert_eq!(runner.app().puzzle_index, 1);
        assert!(runner.app().flipped, "black puzzle should face black");
        runner.action(action_id(FLIP));
        assert!(!runner.app().flipped);
        runner.page_turn(false);
        assert_eq!(runner.app().puzzle_index, 0);
        assert!(!runner.app().flipped, "white puzzle should face white");
        runner.page_turn(false);
        assert_eq!(runner.app().puzzle_index, 0);
        for _ in 0..20 {
            runner.page_turn(true);
        }
        assert_eq!(runner.app().puzzle_index, 9);
        assert!(runner
            .action(action_id(EXIT))
            .iter()
            .any(|command| matches!(command, Command::Exit)));
    }

    #[test]
    fn startup_lists_collections_and_filters_keys_deterministically() {
        let mut runner = new_runner();
        let commands = runner.start();
        assert!(commands
            .iter()
            .any(|command| matches!(command, Command::Store(StoreRequest::List))));

        runner.store_result(StoreResult::Keys(vec![
            "notes.json".into(),
            "puzzles-z.json".into(),
            "puzzles-.json".into(),
            PUZZLES_FILE.into(),
            "puzzles-a.json".into(),
            "puzzles-a.json".into(),
        ]));

        assert_eq!(
            runner
                .app()
                .collections
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["puzzles-a.json", "puzzles-z.json", PUZZLES_FILE]
        );
    }

    #[test]
    fn missing_collections_create_and_load_default_examples() {
        let (runner, commands) = runner(None);
        assert!(commands.iter().any(|command| matches!(
            command, Command::Store(StoreRequest::Save { key, value })
                if key == PUZZLES_FILE && value.as_slice() == EXAMPLE_PUZZLES
        )));
        assert_eq!(
            runner.app().active_collection.as_deref(),
            Some(PUZZLES_FILE)
        );
        assert_eq!(runner.app().puzzles.len(), 10);
        assert!(runner.app().file_error.is_none());
    }

    #[test]
    fn invalid_only_collection_is_preserved_and_examples_are_used_in_memory() {
        let (runner, commands) = runner(Some(b"invalid JSON".to_vec()));
        assert!(runner.app().file_error.is_some());
        assert_eq!(runner.app().puzzles.len(), 10);
        assert!(runner.app().active_collection.is_none());
        assert_eq!(runner.app().collections.len(), 1);
        assert!(!runner.app().collections[0].valid);
        assert!(!commands
            .iter()
            .any(|command| matches!(command, Command::Store(StoreRequest::Save { .. }))));
    }

    #[test]
    fn picker_uses_title_and_switches_to_a_valid_collection() {
        let (mut runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
            ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
        ]);
        assert_eq!(
            runner.app().active_collection.as_deref(),
            Some(PUZZLES_FILE)
        );
        let initial_board = runner.app().board.clone();

        let index = runner
            .app()
            .collections
            .iter()
            .position(|entry| entry.key == "puzzles-endgames.json")
            .unwrap();
        assert_eq!(
            collection_display_name(&runner.app().collections[index]),
            "Endgames"
        );

        runner.action(action_id(PUZZLE_PICKER));
        assert!(runner.app().picker_open);
        let commands = runner.action(action_id(&collection_action(index)));
        assert!(!runner.app().picker_open);
        assert_eq!(runner.app().board, initial_board);
        assert!(commands.iter().any(|command| matches!(
            command,
            Command::Store(StoreRequest::Load { key }) if key == "puzzles-endgames.json"
        )));

        runner.store_result(StoreResult::Loaded {
            key: "puzzles-endgames.json".into(),
            value: Some(ALT_PUZZLES.to_vec()),
        });

        assert_eq!(
            runner.app().active_collection.as_deref(),
            Some("puzzles-endgames.json")
        );
        assert_eq!(runner.app().puzzle_index, 0);
        assert_eq!(runner.app().puzzles.len(), 2);
        assert_eq!(
            runner.app().puzzles[0].description.as_deref(),
            Some("Endgame one.")
        );

        runner.page_turn(true);
        assert_eq!(runner.app().puzzle_index, 1);
        runner.page_turn(true);
        assert_eq!(runner.app().puzzle_index, 1);
    }

    #[test]
    fn invalid_selected_collection_keeps_current_board_and_reports_its_name() {
        let broken = b"{ not valid json".to_vec();
        let (mut runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
            ("puzzles-broken.json", broken.clone()),
        ]);
        let before = runner.app().board.clone();
        let active = runner.app().active_collection.clone();
        let index = runner
            .app()
            .collections
            .iter()
            .position(|entry| entry.key == "puzzles-broken.json")
            .unwrap();

        runner.action(action_id(PUZZLE_PICKER));
        runner.action(action_id(&collection_action(index)));
        runner.store_result(StoreResult::Loaded {
            key: "puzzles-broken.json".into(),
            value: Some(broken),
        });

        assert_eq!(runner.app().active_collection, active);
        assert_eq!(runner.app().board, before);
        assert!(runner
            .app()
            .file_error
            .as_deref()
            .is_some_and(|error| error.contains("puzzles-broken.json")));
        assert!(!runner.app().collections[index].valid);
    }

    #[test]
    fn missing_progress_starts_clean_and_durable() {
        let (runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        assert!(runner.app().progress_ready);
        assert!(runner.app().progress_durable);
        assert!(runner.app().progress.files.is_empty());
        assert_eq!(runner.app().progress.active_file, None);
        assert!(runner.app().progress_warning.is_none());
    }

    #[test]
    fn completing_puzzle_marks_only_that_file_and_reset_free_board_keep_it_solved() {
        let (mut runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
            ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
        ]);
        let puzzle_id = runner.app().puzzles[0].id.clone();

        let commands = play(&mut runner, "d7e8");
        assert!(progress_bytes(&commands).is_some());
        assert!(runner.app().progress.is_solved(PUZZLES_FILE, &puzzle_id));
        assert!(!runner
            .app()
            .progress
            .is_solved("puzzles-endgames.json", &puzzle_id));
        let solved = runner
            .app()
            .progress
            .file(PUZZLES_FILE)
            .unwrap()
            .solved_ids
            .clone();

        ack_progress_save(&mut runner);
        runner.action(action_id(RESET));
        runner.action(action_id(MODE_TOGGLE));
        play(&mut runner, "d7d8");

        assert_eq!(
            runner.app().progress.file(PUZZLES_FILE).unwrap().solved_ids,
            solved
        );
        assert!(runner.app().current_is_solved());
    }

    #[test]
    fn switching_files_restores_each_files_remembered_puzzle() {
        let (mut runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
            ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
        ]);

        runner.page_turn(true);
        assert_eq!(runner.app().puzzle_index, 1);
        ack_progress_save(&mut runner);

        switch_collection(&mut runner, "puzzles-endgames.json", ALT_PUZZLES);
        assert_eq!(runner.app().puzzle_index, 0);
        ack_progress_save(&mut runner);

        runner.page_turn(true);
        assert_eq!(runner.app().puzzle_index, 1);
        ack_progress_save(&mut runner);

        switch_collection(&mut runner, PUZZLES_FILE, EXAMPLE_PUZZLES);
        assert_eq!(runner.app().puzzle_index, 1);
        ack_progress_save(&mut runner);

        switch_collection(&mut runner, "puzzles-endgames.json", ALT_PUZZLES);
        assert_eq!(runner.app().puzzle_index, 1);
    }

    #[test]
    fn restart_restores_active_file_current_puzzle_and_solved_state() {
        let (mut runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
            ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
        ]);

        play(&mut runner, "d7e8");
        ack_progress_save(&mut runner);
        switch_collection(&mut runner, "puzzles-endgames.json", ALT_PUZZLES);
        ack_progress_save(&mut runner);
        runner.page_turn(true);
        ack_progress_save(&mut runner);

        let bytes = runner.app().progress.to_bytes().unwrap();
        let (restored, _) = runner_with_state(
            vec![
                (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
                ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
            ],
            Some(bytes),
        );

        assert_eq!(
            restored.app().active_collection.as_deref(),
            Some("puzzles-endgames.json")
        );
        assert_eq!(restored.app().puzzle_index, 1);
        assert!(restored
            .app()
            .progress
            .is_solved(PUZZLES_FILE, "lichess-001cr"));
    }

    #[test]
    fn missing_remembered_file_and_puzzle_fall_back_safely() {
        let mut progress = Progress::new();
        progress.active_file = Some("puzzles-gone.json".into());
        progress.remember_puzzle(PUZZLES_FILE, "removed-puzzle");
        let (runner, _) = runner_with_state(
            vec![
                (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
                ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
            ],
            Some(progress.to_bytes().unwrap()),
        );
        assert_eq!(
            runner.app().active_collection.as_deref(),
            Some(PUZZLES_FILE)
        );
        assert_eq!(runner.app().puzzle_index, 0);

        let mut progress = Progress::new();
        progress.active_file = Some("puzzles-endgames.json".into());
        progress.remember_puzzle("puzzles-endgames.json", "removed-puzzle");
        let (runner, _) = runner_with_state(
            vec![
                (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
                ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
            ],
            Some(progress.to_bytes().unwrap()),
        );
        assert_eq!(
            runner.app().active_collection.as_deref(),
            Some("puzzles-endgames.json")
        );
        assert_eq!(runner.app().puzzle_index, 0);
    }

    #[test]
    fn corrupt_or_future_progress_is_preserved_without_writes() {
        for bytes in [
            b"not json".to_vec(),
            br#"{"version":2,"active_file":"puzzles.json","files":{}}"#.to_vec(),
        ] {
            let (mut runner, _) =
                runner_with_state(vec![(PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec())], Some(bytes));
            assert!(!runner.app().progress_durable);
            assert!(runner.app().progress_warning.is_some());

            let commands = runner.page_turn(true);
            assert!(progress_bytes(&commands).is_none());
            assert!(!runner.app().progress_dirty);
        }
    }

    #[test]
    fn save_failure_stays_dirty_warns_and_retries_on_next_mutation() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let commands = runner.page_turn(true);
        assert!(progress_bytes(&commands).is_some());
        assert!(runner.app().progress_saving.is_some());

        runner.store_result(StoreResult::Denied(kobo_sdk::StoreError::Unwritable));
        assert!(runner.app().progress_dirty);
        assert!(runner.app().progress_saving.is_none());
        assert_eq!(
            runner.app().progress_warning.as_deref(),
            Some("Progress not saved")
        );

        let retry = runner.page_turn(true);
        assert!(progress_bytes(&retry).is_some());
        assert!(runner.app().progress_saving.is_some());
    }

    #[test]
    fn solved_marker_and_progress_warning_fit_the_kobo_panel() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let id = runner.app().puzzles[0].id.clone();
        runner.app_mut().progress.mark_solved(PUZZLES_FILE, &id);
        runner.app_mut().progress_warning = Some("Progress not saved".into());

        let screen = runner.app().screen();
        let diagnostics =
            screen.diagnostics(&runner.context().metrics(), &Chrome::measuring(false));
        assert!(
            !diagnostics.has_errors(),
            "solved/warning screen: {:?}",
            diagnostics.issues
        );
        assert!(runner.app().current_is_solved());
    }

    #[test]
    fn puzzle_feedback_toolbar_and_promotion_modal_fit_the_kobo_panel() {
        let (mut layout_runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let feedback_states = [
            None,
            Some(SolutionFeedback::Correct),
            Some(SolutionFeedback::Wrong),
            Some(SolutionFeedback::Complete),
        ];
        for index in 0..layout_runner.app().puzzles.len() {
            layout_runner.app_mut().select_puzzle(index);
            for mode in [BoardMode::Solution, BoardMode::FreeBoard] {
                layout_runner.app_mut().mode = mode;
                for feedback in feedback_states {
                    layout_runner.app_mut().solution_feedback = feedback;
                    layout_runner.app_mut().description_visible =
                        feedback == Some(SolutionFeedback::Complete);
                    let screen = layout_runner.app().screen();
                    let diagnostics = screen.diagnostics(
                        &layout_runner.context().metrics(),
                        &Chrome::measuring(false),
                    );
                    assert!(
                        !diagnostics.has_errors(),
                        "puzzle {index}, mode {mode:?}, feedback {feedback:?}: {:?}",
                        diagnostics.issues
                    );
                    let feedback_mark =
                        diagnostics.layout.nodes.iter().find(|node| {
                            matches!(node.kind, kobo_sdk::LayoutKind::ChessFeedback(_))
                        });
                    match (mode, feedback) {
                        (BoardMode::Solution, Some(SolutionFeedback::Wrong)) => {
                            assert!(matches!(
                                feedback_mark.map(|node| node.kind),
                                Some(kobo_sdk::LayoutKind::ChessFeedback(Glyph::ThumbDown))
                            ));
                        }
                        (BoardMode::Solution, Some(SolutionFeedback::Complete)) => {
                            assert!(matches!(
                                feedback_mark.map(|node| node.kind),
                                Some(kobo_sdk::LayoutKind::ChessFeedback(Glyph::ThumbUp))
                            ));
                        }
                        _ => assert!(feedback_mark.is_none()),
                    }
                    if let Some(mark) = feedback_mark {
                        let board = diagnostics
                            .layout
                            .nodes
                            .iter()
                            .find_map(|node| match node.kind {
                                kobo_sdk::LayoutKind::ChessFrame { board, .. } => Some(board),
                                _ => None,
                            })
                            .expect("chess board frame");
                        assert!(
                            (mark.rect.x + mark.rect.width / 2 - board.x - board.width / 2).abs()
                                <= 1
                        );
                        assert!(
                            (mark.rect.y + mark.rect.height / 2 - board.y - board.height / 2).abs()
                                <= 1
                        );
                        assert!((mark.rect.width * 4 - board.width).abs() <= 4);
                    }
                    let rect_for = |action| {
                        diagnostics
                            .layout
                            .nodes
                            .iter()
                            .find(|node| node.kind.acts_on() == Some(action))
                            .expect("action has a touch target")
                            .rect
                    };
                    let board_frame = diagnostics
                        .layout
                        .nodes
                        .iter()
                        .find(|node| matches!(node.kind, kobo_sdk::LayoutKind::ChessFrame { .. }))
                        .expect("board has an outer frame")
                        .rect;
                    let flip = rect_for(action_id(FLIP));
                    assert!(
                        (flip.x + flip.width - board_frame.x - board_frame.width).abs() <= 1,
                        "toolbar should align with the outer board border"
                    );
                    let mode_cell = diagnostics
                        .layout
                        .nodes
                        .iter()
                        .find(|node| {
                            matches!(node.kind, kobo_sdk::LayoutKind::Cell(action, _, _)
                                if action == action_id(MODE_TOGGLE))
                        })
                        .expect("mode toggle should have a cell");
                    assert!(matches!(
                        mode_cell.kind,
                        kobo_sdk::LayoutKind::Cell(_, _, selected)
                            if selected == (mode == BoardMode::FreeBoard)
                    ));
                    for action in [MODE_TOGGLE, ORIENTATION_LOCK, RESET, FLIP] {
                        let button = rect_for(action_id(action));
                        assert!(
                            (button.width - button.height).abs() <= 1,
                            "button should be square"
                        );
                    }
                    for action in [MODE_TOGGLE, PUZZLE_PICKER] {
                        rect_for(action_id(action));
                    }
                }
            }
        }

        let (mut promotion_runner, _) = runner(Some(PROMOTION_PUZZLES.to_vec()));
        let movement = parse_uci_move("a7a8q").unwrap();
        promotion_runner.action(action_id(&square_action(movement.from)));
        promotion_runner.action(action_id(&square_action(movement.to)));
        let screen = promotion_runner.app().screen();
        let diagnostics = screen.diagnostics(
            &promotion_runner.context().metrics(),
            &Chrome::measuring(false),
        );
        assert!(
            !diagnostics.has_errors(),
            "promotion modal: {:?}",
            diagnostics.issues
        );
        for action in [
            PROMOTE_QUEEN,
            PROMOTE_ROOK,
            PROMOTE_BISHOP,
            PROMOTE_KNIGHT,
            PROMOTION_CANCEL,
        ] {
            assert!(
                diagnostics
                    .layout
                    .nodes
                    .iter()
                    .any(|node| node.kind.acts_on() == Some(action_id(action))),
                "{action} should have a touch target"
            );
        }

        let (mut picker_runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
            ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
        ]);
        picker_runner.action(action_id(PUZZLE_PICKER));
        let screen = picker_runner.app().screen();
        let diagnostics = screen.diagnostics(
            &picker_runner.context().metrics(),
            &Chrome::measuring(false),
        );
        assert!(
            !diagnostics.has_errors(),
            "collection picker: {:?}",
            diagnostics.issues
        );
        for index in 0..picker_runner.app().collections.len() {
            assert!(diagnostics
                .layout
                .nodes
                .iter()
                .any(|node| { node.kind.acts_on() == Some(action_id(&collection_action(index))) }));
        }
        assert!(diagnostics
            .layout
            .nodes
            .iter()
            .any(|node| { node.kind.acts_on() == Some(action_id(PUZZLE_PICKER_CANCEL)) }));
    }
    // Task 04: the browser is a transient overlay, not another solving mode.
    const RICH_PUZZLES: &[u8] =
        include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/valid-rich.json");

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct AttemptSnapshot {
        board: Board,
        solution_ply: usize,
        feedback: Option<SolutionFeedback>,
        pending_promotion: Option<PendingPromotion>,
        description_visible: bool,
        progress_bytes: Vec<u8>,
        progress_dirty: bool,
        progress_generation: u64,
        progress_saving: Option<u64>,
        solved: bool,
    }

    fn attempt_snapshot(app: &ChessBoardApp) -> AttemptSnapshot {
        AttemptSnapshot {
            board: app.board.clone(),
            solution_ply: app.solution_ply,
            feedback: app.solution_feedback,
            pending_promotion: app.pending_promotion.clone(),
            description_visible: app.description_visible,
            progress_bytes: app.progress.to_bytes().unwrap(),
            progress_dirty: app.progress_dirty,
            progress_generation: app.progress_generation,
            progress_saving: app.progress_saving,
            solved: app.current_is_solved(),
        }
    }

    fn assert_no_progress_write(commands: &[Command]) {
        assert!(!commands.iter().any(|command| matches!(
            command,
            Command::Store(StoreRequest::Save { key, .. }) if key == PROGRESS_KEY
        )));
    }

    fn rich_with_continuation() -> Vec<u8> {
        use serde_json::json;
        let mut value: serde_json::Value = serde_json::from_slice(RICH_PUZZLES).unwrap();
        value["puzzles"][0]["solution"] = json!(["h2h1q", "a1a2", "h1h2"]);
        value["puzzles"][0]["analysis"]["nodes"][1]["children"] = json!(["n4"]);
        let nodes = value["puzzles"][0]["analysis"]["nodes"]
            .as_array_mut()
            .unwrap();
        nodes.push(json!({
            "id": "n4", "parent": "n1",
            "move": {"uci": "a1a2", "san": "Ka2"}, "role": "main",
            "fen": "k7/8/8/8/8/8/K7/7q b - - 1 2",
            "children": ["n5"]
        }));
        nodes.push(json!({
            "id": "n5", "parent": "n4",
            "move": {"uci": "h1h2", "san": "Qh2"}, "role": "main",
            "fen": "k7/8/8/8/8/8/K6q/8 w - - 2 3"
        }));
        serde_json::to_vec(&value).unwrap()
    }

    #[test]
    fn preview_after_first_correct_move_keeps_attempt_and_progress_byte_identical() {
        let (mut runner, _) = runner(Some(rich_with_continuation()));
        assert_no_progress_write(&play(&mut runner, "h2h1q"));
        assert_eq!(runner.app().solution_ply, 2);
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Correct)
        );
        let before = attempt_snapshot(runner.app());
        let node = runner.app().puzzles[0]
            .analysis
            .as_ref()
            .unwrap()
            .by_id("n2")
            .unwrap();
        let alternative_fen = runner.app().puzzles[0]
            .analysis
            .as_ref()
            .unwrap()
            .node(node)
            .unwrap()
            .fen()
            .to_owned();

        assert!(runner.app_mut().open_analysis());
        assert_eq!(runner.app().displayed_board(), &before.board);
        assert!(runner.app_mut().select_analysis_node(node));
        assert_eq!(
            runner.app().analysis_browser.preview_fen(),
            Some(alternative_fen.as_str())
        );
        assert_eq!(
            runner.app().displayed_board(),
            &Board::from_fen(&alternative_fen).unwrap()
        );
        assert_ne!(runner.app().displayed_board(), &runner.app().board);

        let h1 = square_from_name("h1").unwrap();
        assert_no_progress_write(&runner.action(action_id(&square_action(h1))));
        assert_eq!(attempt_snapshot(runner.app()), before);
        assert_no_progress_write(&runner.action(action_id(FLIP)));
        assert_no_progress_write(&runner.action(action_id(ORIENTATION_LOCK)));
        assert!(runner.app().flipped == false); // started black-facing
        assert!(runner.app().orientation_locked);
        assert_eq!(
            runner.app().displayed_board(),
            &Board::from_fen(&alternative_fen).unwrap()
        );
        assert!(runner.app_mut().turn_analysis_page(true, 3));
        assert!(runner.app_mut().turn_analysis_page(true, 3));
        assert!(!runner.app_mut().turn_analysis_page(true, 3));
        assert_eq!(runner.app().analysis_browser.page(), 2);
        assert_eq!(attempt_snapshot(runner.app()), before);

        assert_no_progress_write(&runner.action(action_id(ANALYSIS_TOGGLE)));
        assert!(!runner.app().analysis_browser.is_open());
        assert_eq!(runner.app().displayed_board(), &before.board);
        assert_eq!(attempt_snapshot(runner.app()), before);

        let commands = play(&mut runner, "h1h2");
        assert_eq!(runner.app().solution_ply, 3);
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Complete)
        );
        assert!(runner.app().current_is_solved());
        assert!(
            progress_bytes(&commands).is_some(),
            "only actual solution completion saves"
        );
    }

    #[test]
    fn browsing_promotion_alternatives_and_sidelines_never_grades_them() {
        let (mut runner, _) = runner(Some(RICH_PUZZLES.to_vec()));
        let before = attempt_snapshot(runner.app());
        assert_no_progress_write(&runner.action(action_id(ANALYSIS_TOGGLE)));
        assert!(runner.app().analysis_browser.is_open());

        for (id, kind) in [("n2", PieceKind::Knight), ("n3", PieceKind::Rook)] {
            let node = runner.app().puzzles[0]
                .analysis
                .as_ref()
                .unwrap()
                .by_id(id)
                .unwrap();
            assert!(runner.app_mut().select_analysis_node(node));
            assert_eq!(
                runner
                    .app()
                    .displayed_board()
                    .piece_at(square_from_name("h1").unwrap()),
                Some(Piece {
                    color: Color::Black,
                    kind
                })
            );
            assert_eq!(attempt_snapshot(runner.app()), before);
        }
        let root = runner.app().puzzles[0].analysis.as_ref().unwrap().root();
        assert!(runner.app_mut().select_analysis_node(root));
        assert_eq!(runner.app().displayed_board(), &before.board);
        assert_eq!(attempt_snapshot(runner.app()), before);
        assert!(runner.app_mut().close_analysis());
        assert_eq!(attempt_snapshot(runner.app()), before);

        assert_no_progress_write(&play(&mut runner, "h2h1n"));
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Wrong)
        );
        assert!(!runner.app().current_is_solved());
        assert_eq!(
            runner.app().progress.to_bytes().unwrap(),
            before.progress_bytes
        );
        assert_eq!(runner.app().progress_generation, before.progress_generation);
        let commands = play(&mut runner, "h2h1q");
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Complete)
        );
        assert!(runner.app().current_is_solved());
        assert!(progress_bytes(&commands).is_some());
    }

    #[test]
    fn modal_and_position_transitions_clear_reader_without_changing_legacy_page_turns() {
        let (mut runner, _) = runner_with_collections(vec![
            (PUZZLES_FILE, RICH_PUZZLES.to_vec()),
            ("puzzles-endgames.json", ALT_PUZZLES.to_vec()),
        ]);
        let target = runner.app().puzzles[0]
            .analysis
            .as_ref()
            .unwrap()
            .by_id("n2")
            .unwrap();
        let h2 = square_from_name("h2").unwrap();
        let h1 = square_from_name("h1").unwrap();
        runner.action(action_id(&square_action(h2)));
        runner.action(action_id(&square_action(h1)));
        assert!(runner.app().pending_promotion.is_some());
        assert!(!runner.app_mut().open_analysis());
        assert_no_progress_write(&runner.action(action_id(ANALYSIS_TOGGLE)));
        assert!(!runner.app().analysis_browser.is_open());
        runner.action(action_id(PROMOTION_CANCEL));

        assert!(runner.app_mut().open_analysis());
        assert!(runner.app_mut().select_analysis_node(target));
        runner.page_turn(true);
        assert!(runner.app().analysis_browser.is_open());
        assert_eq!(runner.app().puzzle_index, 0);
        runner.action(action_id(RESET));
        assert!(!runner.app().analysis_browser.is_open());
        assert!(runner.app().analysis_browser.preview_board().is_none());

        assert!(runner.app_mut().open_analysis());
        assert!(runner.app_mut().select_analysis_node(target));
        runner.action(action_id(MODE_TOGGLE));
        assert_eq!(runner.app().mode, BoardMode::FreeBoard);
        assert!(!runner.app().analysis_browser.is_open());
        assert!(!runner.app_mut().open_analysis());
        runner.action(action_id(MODE_TOGGLE));

        assert!(runner.app_mut().open_analysis());
        runner.action(action_id(PUZZLE_PICKER));
        assert!(!runner.app().analysis_browser.is_open());
        assert!(runner.app().picker_open);
        assert!(!runner.app_mut().open_analysis());
        runner.action(action_id(PUZZLE_PICKER_CANCEL));

        assert!(runner.app_mut().open_analysis());
        assert!(runner.app_mut().select_analysis_node(target));
        let index = runner
            .app()
            .collections
            .iter()
            .position(|entry| entry.key == "puzzles-endgames.json")
            .unwrap();
        runner.action(action_id(PUZZLE_PICKER));
        let commands = runner.action(action_id(&collection_action(index)));
        assert!(!runner.app().analysis_browser.is_open());
        assert!(commands.iter().any(|command| matches!(
            command, Command::Store(StoreRequest::Load { key })
                if key == "puzzles-endgames.json"
        )));
        runner.store_result(StoreResult::Loaded {
            key: "puzzles-endgames.json".into(),
            value: Some(ALT_PUZZLES.to_vec()),
        });
        assert_eq!(runner.app().puzzles[0].id, "end-1");
        assert!(runner.app().analysis_browser.preview_board().is_none());
        // Legacy descriptions are readable, but cannot synthesize analysis nodes.
        assert!(runner.app_mut().open_analysis());
        assert!(!runner.app_mut().select_analysis_node(target));
        assert!(runner.app().analysis_browser.preview_board().is_none());
        runner.app_mut().close_analysis();

        // Normal page turns still navigate puzzles once the reader is closed.
        runner.page_turn(true);
        assert_eq!(runner.app().puzzle_index, 1);
        runner.page_turn(false);
        assert_eq!(runner.app().puzzle_index, 0);
    }

    #[test]
    fn suspend_and_resume_discard_preview_but_keep_the_live_attempt() {
        let (mut runner, _) = runner(Some(rich_with_continuation()));
        play(&mut runner, "h2h1q");
        let before = attempt_snapshot(runner.app());
        let alternative = runner.app().puzzles[0]
            .analysis
            .as_ref()
            .unwrap()
            .by_id("n2")
            .unwrap();
        assert!(runner.app_mut().open_analysis());
        assert!(runner.app_mut().select_analysis_node(alternative));
        assert_ne!(runner.app().displayed_board(), &before.board);

        assert_no_progress_write(&runner.suspend());
        assert!(runner.app().sleeping);
        assert!(!runner.app().analysis_browser.is_open());
        assert_eq!(runner.app().analysis_browser.selected_node(), None);
        assert_eq!(runner.app().displayed_board(), &before.board);
        assert_eq!(attempt_snapshot(runner.app()), before);
        assert!(!runner.app_mut().open_analysis());

        assert_no_progress_write(&runner.resume());
        assert!(!runner.app().sleeping);
        assert_eq!(attempt_snapshot(runner.app()), before);
        play(&mut runner, "h1h2");
        assert!(runner.app().current_is_solved());
    }
}

fn main() -> ExitCode {
    match kobo_sdk::run("eink-chess", ChessBoardApp::default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("eink-chess: {error}");
            ExitCode::FAILURE
        }
    }
}
