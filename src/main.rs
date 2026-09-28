mod board;
mod puzzle;

use board::{Board, Color, Piece, PieceKind, TapResult};
use kobo_sdk::{
    action_id, ActionId, BandAlign, BannerLevel, Context, Glyph, KoboApp, Screen, ScreenBuilder,
    SlotWidth, StoreResult,
};
use puzzle::{parse_puzzle_file, Puzzle};
use std::process::ExitCode;

const SIDE: usize = 8;
const CELLS: usize = SIDE * SIDE;
const RESET: &str = "reset";
const FLIP: &str = "flip";
const EXIT: &str = "exit";
const PUZZLES_FILE: &str = "puzzles.json";
const EXAMPLE_PUZZLES: &[u8] = include_bytes!("../examples/puzzles.json");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SolutionFeedback {
    Correct,
    Wrong,
    Complete,
}

#[derive(Default)]
struct ChessBoardApp {
    board: Board,
    puzzles: Vec<Puzzle>,
    puzzle_index: usize,
    solution_ply: usize,
    solution_feedback: Option<SolutionFeedback>,
    file_error: Option<String>,
    sleeping: bool,
    flipped: bool,
}

impl ChessBoardApp {
    fn activate_puzzles(&mut self, context: &mut Context, puzzles: Vec<Puzzle>, save_copy: bool) {
        self.puzzles = puzzles;
        self.select_puzzle(0);
        if save_copy {
            context.store().save(PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec());
        }
        self.show(context);
    }

    fn load_examples(&mut self, context: &mut Context, save_copy: bool) {
        let puzzles =
            parse_puzzle_file(EXAMPLE_PUZZLES).expect("bundled example puzzles are valid");
        self.activate_puzzles(context, puzzles, save_copy);
    }

    fn select_puzzle(&mut self, index: usize) {
        let puzzle = &self.puzzles[index];
        self.board = Board::from_fen(&puzzle.fen).expect("validated puzzle FEN");
        self.flipped = puzzle.side_to_move() == Color::Black;
        self.puzzle_index = index;
        self.reset_attempt();
    }

    fn reset_attempt(&mut self) {
        self.solution_ply = 0;
        self.solution_feedback = None;
    }

    fn show(&self, context: &mut Context) {
        context.set_screen(self.screen());
    }

