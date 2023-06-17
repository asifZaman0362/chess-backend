use actix::{Actor, ActorContext, Context, Handler, Message as ActixMessage, Recipient};
use serde::{Deserialize, Serialize};
use serde_json::to_string;

use crate::{chessclient::Message, message::OutgoingMessage};

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
    players: [Recipient<Message>; 2],
    turn: usize,
    discarded: Vec<ChessPiece>,
    boards: [[Option<ChessPiece>; 64]; 2],
}

impl Game {
    pub fn new(players: [Recipient<Message>; 2]) -> Self {
        let turn = 0;
        let discarded = vec![];
        let mut boards = [[None; 64]; 2];
        // set white pieces
        boards[0][0] = Some(ChessPiece::White(PieceVariant::Rook));
        boards[0][7] = Some(ChessPiece::White(PieceVariant::Rook));
        boards[0][1] = Some(ChessPiece::White(PieceVariant::Knight));
        boards[0][6] = Some(ChessPiece::White(PieceVariant::Knight));
        boards[0][2] = Some(ChessPiece::White(PieceVariant::Bishop));
        boards[0][5] = Some(ChessPiece::White(PieceVariant::Bishop));
        boards[0][3] = Some(ChessPiece::White(PieceVariant::Queen));
        boards[0][4] = Some(ChessPiece::White(PieceVariant::King));
        boards[0][8..16].fill(Some(ChessPiece::White(PieceVariant::Pawn)));
        // set black pieces
        boards[1][56] = Some(ChessPiece::Black(PieceVariant::Rook));
        boards[1][63] = Some(ChessPiece::Black(PieceVariant::Rook));
        boards[1][57] = Some(ChessPiece::Black(PieceVariant::Knight));
        boards[1][62] = Some(ChessPiece::Black(PieceVariant::Knight));
        boards[1][58] = Some(ChessPiece::Black(PieceVariant::Bishop));
        boards[1][61] = Some(ChessPiece::Black(PieceVariant::Bishop));
        boards[1][59] = Some(ChessPiece::Black(PieceVariant::Queen));
        boards[1][60] = Some(ChessPiece::Black(PieceVariant::King));
        boards[1][48..56].fill(Some(ChessPiece::Black(PieceVariant::Pawn)));
        Game {
            players,
            turn,
            discarded,
            boards,
        }
    }

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
                let msg = Message {
                    inner: OutgoingMessage::RemovePiece { at },
                    game: None,
                };
                player.do_send(msg);
            }
        }
    }

    fn move_piece(&mut self, whose: usize, from: Pos, to: Pos) {
        let p_from = (from.y * 8 + from.x) as usize;
        let p_to = (to.y * 8 + to.x) as usize;
        let piece = self.boards[whose][p_from];
        self.boards[whose][p_from] = None;
        self.boards[whose][p_to] = piece;
        self.players.clone().map(|p| {
            let inner = OutgoingMessage::MovePiece {
                from: p_from,
                to: p_to,
            };
            p.do_send(Message { inner, game: None })
        });
    }

    fn check_board(&mut self) {}

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
                            self.move_piece(self.turn, from, to);
                            self.turn = (self.turn + 1) % 2;
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

#[derive(Deserialize, Serialize, PartialEq, PartialOrd, Clone, Copy)]
pub enum PieceVariant {
    Bishop,
    King,
    Knight,
    Pawn,
    Queen,
    Rook,
}

#[derive(Deserialize, Serialize, PartialEq, PartialOrd, Clone, Copy)]
pub enum ChessPiece {
    White(PieceVariant),
    Black(PieceVariant),
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct Pos {
    x: u8,
    y: u8,
}

#[derive(Deserialize, Serialize)]
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

#[derive(ActixMessage)]
#[rtype(result = "Result<(), String>")]
pub struct MakeMove {
    pub move_details: MoveDetails,
    pub player: Recipient<Message>,
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
                        Ok(()) => {
                            log::debug!("Moved");
                            return Ok(());
                        }
                        Err(err) => {
                            log::error!("error {}", to_string(&err).unwrap());
                            return Err(to_string(&err).unwrap());
                        }
                    }
                } else {
                    Err(to_string(&MoveError::InvalidTurn).unwrap())
                }
            }
            None => Err(to_string(&MoveError::InvalidTurn).unwrap()),
        }
    }
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct ForfeitGame(pub Recipient<Message>);

impl Handler<ForfeitGame> for Game {
    type Result = ();
    fn handle(&mut self, msg: ForfeitGame, ctx: &mut Self::Context) -> Self::Result {
        if let Some(player) = self.players.iter().position(|p| *p == msg.0) {
            self.players[player].do_send(Message {
                inner: OutgoingMessage::LoseGame("Forfeit".to_owned()),
                game: None,
            });
            self.players[(player + 1) % 2].do_send(Message {
                inner: OutgoingMessage::WinGame("Forfeit".to_string()),
                game: None,
            });
            ctx.stop();
        }
    }
}
