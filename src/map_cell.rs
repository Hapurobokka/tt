use crate::color_serde;
use std::collections::HashMap;
use std::fmt;
use std::ops::Add;

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
    offset: Position,
    visible: (u16, u16),
}

#[derive(Serialize, Deserialize)]
struct MapState {
    cells: Vec<Vec<Cell>>,
    tokens: Vec<Token>,
}

const PALETTE: [Color; 8] = [
    Color::White,
    Color::Black,
    Color::Red,
    Color::Blue,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Gray,
];

const TERRAIN: [char; 8] = ['.', '#', '|', '"', '-', '+', '<', '>'];

// MAN, THIS REDERING SHIT IS SO HARD
const WIDTH: u16 = 120;
const HEIGHT: u16 = 70;

// It is probably okay to copy and clone this stuff
// since it is not that big. Colors are just hexes (maybe)
// and chars are only ints
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Cell {
    #[serde(with = "color_serde")]
    bg_color: Color,
    #[serde(with = "color_serde")]
    fg_color: Color,
    character: char,
}

// but it makes feel kinda dumb lol
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
struct Token {
    pos: Position,
    character: char,
    #[serde(with = "color_serde")]
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
    MovingToken(usize, Position),
}

#[derive(Debug, PartialEq)]
enum State {
    Normal,
    Active(Mode),
}

use State::{Active, Normal};
use serde::{Deserialize, Serialize};

