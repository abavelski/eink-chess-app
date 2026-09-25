mod board;

use board::{Board, Color, Piece, PieceKind};
use kobo_sdk::{action_id, ActionId, Context, Glyph, KoboApp, Screen, ScreenBuilder, StoreResult};
use std::process::ExitCode;

const SIDE: usize = 8;
const CELLS: usize = SIDE * SIDE;
const RESET: &str = "reset";
const FLIP: &str = "flip";
const EXIT: &str = "exit";
const POSITIONS_FILE: &str = "positions.fen";
const EXAMPLE_POSITIONS: &str = include_str!("../examples/positions.fen");

#[derive(Default)]
struct ChessBoardApp {
    board: Board,
    positions: Vec<String>,
    position_index: usize,
    file_error: Option<String>,
    sleeping: bool,
    flipped: bool,
}

impl ChessBoardApp {
    fn activate_positions(
        &mut self,
        context: &mut Context,
        positions: Vec<String>,
        save_copy: bool,
    ) {
        self.positions = positions;
        self.position_index = 0;
        self.board = Board::from_fen(&self.positions[0]).expect("validated FEN position");
        if save_copy {
            let mut contents = self.positions.join("\n");
            contents.push('\n');
            context.store().save(POSITIONS_FILE, contents.into_bytes());
        }
        self.show(context);
    }

    fn load_examples(&mut self, context: &mut Context, save_copy: bool) {
        let positions = parse_position_file(EXAMPLE_POSITIONS)
            .expect("bundled example FEN positions are valid");
        self.activate_positions(context, positions, save_copy);
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
            "FEN file needs fixing".to_owned()
        } else {
            format!(
                "E-Ink Chess {}/{}",
                self.position_index + 1,
                self.positions.len()
            )
        };
        let mut screen = ScreenBuilder::new("chessboard")
            .top_bar(title)
            .top_bar_glyph(EXIT, "Return to Kobo reader", Glyph::Close)
            .board_with_selection(SIDE as u8, cells)
            .controls(
                2,
                [
                    (RESET, "Reset position", Glyph::Refresh),
                    (FLIP, "Flip board", Glyph::Grid),
                ],
            );
        if let Some(error) = self.file_error.as_ref() {
            screen = screen.text(error.clone());
        }
        screen.build()
    }

    fn turn_position(&mut self, context: &mut Context, forward: bool) {
        let next = if forward {
            self.position_index
                .checked_add(1)
                .filter(|&index| index < self.positions.len())
        } else {
            self.position_index.checked_sub(1)
        };
        let Some(next) = next else {
            return;
        };
        if let Ok(board) = Board::from_fen(&self.positions[next]) {
            self.position_index = next;
            self.board = board;
            self.show(context);
        }
    }
}

impl KoboApp for ChessBoardApp {
    fn on_start(&mut self, context: &mut Context) {
        context.store().load(POSITIONS_FILE);
    }

    fn on_load(&mut self, context: &mut Context, key: &str, result: StoreResult) {
        if key != POSITIONS_FILE {
            return;
        }
        match result {
            StoreResult::Loaded {
                value: Some(bytes), ..
            } => match String::from_utf8(bytes) {
                Ok(contents) => match parse_position_file(&contents) {
                    Ok(positions) => {
                        self.file_error = None;
                        self.activate_positions(context, positions, false);
                    }
                    Err(error) => {
                        self.file_error = Some(error);
                        self.load_examples(context, false);
                    }
                },
                Err(_) => {
                    self.file_error = Some("The positions.fen file is not UTF-8 text.".into());
                    self.load_examples(context, false);
                }
            },
            StoreResult::Loaded { value: None, .. } => {
                self.file_error = None;
                self.load_examples(context, true);
            }
            StoreResult::Denied(_) => {
                self.file_error =
                    Some("Saved positions could not be read; showing examples.".into());
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
                if self.board.tap(square) {
                    self.show(context);
                }
                return;
            }
        }
    }

    fn on_page_turn(&mut self, context: &mut Context, forward: bool) {
        self.turn_position(context, forward);
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

fn parse_position_file(contents: &str) -> Result<Vec<String>, String> {
    let mut positions = Vec::new();
    for (line_index, line) in contents.lines().enumerate() {
        let fen = line.trim();
        if fen.is_empty() || fen.starts_with('#') {
            continue;
        }
        Board::from_fen(fen).map_err(|error| format!("Line {}: {error}", line_index + 1))?;
        positions.push(fen.to_owned());
    }
    if positions.is_empty() {
        return Err("No FEN positions found. Add one six-field FEN per line.".into());
    }
    Ok(positions)
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

fn square_label(square: usize, piece: Option<Piece>) -> String {
    let file = (b'a' + (square % SIDE) as u8) as char;
    let rank = (b'8' - (square / SIDE) as u8) as char;
    match piece {
        Some(piece) => format!("{file}{rank} {}", piece_label(piece)),
        None => format!("{file}{rank}"),
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
    use super::{board_square, square_label, CELLS};

    #[test]
    fn flipped_display_reverses_board_and_algebraic_notation() {
        assert_eq!(board_square(0, false), 0);
        assert_eq!(board_square(CELLS - 1, false), CELLS - 1);
        assert_eq!(board_square(0, true), CELLS - 1);
        assert_eq!(board_square(CELLS - 1, true), 0);
        assert_eq!(square_label(0, None), "a8");
        assert_eq!(square_label(CELLS - 1, None), "h1");
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