    fn screen(&self) -> Screen {
        let selected = self.board.selected();

        let cells = (0..CELLS).map(|display_square| {
            let square = board_square(display_square, self.flipped);
            let piece = self.board.piece_at(square);
            (
                square_action(square),
                square_label(square, piece),
                piece.map(piece_glyph),
                selected == Some(square),
            )
        });

        let title = if self.sleeping {
            "Sleeping - press power to wake".to_owned()
        } else if self.file_error.is_some() {
            "Puzzle file needs fixing".to_owned()
        } else {
            format!(
                "E-Ink Chess {}/{}",
                self.puzzle_index + 1,
                self.puzzles.len()
            )
        };
        let mut screen = ScreenBuilder::new("chessboard")
            .top_bar(title)
            .top_bar_glyph(EXIT, "Return to Kobo reader", Glyph::Close)
            .board_with_selection(SIDE as u8, cells)
            .band(
                BandAlign::Middle,
                [
                    (
                        SlotWidth::Fill,
                        (|slot: ScreenBuilder| slot) as fn(ScreenBuilder) -> ScreenBuilder,
                    ),
                    (
                        // Two 10 mm targets with the grid's 1 mm gap.
                        SlotWidth::Fixed(210),
                        |slot| {
                            slot.controls(
                                2,
                                [
                                    (RESET, "Reset position", Glyph::Refresh),
                                    (FLIP, "Flip board", Glyph::SwapVertical),
                                ],
                            )
                        },
                    ),
                ],
            );

        if let Some(feedback) = self.solution_feedback {
            screen = match feedback {
                SolutionFeedback::Correct => screen.banner(BannerLevel::Info, "Correct"),
                SolutionFeedback::Wrong => {
                    screen.banner(BannerLevel::Attention, "Wrong move — try again")
                }
                SolutionFeedback::Complete => {
                    screen.banner(BannerLevel::Attention, "Solution complete")
                }
            };
        }

        if let Some(puzzle) = self.puzzles.get(self.puzzle_index) {
            screen = screen.secondary(match puzzle.side_to_move() {
                Color::White => "White to move",
                Color::Black => "Black to move",
            });
            if let Some(description) = puzzle
                .description
                .as_deref()
                .filter(|text| !text.trim().is_empty())
            {
                screen = screen.text(description);
            }
        }
        if let Some(error) = self.file_error.as_ref() {
            screen = screen.text(error.clone());
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
        self.show(context);
    }

    fn handle_square_tap(&mut self, square: usize) -> bool {
        if self.solution_feedback == Some(SolutionFeedback::Complete) {
            return false;
        }

        let before = self.board.clone();
        match self.board.tap(square) {
            TapResult::NoChange => false,
            TapResult::SelectionChanged => true,
            TapResult::Moved { from, to } => {
                self.check_solver_move(before, from, to);
                true
            }
        }
    }

    fn check_solver_move(&mut self, before: Board, from: usize, to: usize) {
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

        if uci_move(from, to) != expected {
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
            let Some((reply_from, reply_to)) = uci_squares(&reply) else {
                self.restore_invalid_reply(before, original_ply, &reply);
                return;
            };
            if !self.board.move_piece(reply_from, reply_to) {
                self.restore_invalid_reply(before, original_ply, &reply);
                return;
            }
            self.solution_ply += 1;
        }

        let solution_len = self
            .puzzles
            .get(self.puzzle_index)
            .map_or(0, |puzzle| puzzle.solution.len());
        self.solution_feedback = Some(if self.solution_ply >= solution_len {
            SolutionFeedback::Complete
        } else {
            SolutionFeedback::Correct
        });
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
        context.store().load(PUZZLES_FILE);
    }

    fn on_load(&mut self, context: &mut Context, key: &str, result: StoreResult) {
        if key != PUZZLES_FILE {
            return;
        }
        match result {
            StoreResult::Loaded {
                value: Some(bytes), ..
            } => match parse_puzzle_file(&bytes) {
                Ok(puzzles) => {
                    self.file_error = None;
                    self.activate_puzzles(context, puzzles, false);
                }
                Err(error) => {
                    self.file_error = Some(error);
                    self.load_examples(context, false);
                }
            },
            StoreResult::Loaded { value: None, .. } => {
                self.file_error = None;
                self.load_examples(context, true);
            }
            StoreResult::Denied(_) => {
                self.file_error = Some("puzzles.json could not be read; showing examples.".into());
                self.load_examples(context, false);
            }
            _ => {}
        }
    }

    fn on_action(&mut self, context: &mut Context, action: ActionId) {
        if action == action_id(EXIT) {
            context.exit();
            return;
        }

        if action == action_id(RESET) {
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
                    self.show(context);
                }
                return;
            }
        }
    }

    fn on_page_turn(&mut self, context: &mut Context, forward: bool) {
        self.turn_puzzle(context, forward);
    }

    fn on_suspend(&mut self, context: &mut Context) {
        self.sleeping = true;
        self.show(context);
    }

    fn on_resume(&mut self, context: &mut Context) {
        self.sleeping = false;
        self.show(context);
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

fn uci_move(from: usize, to: usize) -> String {
    format!("{}{}", square_name(from), square_name(to))
}

fn uci_squares(movement: &str) -> Option<(usize, usize)> {
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
    Some((from, to))
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

    fn runner(bytes: Option<Vec<u8>>) -> (AppRunner<ChessBoardApp>, Vec<Command>) {
        let mut runner = AppRunner::with_metrics(
            ChessBoardApp::default(),
            DisplayMetrics {
                width: 1264,
                height: 1680,
                pixels_per_inch: 300,
                ..DisplayMetrics::default()
            },
        );
        assert!(runner.start().iter().any(|command| matches!(
            command, Command::Store(StoreRequest::Load { key }) if key == PUZZLES_FILE
        )));
        let commands = runner.store_result(StoreResult::Loaded {
            key: PUZZLES_FILE.into(),
            value: bytes,
        });
        (runner, commands)
    }

    fn play(runner: &mut AppRunner<ChessBoardApp>, movement: &str) {
        let (from, to) = uci_squares(movement).expect("valid test UCI");
        runner.action(action_id(&square_action(from)));
        runner.action(action_id(&square_action(to)));
    }

    #[test]
    fn flipped_display_reverses_board_but_not_uci_coordinates() {
        assert_eq!(board_square(0, false), 0);
        assert_eq!(board_square(CELLS - 1, false), CELLS - 1);
        assert_eq!(board_square(0, true), CELLS - 1);
        assert_eq!(board_square(CELLS - 1, true), 0);
        assert_eq!(square_label(0, None), "a8");
        assert_eq!(square_label(CELLS - 1, None), "h1");

        let (from, to) = uci_squares("e2e4").unwrap();
        assert_eq!(uci_move(from, to), "e2e4");
        let flipped_from = (0..CELLS)
            .find(|&display| board_square(display, true) == from)
            .unwrap();
        let flipped_to = (0..CELLS)
            .find(|&display| board_square(display, true) == to)
            .unwrap();
        assert_ne!(flipped_from, from);
        assert_ne!(flipped_to, to);
        assert_eq!(
            uci_move(
                board_square(flipped_from, true),
                board_square(flipped_to, true)
            ),
            "e2e4"
        );
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
    fn changing_puzzles_clears_transient_solution_feedback() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        play(&mut runner, "d7e8");
        assert_eq!(
            runner.app().solution_feedback,
            Some(SolutionFeedback::Complete)
        );

        runner.page_turn(true);

        assert_eq!(runner.app().puzzle_index, 1);
        assert_eq!(runner.app().solution_ply, 0);
        assert_eq!(runner.app().solution_feedback, None);
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
    fn missing_file_creates_examples_but_invalid_file_is_preserved() {
        let (_, commands) = runner(None);
        assert!(commands.iter().any(|command| matches!(
            command, Command::Store(StoreRequest::Save { key, value })
                if key == PUZZLES_FILE && value.as_slice() == EXAMPLE_PUZZLES
        )));
        let (runner, commands) = runner(Some(b"invalid JSON".to_vec()));
        assert!(runner.app().file_error.is_some());
        assert_eq!(runner.app().puzzles.len(), 10);
        assert!(!commands
            .iter()
            .any(|command| matches!(command, Command::Store(StoreRequest::Save { .. }))));
    }

    #[test]
    fn puzzle_feedback_and_toolbar_fit_the_kobo_panel() {
        let (mut runner, _) = runner(Some(EXAMPLE_PUZZLES.to_vec()));
        let feedback_states = [
            None,
            Some(SolutionFeedback::Correct),
            Some(SolutionFeedback::Wrong),
            Some(SolutionFeedback::Complete),
        ];
        for index in 0..runner.app().puzzles.len() {
            runner.app_mut().select_puzzle(index);
            for feedback in feedback_states {
                runner.app_mut().solution_feedback = feedback;
                let screen = runner.app().screen();
                let diagnostics =
                    screen.diagnostics(&runner.context().metrics(), &Chrome::measuring(false));
                assert!(
                    !diagnostics.has_errors(),
                    "puzzle {index}, feedback {feedback:?}: {:?}",
                    diagnostics.issues
                );
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
                for action in [RESET, FLIP] {
                    let button = rect_for(action_id(action));
                    assert!(
                        (button.width - button.height).abs() <= 1,
                        "button should be square"
                    );
                }
            }
        }
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
