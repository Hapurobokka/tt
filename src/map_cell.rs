use std::collections::HashMap;
use std::fmt;

use crossterm::event::KeyModifiers;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Modifier;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

#[derive(Debug)]

pub struct CellMap {
    cursor: Cursor,
    cells: Vec<Vec<Cell>>,
    overlay: HashMap<(usize, usize), Cell>,
    bg_color_i: usize,
    fg_color_i: usize,
    char_i: usize,
    state: State,
    tokens: Vec<Token>,
    brush: Brush,
}

const PALETTE: [Color; 8] = [
    Color::White,
    Color::Cyan,
    Color::Red,
    Color::Blue,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Gray,
];

const TERRAIN: [char; 8] = ['.', '#', '|', '"', '-', '+', '<', '>'];

// MAN, THIS REDERING SHIT IS SO HARD
const WIDTH: u16 = 100;
const HEIGHT: u16 = 23;

// It is probably okay to copy and clone this stuff
// since it is not that big. Colors are just hexes (maybe)
// and chars are only ints
#[derive(Debug, Default, Clone, Copy)]
pub struct Cell {
    bg_color: Color,
    fg_color: Color,
    character: char,
}

// but it makes feel kinda dumb lol
#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    x: u16,
    y: u16,
}

#[derive(Debug)]
struct Cursor {
    pos: Position,
    prev_position: Position,
    character: char,
    fg_color: Color,
}

#[derive(Debug)]
struct Token {
    pos: Position,
    character: char,
    fg_color: Color,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' ({}, {})", self.character, self.pos.x, self.pos.y)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Brush {
    BgColor(Color),
    FgColor(Color),
    Char(char),
}

#[derive(Debug, PartialEq)]
enum Mode {
    Drawing,
    Rectangle { anchor: Position },
    DeletingTerrain,
    DeletingRect { anchor: Position },
    PlacingToken { character: Option<char> },
    MovingToken(usize),
}

#[derive(Debug, PartialEq)]
enum State {
    Normal,
    Active(Mode),
}

use State::{Active, Normal};

impl Cursor {
    const fn reset_cursor(&mut self) {
        self.pos = self.prev_position;
        self.fg_color = Color::Yellow;
        self.character = '@';
    }

    const fn stay_here(&mut self) {
        self.fg_color = Color::Yellow;
        self.character = '@';
    }

    const fn save_position(&mut self) {
        self.prev_position = self.pos;
    }
}

impl CellMap {
    pub fn build() -> Self {
        let mut cells: Vec<Vec<Cell>> = Vec::new();
        // TODO Hardcoded because we need to implement other stuff
        for _ in 1..WIDTH {
            let mut cs = Vec::<Cell>::new();
            for _ in 1..HEIGHT {
                cs.push(Cell {
                    fg_color: Color::White,
                    bg_color: Color::Black,
                    character: ' ',
                });
            }
            cells.push(cs);
        }

        Self {
            cursor: Cursor {
                pos: Position { x: 1, y: 1 },
                prev_position: Position { x: 1, y: 1 },
                character: '@',
                fg_color: Color::Yellow,
            },
            cells,
            overlay: HashMap::new(),
            tokens: Vec::new(),
            bg_color_i: 0,
            fg_color_i: 0,
            char_i: 0,
            state: Normal,
            brush: Brush::BgColor(Color::White),
        }
    }

    pub fn update(&mut self) {
        match self.state {
            Active(Mode::Drawing) => self.paint(self.brush, self.cursor.pos),
            Active(Mode::DeletingTerrain) => self.delete(),
            Active(Mode::MovingToken(i)) => {
                self.tokens[i].pos = self.cursor.pos;
            }
            Active(Mode::Rectangle { anchor: a }) => {
                self.paint_rec(self.brush, a);
            }
            Active(Mode::DeletingRect { anchor: a }) => {
                self.delete_rect(a);
            }
            _ => {}
        }
    }

    fn delete(&mut self) {
        match self.brush {
            Brush::BgColor(_) => {
                self.paint(Brush::BgColor(Color::Black), self.cursor.pos);
            }
            Brush::FgColor(_) => {
                self.paint(Brush::FgColor(Color::White), self.cursor.pos);
            }
            Brush::Char(_) => self.paint(Brush::Char(' '), self.cursor.pos),
        }
    }

    fn delete_rect(&mut self, anchor: Position) {
        let b = match self.brush {
            Brush::BgColor(_) => Brush::BgColor(Color::Black),
            Brush::FgColor(_) => Brush::FgColor(Color::White),
            Brush::Char(_) => Brush::Char(' '),
        };
        self.paint_rec(b, anchor);
    }