impl Add for Position {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

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
        for _ in 1..=WIDTH {
            let mut cs = Vec::<Cell>::new();
            for _ in 1..=HEIGHT {
                cs.push(Cell {
                    fg_color: Color::White,
                    bg_color: Color::Reset,
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
            offset: Position { x: 0, y: 0 },
            visible: (0, 0),
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
            Active(Mode::Drawing) => self.paint(self.brush, self.cursor.pos + self.offset),
            Active(Mode::DeletingTerrain) => self.delete(),
            Active(Mode::MovingToken(i, _)) => {
                self.tokens[i].pos = self.cursor.pos + self.offset;
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
        let pos = self.cursor.pos + self.offset;
        match self.brush {
            Brush::BgColor(_) => self.paint(Brush::BgColor(Color::Reset), pos),
            Brush::FgColor(_) => self.paint(Brush::FgColor(Color::White), pos),
            Brush::Char(_) => self.paint(Brush::Char(' '), pos),
        }
    }

    fn delete_rect(&mut self, anchor: Position) {
        let b = match self.brush {
            Brush::BgColor(_) => Brush::BgColor(Color::Reset),
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
        let second = self.cursor.pos + self.offset;

        let dx = second.x.cast_signed() - anchor.x.cast_signed();
        let dy = second.y.cast_signed() - anchor.y.cast_signed();

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
        self.tokens
            .iter()
            .position(|t| t.pos == self.cursor.pos + self.offset)
    }

    const fn move_cursor(&mut self, dx: i16, dy: i16) {
        if self.visible.0 == 0 || self.visible.1 == 0 {
            return;
        }

        let new_x = (self.cursor.pos.x.cast_signed() + dx).cast_unsigned();
        let new_y = (self.cursor.pos.y.cast_signed() + dy).cast_unsigned();

        if new_x > self.visible.0 {
            if self.offset.x + self.visible.0 < WIDTH {
                self.offset.x += 1;
            }
        } else if new_x == 0 {
            if self.offset.x > 0 {
                self.offset.x -= 1;
            }
        } else {
            self.cursor.pos.x = new_x;
        }

        if new_y > self.visible.1 {
            if self.offset.y + self.visible.1 < HEIGHT {
                self.offset.y += 1;
            }
        } else if new_y == 0 {
            if self.offset.y > 0 {
                self.offset.y -= 1;
            }
        } else {
            self.cursor.pos.y = new_y;
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
                Mode::MovingToken(..) => {
                    self.cursor.stay_here();
                }
                Mode::PlacingToken { character: Some(c) } => {
                    self.tokens.push(Token {
                        pos: self.cursor.pos + self.offset,
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
        if let Active(Mode::MovingToken(i, origin)) = self.state {
            self.tokens[i].pos = origin;
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

    pub fn set_visible(&mut self, area: Rect) {
        self.visible = (area.width.saturating_sub(2), area.height.saturating_sub(2));
        self.cursor.pos.x = self.cursor.pos.x.clamp(1, self.visible.0.max(1));
        self.cursor.pos.y = self.cursor.pos.y.clamp(1, self.visible.1.max(1));
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
                        anchor: self.cursor.pos + self.offset,
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
                        anchor: self.cursor.pos + self.offset,
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
                    self.state = Active(Mode::MovingToken(i, self.tokens[i].pos));
                    self.cursor.character = self.tokens[i].character;
                    self.cursor.fg_color = self.tokens[i].fg_color;
                } else if let Active(Mode::MovingToken(..)) = self.state {
                    self.commit();
                }
            }
            _ => {}
        }
    }

    fn draw_map(&self, area: Rect, buf: &mut Buffer) {
        for x in 1..=self.visible.0 {
            for y in 1..=self.visible.1 {
                let map_x = self.offset.x + x;
                let map_y = self.offset.y + y;

                if !(1..=WIDTH).contains(&map_x) || !(1..=HEIGHT).contains(&map_y) {
                    continue;
                }

                let (nx, ny) = (map_x as usize - 1, map_y as usize - 1);

                if let Some(c) = self.overlay.get(&(nx, ny)) {
                    buf[(area.x + x, area.y + y)]
                        .set_char(c.character)
                        .set_bg(c.bg_color)
                        .set_fg(c.fg_color);
                } else {
                    buf[(area.x + x, area.y + y)]
                        .set_char(self.cells[nx][ny].character)
                        .set_bg(self.cells[nx][ny].bg_color)
                        .set_fg(self.cells[nx][ny].fg_color);
                }
            }
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
        self.draw_map(area, buf);

        // TODO A way to implement key hints
        // let x = key_hints!(("x", "Borrar"), ("d", "Borrar token"));

        for t in &self.tokens {
            if t.pos.x <= self.offset.x
                || t.pos.x > self.offset.x + self.visible.0
                || t.pos.y <= self.offset.y
                || t.pos.y > self.offset.y + self.visible.1
            {
                continue;
            }
            let screen_x = area.x + t.pos.x - self.offset.x;
            let screen_y = area.y + t.pos.y - self.offset.y;
            buf[(screen_x, screen_y)]
                .set_char(t.character)
                .set_fg(t.fg_color)
                .set_style(Modifier::BOLD);
        }

        buf[(area.x + self.cursor.pos.x, area.y + self.cursor.pos.y)]
            .set_char(self.cursor.character)
            .set_fg(self.cursor.fg_color)
            .set_style(Modifier::BOLD);

        let mut title = Line::from(match self.state {
            Active(Mode::Drawing | Mode::Rectangle { anchor: _ }) => "DRAWING",
            Active(Mode::DeletingTerrain | Mode::DeletingRect { anchor: _ }) => "DELETING",
            Normal => "EXPLORING",
            Active(Mode::MovingToken(..)) => "MOVING",
            Active(Mode::PlacingToken { character: None }) => "PLACING (waiting...)",
            Active(Mode::PlacingToken { character: Some(_) }) => "PLACING (be nice!)",
        });
        title.push_span(format!(" {:?} ", self.offset));

        let current_color = Line::from(format!(
            "COLOR: {:?} | {} ",
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
                Active(Mode::MovingToken(..)) => Color::Yellow,
                Active(Mode::PlacingToken { character: _ }) => Color::Cyan,
            })
            .border_set(border::THICK);
        block.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_color_roundtrip() {
        let colors = [
            Color::White,
            Color::Black,
            Color::Red,
            Color::Blue,
            Color::Green,
            Color::Yellow,
            Color::Magenta,
            Color::Gray,
        ];

        for color in colors {
            let serialized = serde_json::to_string(&ColorWrapper(color)).unwrap();
            let deserialized: ColorWrapper = serde_json::from_str(&serialized).unwrap();
            assert_eq!(color, deserialized.0);
        }
    }

    // Wrapper auxiliar para poder serializar Color solo en el test
    #[derive(Serialize, Deserialize)]
    struct ColorWrapper(#[serde(with = "color_serde")] Color);

    #[test]
    fn test_cell_roundtrip() {
        let cell = Cell {
            bg_color: Color::Black,
            fg_color: Color::White,
            character: 'A',
        };

        let serialized = serde_json::to_string(&cell).unwrap();
        let deserialized: Cell = serde_json::from_str(&serialized).unwrap();

        assert_eq!(cell.bg_color, deserialized.bg_color);
        assert_eq!(cell.fg_color, deserialized.fg_color);
        assert_eq!(cell.character, deserialized.character);
    }

    #[test]
    fn test_color_json_format() {
        // Verifica que el JSON quede como esperamos visualmente
        let cell = Cell {
            bg_color: Color::Black,
            fg_color: Color::Red,
            character: 'X',
        };

        let serialized = serde_json::to_string(&cell).unwrap();
        assert!(serialized.contains("\"black\""));
        assert!(serialized.contains("\"red\""));
    }

    #[test]
    fn test_unknown_color_fails() {
        // Un color desconocido debe fallar limpiamente
        let bad_json = r#"{"bg_color":"purple","fg_color":"white","character":"A"}"#;
        let result: Result<Cell, _> = serde_json::from_str(bad_json);
        assert!(result.is_err());
    }
}
