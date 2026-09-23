//! Platform-neutral chessboard state.
//!
//! This is intentionally not a chess engine. It models a physical board:
//! select a piece, then put it on any square. There are no turns or rules.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Color {
    White,
    Black,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Board {
    squares: [Option<Piece>; 64],
    selected: Option<usize>,
}

impl Default for Board {
    fn default() -> Self {
        Self::starting_position()
    }
}

impl Board {
    pub fn starting_position() -> Self {
        let mut squares = [None; 64];
        let back_rank = [
            PieceKind::Rook,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Queen,
            PieceKind::King,
            PieceKind::Bishop,
            PieceKind::Knight,
            PieceKind::Rook,
        ];

        // Index 0 is a8, index 63 is h1.
        for (file, kind) in back_rank.into_iter().enumerate() {
            squares[file] = Some(Piece {
                color: Color::Black,
                kind,
            });
            squares[8 + file] = Some(Piece {
                color: Color::Black,
                kind: PieceKind::Pawn,
            });
            squares[48 + file] = Some(Piece {
                color: Color::White,
                kind: PieceKind::Pawn,
            });
            squares[56 + file] = Some(Piece {
                color: Color::White,
                kind,
            });
        }

        Self {
            squares,
            selected: None,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::starting_position();
    }

    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn piece_at(&self, square: usize) -> Option<Piece> {
        self.squares.get(square).copied().flatten()
    }

    /// Apply one tap. Returns true only when visible state changed.
    ///
    /// - Tap an occupied square to select it.
    /// - Tap it again to deselect it.
    /// - Tap any other square to move the selected piece there.
    /// - A move may replace another piece, just like picking pieces up by hand.
    pub fn tap(&mut self, square: usize) -> bool {
        if square >= self.squares.len() {
            return false;
        }

        match self.selected {
            None => {
                if self.squares[square].is_some() {
                    self.selected = Some(square);
                    true
                } else {
                    false
                }
            }
            Some(from) if from == square => {
                self.selected = None;
                true
            }
            Some(from) => {
                let Some(piece) = self.squares[from].take() else {
                    // Defensive recovery: selection should always contain a piece.
                    self.selected = None;
                    return true;
                };
                self.squares[square] = Some(piece);
                self.selected = None;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Board, Color, Piece, PieceKind};

    #[test]
    fn a_piece_can_be_selected_and_moved_anywhere() {
        let mut board = Board::default();

        assert!(board.tap(52)); // e2
        assert_eq!(board.selected(), Some(52));

        assert!(board.tap(36)); // e4
        assert!(board.piece_at(52).is_none());
        assert_eq!(
            board.piece_at(36),
            Some(Piece {
                color: Color::White,
                kind: PieceKind::Pawn,
            })
        );
        assert_eq!(board.selected(), None);
    }

    #[test]
    fn tapping_empty_square_first_does_nothing() {
        let mut board = Board::default();
        assert!(!board.tap(32));
        assert_eq!(board.selected(), None);
    }

    #[test]
    fn tapping_selected_piece_again_deselects_it() {
        let mut board = Board::default();

        assert!(board.tap(57)); // b1 knight
        assert!(board.tap(57));

        assert_eq!(board.selected(), None);
        assert_eq!(
            board.piece_at(57),
            Some(Piece {
                color: Color::White,
                kind: PieceKind::Knight,
            })
        );
    }

    #[test]
    fn moving_onto_an_occupied_square_replaces_that_piece() {
        let mut board = Board::default();

        assert!(board.tap(56)); // a1 white rook
        assert!(board.tap(0)); // a8 black rook

        assert_eq!(
            board.piece_at(0),
            Some(Piece {
                color: Color::White,
                kind: PieceKind::Rook,
            })
        );
        assert!(board.piece_at(56).is_none());
    }

    #[test]
    fn reset_restores_the_initial_position() {
        let mut board = Board::default();
        board.tap(52);
        board.tap(36);
        board.reset();

        assert_eq!(board, Board::starting_position());
    }
}
