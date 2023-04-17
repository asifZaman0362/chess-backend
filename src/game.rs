use actix::{Actor, Addr, Context, Handler, Message};
use serde::{Deserialize, Serialize};
use serde_json::to_string;

use crate::chessclient::ChessClient;
use crate::chessclient::TakePiece;

fn max(x: u8, y: u8) -> u8 {
    match x > y {
        true => x,
        false => y,
    }
}

fn min(x: u8, y: u8) -> u8 {
    match x < y {
        true => x,
        false => y,
    }
}

// manages game state
// associated with a server
// cannot exist independantly
pub struct Game {
    players: [Addr<ChessClient>; 2],
    turn: usize,
    discarded: Vec<ChessPiece>,
    boards: [[Option<ChessPiece>; 64]; 2],
}

impl Game {
    fn check_move_pattern(&self, piece: &ChessPiece, from: Pos, to: Pos) -> bool {
        let dx = max(from.x, to.x) - min(from.x, to.x);
        let dy = max(from.y, to.y) - min(from.y, to.y);
        let to_projected_pos = (to.y * 8 + to.x) as usize;
        if dx == 0 && dy == 0 {
            return false;
        }
        match piece {
            ChessPiece::White(variant) | ChessPiece::Black(variant) => match variant {
                PieceVariant::Bishop => dx == dy,
                PieceVariant::King => dx == 1 && dy == 1,
                PieceVariant::Knight => (dx == 1 && dy == 2) | (dx == 2 && dy == 1),
                PieceVariant::Pawn => {
                    if from.y == 1 {
                        // if first move
                        dy == 1 || dy == 2 && dx == 0 // allow moving either 1 or 2 places straight ahead
                    } else {
                        // if there is an enemy pawn to the immediate diagonal on either side
                        // then allow moving diagonally ahead one place
                        if dy == 1
                            && dx == 1
                            && self.boards[(self.turn + 1) % 2][to_projected_pos].is_some()
                        {
                            true
                        } else if dy == 1 && dx == 0 {
                            // othwerwise allow a single move straight ahead
                            true
                        } else {
                            false
                        }
                    }
                }
                PieceVariant::Rook => dx == 0 || dy == 0,
                PieceVariant::Queen => dx == 0 || dy == 0 || (dx == dy),
            },
        }
    }

    fn take_piece_if_exists(&mut self, whose: usize, at: usize) {
        if self.boards[whose][at].is_some() {
            self.discarded.push(self.boards[whose][at].unwrap().clone());
            self.boards[whose][at] = None;
            for player in self.players.iter() {
                player.do_send(TakePiece { at });
            }
        }
    }

    fn make_move(&mut self, chess_piece: ChessPiece, from: Pos, to: Pos) -> Result<(), MoveError> {
        let from_projected_pos = (from.y * 8 + from.x) as usize;
        match &self.boards[self.turn][from_projected_pos] {
            Some(piece) => {
                if *piece == chess_piece {
                    let to_projected_pos = (to.y * 8 + to.y) as usize;
                    if self.boards[self.turn][to_projected_pos].is_some() {
                        Err(MoveError::SpaceOccupied)
                    } else {
                        if self.check_move_pattern(piece, from, to) {
                            self.take_piece_if_exists((self.turn + 1) % 2, to_projected_pos);
                            Ok(())
                        } else {
                            Err(MoveError::InvalidPosition)
                        }
                    }
                } else {
                    Err(MoveError::PieceMismatch)
                }
            }
            None => Err(MoveError::PieceMismatch),
        }
    }
}

impl Actor for Game {
    type Context = Context<Self>;
}

#[derive(Deserialize, PartialEq, PartialOrd, Clone, Copy)]
pub enum PieceVariant {
    Bishop,
    King,
    Knight,
    Pawn,
    Queen,
    Rook,
}

#[derive(Deserialize, PartialEq, PartialOrd, Clone, Copy)]
pub enum ChessPiece {
    White(PieceVariant),
    Black(PieceVariant),
}

#[derive(Deserialize)]
pub struct Pos {
    x: u8,
    y: u8,
}

#[derive(Deserialize)]
pub struct MoveDetails {
    pub piece: ChessPiece,
    pub from: Pos,
    pub to: Pos,
}

#[derive(Serialize)]
pub enum MoveError {
    PieceMismatch,
    InvalidPosition,
    SpaceOccupied,
    InvalidTurn,
    NotInGame,
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct MakeMove {
    pub move_details: MoveDetails,
    pub player: Addr<ChessClient>,
}

impl Handler<MakeMove> for Game {
    type Result = Result<(), String>;
    fn handle(&mut self, msg: MakeMove, _ctx: &mut Self::Context) -> Self::Result {
        match self.players.iter().position(|player| *player == msg.player) {
            Some(pos) => {
                if pos == self.turn {
                    match self.make_move(
                        msg.move_details.piece,
                        msg.move_details.from,
                        msg.move_details.to,
                    ) {
                        Ok(()) => Ok(()),
                        Err(err) => Err(to_string(&err).unwrap()),
                    }
                } else {
                    Err(to_string(&MoveError::InvalidTurn).unwrap())
                }
            }
            None => Err(to_string(&MoveError::InvalidTurn).unwrap()),
        }
    }
}