    fn paint(&mut self, brush: Brush, pos: Position) {
        let nx = pos.x as usize - 1;
        let ny = pos.y as usize - 1;
        let curr_cell = self
            .overlay
            .get(&(nx, ny))
            .copied()
            .unwrap_or_else(|| self.cells[nx][ny]);
        match brush {
            Brush::BgColor(c) => {
                let cell = Cell {
                    bg_color: c,
                    fg_color: curr_cell.fg_color,
                    character: curr_cell.character,
                };
                self.overlay.insert((nx, ny), cell);
            }
            Brush::Char(ch) => {
                let cell = Cell {
                    bg_color: curr_cell.bg_color,
                    fg_color: curr_cell.fg_color,
                    character: ch,
                };
                self.overlay.insert((nx, ny), cell);
            }
            Brush::FgColor(c) => {
                let cell = Cell {
                    bg_color: curr_cell.bg_color,
                    fg_color: c,
                    character: curr_cell.character,
                };
                self.overlay.insert((nx, ny), cell);
            }
        }
    }

    fn paint_rec(&mut self, brush: Brush, anchor: Position) {
        let add_to = |a: u16, b: u16| a + b;
        let sub_to = |a: u16, b: u16| a - b;

        // TODO not sold on this yet
        self.overlay.clear();
        let second = (self.cursor.pos.x, self.cursor.pos.y);

        let dx = second.0.cast_signed() - anchor.x.cast_signed();
        let dy = second.1.cast_signed() - anchor.y.cast_signed();

        let op_x = if dx < 0 { sub_to } else { add_to };
        let op_y = if dy < 0 { sub_to } else { add_to };

        for i in 0..=dx.abs().cast_unsigned() {
            for j in 0..=dy.abs().cast_unsigned() {
                self.paint(
                    brush,
                    Position {
                        x: op_x(anchor.x, i),
                        y: op_y(anchor.y, j),
                    },
                );
            }
        }
    }

    fn token_at(&self) -> Option<usize> {
        self.tokens.iter().position(|t| t.pos == self.cursor.pos)
    }

    const fn move_cursor(&mut self, dx: i16, dy: i16) {
        let new_x = self.cursor.pos.x.cast_signed() + dx;
        let new_y = self.cursor.pos.y.cast_signed() + dy;
        if new_x >= 1 && new_x < WIDTH.cast_signed() {
            self.cursor.pos.x = new_x.cast_unsigned();
        }
        if new_y >= 1 && new_y < HEIGHT.cast_signed() {
            self.cursor.pos.y = new_y.cast_unsigned();
        }
    }

    fn merge_overlay(&mut self) {
        for ((x, y), v) in &self.overlay {
            self.cells[*x][*y] = *v;
        }
    }

    fn commit(&mut self) {
        if let State::Active(mode) = &self.state {
            match mode {
                Mode::Drawing
                | Mode::DeletingTerrain
                | Mode::Rectangle { anchor: _ }
                | Mode::DeletingRect { anchor: _ } => {
                    self.merge_overlay();
                }
                Mode::MovingToken(_) => {
                    self.cursor.stay_here();
                }
                Mode::PlacingToken { character: Some(c) } => {
                    self.tokens.push(Token {
                        pos: Position {
                            x: self.cursor.pos.x,
                            y: self.cursor.pos.y,
                        },
                        character: *c,
                        fg_color: PALETTE[self.fg_color_i],
                    });
                    self.cursor.stay_here();
                }
                Mode::PlacingToken { character: None } => {}
            }
        }
        self.overlay.clear();
        self.state = Normal;
    }

    fn revert(&mut self) {
        if let Active(Mode::MovingToken(i)) = self.state {
            self.tokens[i].pos = self.cursor.prev_position;
        }
        if self.state != Normal {
            self.overlay.clear();
            self.cursor.reset_cursor();
            self.state = Normal;
        }
    }

    const fn change_brush(&mut self, choice: char) {
        let i = (choice as usize) - ('1' as usize);
        self.brush = match self.brush {
            Brush::Char(_) => {
                self.char_i = i;
                Brush::Char(TERRAIN[self.char_i])
            }
            Brush::BgColor(_) => {
                self.bg_color_i = i;
                Brush::BgColor(PALETTE[self.bg_color_i])
            }
            Brush::FgColor(_) => {
                self.fg_color_i = i;
                Brush::FgColor(PALETTE[self.fg_color_i])
            }
        };
    }

    pub fn handle_key_press(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char(c)
                if self.state == Active(Mode::PlacingToken { character: None })
                    && !key_event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if c == ' ' {
                    return;
                }
                self.state = Active(Mode::PlacingToken { character: Some(c) });
                self.cursor.fg_color = PALETTE[self.fg_color_i];
                self.cursor.character = c;
            }
            KeyCode::Left | KeyCode::Char('h') => self.move_cursor(-1, 0),
            KeyCode::Right | KeyCode::Char('l') => self.move_cursor(1, 0),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(0, -1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(0, 1),
            KeyCode::Char('y') => self.move_cursor(-1, -1),
            KeyCode::Char('u') => self.move_cursor(1, -1),
            KeyCode::Char('b') => self.move_cursor(-1, 1),
            KeyCode::Char('n') => self.move_cursor(1, 1),
            KeyCode::Tab => match self.brush {
                Brush::BgColor(_) => self.brush = Brush::FgColor(PALETTE[self.fg_color_i]),
                Brush::FgColor(_) => self.brush = Brush::Char(TERRAIN[self.char_i]),
                Brush::Char(_) => self.brush = Brush::BgColor(PALETTE[self.bg_color_i]),
            },
            KeyCode::Char(c @ '1'..='8') => self.change_brush(c),
            KeyCode::Esc => self.revert(),
            // The keys down here will do "commits", aka, here is where we put the keys
            // that actually do something.
            KeyCode::Char(' ') => {
                self.commit();
            }
            KeyCode::Char('d') => {
                if self.state == Active(Mode::Drawing) {
                    self.commit();
                } else if self.state == Normal {
                    self.cursor.save_position();
                    self.state = Active(Mode::Drawing);
                }
            }
            KeyCode::Char('D') => {
                if let Active(Mode::Rectangle { anchor: _ }) = self.state {
                    self.commit();
                } else if self.state == Normal {
                    self.cursor.save_position();
                    self.state = Active(Mode::Rectangle {
                        anchor: self.cursor.pos,
                    });
                }
            }
            KeyCode::Char('x') if key_event.modifiers == KeyModifiers::CONTROL => {
                if let Some(i) = self.token_at()
                    && self.state == Normal
                {
                    self.tokens.remove(i);
                }
            }
            KeyCode::Char('x') => {
                if self.state == Active(Mode::DeletingTerrain) {
                    self.commit();
                } else if self.state == Normal {
                    self.cursor.save_position();
                    self.state = Active(Mode::DeletingTerrain);
                }
            }
            KeyCode::Char('X') => {
                if let Active(Mode::DeletingRect { anchor: _ }) = self.state {
                    self.commit();
                } else if self.state == Normal {
                    self.cursor.save_position();
                    self.state = Active(Mode::DeletingRect {
                        anchor: self.cursor.pos,
                    });
                }
            }
            KeyCode::Char('t') => {
                if let Active(Mode::PlacingToken { character: _ }) = self.state {
                    self.commit();
                } else if self.state == Normal {
                    self.cursor.save_position();
                    self.state = Active(Mode::PlacingToken { character: None });
                }
            }
            KeyCode::Char('m') => {
                if let Some(i) = self.token_at()
                    && self.state == Normal
                {
                    self.cursor.save_position();
                    self.state = Active(Mode::MovingToken(i));
                    self.cursor.character = self.tokens[i].character;
                    self.cursor.fg_color = self.tokens[i].fg_color;
                } else if let Active(Mode::MovingToken(_)) = self.state {
                    self.commit();
                }
            }
            _ => {}
        }
    }
}

// macro_rules! key_hints {
//     ($(($key:expr, $desc:expr)),+) => {
//         Line::from(vec![
//             $(
//                 format!("[{}]: {} ", $key, $desc).into(),
//             )+
//         ])
//     }
// }

impl Widget for &CellMap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in 1..WIDTH {
            for y in 1..HEIGHT {
                let (dx, dy) = (area.x + x, area.y + y);
                let (nx, ny) = (x as usize - 1, y as usize - 1);
                if let Some(c) = self.overlay.get(&(nx, ny)) {
                    buf[(dx, dy)]
                        .set_char(c.character)
                        .set_bg(c.bg_color)
                        .set_fg(c.fg_color);
                } else {
                    buf[(dx, dy)]
                        .set_char(self.cells[nx][ny].character)
                        .set_bg(self.cells[nx][ny].bg_color)
                        .set_fg(self.cells[nx][ny].fg_color);
                }
            }
        }

        // TODO A way to implement key hints
        // let x = key_hints!(("x", "Borrar"), ("d", "Borrar token"));

        for t in &self.tokens {
            buf[(t.pos.x, t.pos.y)]
                .set_char(t.character)
                .set_fg(t.fg_color)
                .set_style(Modifier::BOLD);
        }

        buf[(area.x + self.cursor.pos.x, area.y + self.cursor.pos.y)]
            .set_char(self.cursor.character)
            .set_fg(self.cursor.fg_color)
            .set_style(Modifier::BOLD);

        let title = Line::from(match self.state {
            Active(Mode::Drawing | Mode::Rectangle { anchor: _ }) => "DRAWING",
            Active(Mode::DeletingTerrain | Mode::DeletingRect { anchor: _ }) => "DELETING",
            Normal => "EXPLORING",
            Active(Mode::MovingToken(_)) => "MOVING",
            Active(Mode::PlacingToken { character: None }) => "PLACING (waiting...)",
            Active(Mode::PlacingToken { character: Some(_) }) => "PLACING (be nice!)",
        });

        let current_color = Line::from(format!(
            "COLOR: {:?} | {}",
            self.brush,
            self.token_at()
                .map(|i| self.tokens[i].to_string())
                .unwrap_or_default()
        ));

        let block = Block::bordered()
            .title(title)
            .title_bottom(current_color)
            .border_style(match self.state {
                Active(Mode::Drawing | Mode::Rectangle { anchor: _ }) => Color::Magenta,
                Active(Mode::DeletingTerrain | Mode::DeletingRect { anchor: _ }) => Color::Red,
                Normal => Color::White,
                Active(Mode::MovingToken(_)) => Color::Yellow,
                Active(Mode::PlacingToken { character: _ }) => Color::Cyan,
            })
            .border_set(border::THICK);
        block.render(area, buf);
    }
}
