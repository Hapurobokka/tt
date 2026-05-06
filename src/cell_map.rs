use crate::color_serde;
use color_eyre::eyre::{Context, Result};
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
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Add;

use MapEvent::{CommandFocus, Quit, StatusMessage};
use State::{Active, Normal};
use serde::{Deserialize, Serialize};

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
const WIDTH: u16 = 200;
const HEIGHT: u16 = 100;

#[derive(Debug, PartialEq, Clone, Copy)]
enum Brush {
    BgColor(Color),
    FgColor(Color),
    Char(char),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Drawing,
    Rectangle { anchor: Position },
    DeletingTerrain,
    DeletingRect { anchor: Position },
    PlacingToken { character: Option<char> },
    MovingToken(Token, Position),
    Prompt,
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Normal,
    Active(Mode),
}

#[derive(Debug)]
pub struct CellMap {
    size: (u16, u16),
    cursor: Cursor,
    cells: Vec<Vec<Cell>>,
    overlay: HashMap<Position, Cell>,
    bg_color_i: usize,
    fg_color_i: usize,
    char_i: usize,
    state: State,
    tokens: HashMap<Position, Vec<Token>>,
    brush: Brush,
    offset: Position,
    visible: (u16, u16),
    pub filename: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct MapState {
    size: (u16, u16),
    cells: Vec<CellRepr>,
    tokens: Vec<TokenRepr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct CellRepr {
    position: Position,
    #[serde(with = "color_serde")]
    bg_color: Color,
    #[serde(with = "color_serde")]
    fg_color: Color,
    character: char,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TokenRepr {
    position: Position,
    character: char,
    #[serde(with = "color_serde")]
    fg_color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Cell {
    bg_color: Color,
    fg_color: Color,
    character: char,
}

// but it makes feel kinda dumb lol
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    character: char,
    fg_color: Color,
}

pub enum MapEvent {
    Quit,
    CommandFocus,
    StatusMessage(String, Color),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}'", self.character)
    }
}

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

impl Default for Cell {
    fn default() -> Self {
        Self {
            fg_color: Color::White,
            bg_color: Color::Black,
            character: ' ',
        }
    }
}

impl CellMap {
    pub fn build() -> Self {
        let mut cells: Vec<Vec<Cell>> = Vec::new();
        // TODO Hardcoded because we need to implement other stuff
        for _ in 1..=WIDTH {
            let mut cs = Vec::<Cell>::new();
            for _ in 1..=HEIGHT {
                cs.push(Cell::default());
            }
            cells.push(cs);
        }

        Self {
            size: (WIDTH, HEIGHT),
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
            tokens: HashMap::new(),
            bg_color_i: 0,
            fg_color_i: 0,
            char_i: 0,
            state: Normal,
            brush: Brush::BgColor(Color::White),
            filename: None,
        }
    }

    pub fn update(&mut self) {
        match self.state {
            Active(Mode::Drawing) => self.paint(self.brush, self.cursor.pos + self.offset),
            Active(Mode::DeletingTerrain) => self.delete(),
            Active(Mode::Rectangle { anchor: a }) => {
                self.paint_rec(self.brush, a);
            }
            Active(Mode::DeletingRect { anchor: a }) => {
                self.delete_rect(a);
            }
            _ => {}
        }
    }

    pub fn save_map(&mut self, filename: String) -> Result<String> {
        let cells = self
            .cells
            .iter()
            .enumerate()
            .flat_map(|(i, row)| {
                row.iter().enumerate().filter_map(move |(j, c)| {
                    (*c != Cell::default()).then_some(CellRepr {
                        position: Position {
                            // we trust this won't go wrong
                            // but we'll now if it does
                            #[allow(clippy::cast_possible_truncation)]
                            x: i as u16,
                            #[allow(clippy::cast_possible_truncation)]
                            y: j as u16,
                        },
                        bg_color: c.bg_color,
                        fg_color: c.fg_color,
                        character: c.character,
                    })
                })
            })
            .collect();

        let tokens: Vec<TokenRepr> = self
            .tokens
            .iter()
            .flat_map(|(pos, tokens)| {
                tokens.iter().map(|t| TokenRepr {
                    position: *pos,
                    fg_color: t.fg_color,
                    character: t.character,
                })
            })
            .collect();

        let ms = MapState {
            size: self.size,
            cells,
            tokens,
        };
        let j = serde_json::to_string(&ms).wrap_err("Failed to serialize map")?;
        let mut f = File::create(&filename).wrap_err("Failed to open file 'test.json'")?;
        f.write(j.as_bytes())
            .wrap_err("Failed to write to file 'test.json'")?;
        self.filename = Some(filename);
        Ok(String::from("Succesfully written to 'test.json'"))
    }

    pub fn load_map(&mut self, filename: String) -> Result<()> {
        let mut f = File::open(&filename)?;
        let mut buffer = String::new();

        let _ = f.read_to_string(&mut buffer)?;
        let repr: MapState = serde_json::from_str(&buffer)?;

        let good_cells = repr
            .cells
            .iter()
            .map(|c| {
                (
                    (c.position.x, c.position.y),
                    Cell {
                        character: c.character,
                        fg_color: c.fg_color,
                        bg_color: c.bg_color,
                    },
                )
            })
            .collect::<HashMap<(u16, u16), Cell>>();

        let mut cells: Vec<Vec<Cell>> = Vec::new();

        for i in 0..repr.size.0 {
            let mut cs = Vec::<Cell>::new();
            for j in 0..repr.size.1 {
                if let Some(c) = good_cells.get(&(i, j)) {
                    cs.push(*c);
                } else {
                    cs.push(Cell::default());
                }
            }
            cells.push(cs);
        }

        let mut tokens = HashMap::<Position, Vec<Token>>::new();
        for t in repr.tokens {
            tokens.entry(t.position).or_default().push(Token {
                fg_color: t.fg_color,
                character: t.character,
            });
        }

        self.size = repr.size;
        self.cursor = Cursor {
            pos: Position { x: 1, y: 1 },
            prev_position: Position { x: 1, y: 1 },
            character: '@',
            fg_color: Color::Yellow,
        };
        self.cells = cells;
        self.tokens = tokens;
        self.offset = Position { x: 0, y: 0 };
        self.visible = (0, 0);
        self.overlay = HashMap::new();
        self.bg_color_i = 0;
        self.fg_color_i = 0;
        self.char_i = 0;
        self.state = Normal;
        self.brush = Brush::BgColor(Color::White);
        self.filename = Some(filename);

        Ok(())
    }

    fn delete(&mut self) {
        let pos = self.cursor.pos + self.offset;
        match self.brush {
            Brush::BgColor(_) => self.paint(Brush::BgColor(Color::Black), pos),
            Brush::FgColor(_) => self.paint(Brush::FgColor(Color::White), pos),
            Brush::Char(_) => self.paint(Brush::Char(' '), pos),
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
        let x = pos.x - 1;
        let y = pos.y - 1;
        let curr_cell = self
            .overlay
            .get(&Position { x, y })
            .copied()
            .unwrap_or_else(|| self.cells[x as usize][y as usize]);
        match brush {
            Brush::BgColor(c) => {
                let cell = Cell {
                    bg_color: c,
                    fg_color: curr_cell.fg_color,
                    character: curr_cell.character,
                };
                self.overlay.insert(Position { x, y }, cell);
            }
            Brush::Char(ch) => {
                let cell = Cell {
                    bg_color: curr_cell.bg_color,
                    fg_color: curr_cell.fg_color,
                    character: ch,
                };
                self.overlay.insert(Position { x, y }, cell);
            }
            Brush::FgColor(c) => {
                let cell = Cell {
                    bg_color: curr_cell.bg_color,
                    fg_color: c,
                    character: curr_cell.character,
                };
                self.overlay.insert(Position { x, y }, cell);
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

    fn token_at(&self) -> Option<Position> {
        if self
            .tokens
            .get(&(self.cursor.pos + self.offset))?
            .last()
            .is_some()
        {
            Some(self.cursor.pos + self.offset)
        } else {
            None
        }
    }

    const fn move_cursor(&mut self, dx: i16, dy: i16) {
        if self.visible.0 == 0 || self.visible.1 == 0 {
            return;
        }

        let new_x = (self.cursor.pos.x.cast_signed() + dx).cast_unsigned();
        let new_y = (self.cursor.pos.y.cast_signed() + dy).cast_unsigned();

        let max_x = if self.visible.0 < self.size.0 {
            self.visible.0
        } else {
            self.size.0 - self.offset.x
        };
        let max_y = if self.visible.1 < self.size.1 {
            self.visible.1
        } else {
            self.size.1 - self.offset.y
        };

        if new_x > max_x {
            if self.offset.x + self.visible.0 < self.size.0 {
                self.offset.x += 1;
            }
        } else if new_x == 0 {
            if self.offset.x > 0 {
                self.offset.x -= 1;
            }
        } else {
            self.cursor.pos.x = new_x;
        }

        if new_y > max_y {
            if self.offset.y + self.visible.1 < self.size.1 {
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
        for (pos, v) in &self.overlay {
            self.cells[pos.x as usize][pos.y as usize] = *v;
        }
    }

    fn push_token_to_cell(&mut self, t: Token, pos: Position) {
        self.tokens.entry(pos).or_default().push(t);
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
                Mode::MovingToken(t, _) => {
                    self.push_token_to_cell(t.clone(), self.cursor.pos + self.offset);
                    self.cursor.stay_here();
                }
                Mode::PlacingToken { character: Some(c) } => {
                    self.push_token_to_cell(
                        Token {
                            character: *c,
                            fg_color: PALETTE[self.fg_color_i],
                        },
                        self.cursor.pos + self.offset,
                    );
                    self.cursor.stay_here();
                }
                Mode::PlacingToken { character: None } | Mode::Prompt => {}
            }
        }
        self.overlay.clear();
        self.state = Normal;
    }

    fn revert(&mut self) {
        if let Active(Mode::MovingToken(t, origin)) = &self.state {
            self.push_token_to_cell(t.clone(), *origin);
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

    pub const fn set_mode(&mut self, state: State) {
        self.state = state;
    }

    fn handle_token_placement(&mut self, key_event: KeyEvent) -> bool {
        if let KeyCode::Char(c) = key_event.code
            && self.state == Active(Mode::PlacingToken { character: None })
            && !key_event.modifiers.contains(KeyModifiers::CONTROL)
        {
            if c != ' ' {
                self.state = Active(Mode::PlacingToken { character: Some(c) });
                self.cursor.fg_color = PALETTE[self.fg_color_i];
                self.cursor.character = c;
            }
            return true;
        }
        false
    }

    const fn handle_movement(&mut self, key_event: KeyEvent) -> bool {
        match key_event.code {
            KeyCode::Left | KeyCode::Char('h') => self.move_cursor(-1, 0),
            KeyCode::Right | KeyCode::Char('l') => self.move_cursor(1, 0),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(0, -1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(0, 1),
            KeyCode::Char('y') => self.move_cursor(-1, -1),
            KeyCode::Char('u') => self.move_cursor(1, -1),
            KeyCode::Char('b') => self.move_cursor(-1, 1),
            KeyCode::Char('n') => self.move_cursor(1, 1),
            _ => return false,
        }
        true
    }

    const fn handle_brush_selection(&mut self, key_event: KeyEvent) -> bool {
        match key_event.code {
            KeyCode::Tab => match self.brush {
                Brush::BgColor(_) => self.brush = Brush::FgColor(PALETTE[self.fg_color_i]),
                Brush::FgColor(_) => self.brush = Brush::Char(TERRAIN[self.char_i]),
                Brush::Char(_) => self.brush = Brush::BgColor(PALETTE[self.bg_color_i]),
            },
            KeyCode::BackTab => match self.brush {
                Brush::BgColor(_) => self.brush = Brush::Char(TERRAIN[self.char_i]),
                Brush::FgColor(_) => self.brush = Brush::BgColor(PALETTE[self.bg_color_i]),
                Brush::Char(_) => self.brush = Brush::FgColor(PALETTE[self.fg_color_i]),
            },
            KeyCode::Char(c @ '1'..='8') => self.change_brush(c),
            _ => return false,
        }
        true
    }

    fn handle_mode_transitions(&mut self, key_event: KeyEvent) -> bool {
        match key_event.code {
            KeyCode::Esc => self.revert(),
            KeyCode::Char(' ') => self.commit(),
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
                    && let Some(v) = self.tokens.get_mut(&i)
                    && self.state == Normal
                {
                    v.pop();
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
                if let Active(Mode::MovingToken(..)) = self.state {
                    self.commit();
                } else if self.state == Normal
                    && let Some(origin) = self.token_at()
                    && let Some(v) = self.tokens.get_mut(&origin)
                    && let Some(t) = v.pop()
                {
                    self.cursor.save_position();
                    self.state = Active(Mode::MovingToken(t.clone(), origin));
                    self.cursor.character = t.character;
                    self.cursor.fg_color = t.fg_color;
                }
            }
            _ => return false,
        }
        true
    }

    fn handle_global_commands(&mut self, key_event: KeyEvent) -> Option<MapEvent> {
        match key_event.code {
            KeyCode::Char('q') => Some(Quit),
            KeyCode::Char(':') => Some(CommandFocus),
            KeyCode::Char('s') => match self.save_map("test.json".to_string()) {
                Ok(_) => Some(StatusMessage(
                    "Map saved correctly :3".to_string(),
                    Color::Green,
                )),
                Err(err) => Some(StatusMessage(err.to_string(), Color::Red)),
            },
            KeyCode::Char('L') => match self.load_map("test.json".to_string()) {
                Ok(()) => Some(StatusMessage(
                    "Map loaded correctly :3".to_string(),
                    Color::Green,
                )),
                Err(err) => Some(StatusMessage(err.to_string(), Color::Red)),
            },
            _ => None,
        }
    }

    pub fn handle_events(&mut self, key_event: KeyEvent) -> Option<MapEvent> {
        if self.handle_token_placement(key_event) {
            return None;
        }

        if self.handle_movement(key_event) {
            return None;
        }

        if self.handle_brush_selection(key_event) {
            return None;
        }

        if self.handle_mode_transitions(key_event) {
            return None;
        }

        self.handle_global_commands(key_event)
    }

    fn draw_map(&self, area: Rect, buf: &mut Buffer) {
        for x in 1..=self.visible.0 {
            for y in 1..=self.visible.1 {
                let map_x = self.offset.x + x;
                let map_y = self.offset.y + y;

                if !(1..=self.size.0).contains(&map_x) || !(1..=self.size.1).contains(&map_y) {
                    continue;
                }

                let (nx, ny) = (map_x as usize - 1, map_y as usize - 1);

                if let Some(c) = self.overlay.get(&Position {
                    x: map_x - 1,
                    y: map_y - 1,
                }) {
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

    const fn get_mode_prompt(&self) -> &str {
        match self.state {
            Active(Mode::Drawing | Mode::Rectangle { anchor: _ }) => "DRAWING",
            Active(Mode::DeletingTerrain | Mode::DeletingRect { anchor: _ }) => "DELETING",
            Normal => "EXPLORING",
            Active(Mode::MovingToken(..)) => "MOVING",
            Active(Mode::PlacingToken { character: None }) => "PLACING (waiting...)",
            Active(Mode::PlacingToken { character: Some(_) }) => "PLACING (be nice!)",
            Active(Mode::Prompt) => "PROMPTING...",
        }
    }

    const fn get_mode_color(&self) -> Color {
        match self.state {
            Active(Mode::Drawing | Mode::Rectangle { anchor: _ }) => Color::Magenta,
            Active(Mode::DeletingTerrain | Mode::DeletingRect { anchor: _ }) => Color::Red,
            Normal => Color::White,
            Active(Mode::MovingToken(..)) => Color::Yellow,
            Active(Mode::PlacingToken { character: _ }) => Color::Cyan,
            Active(Mode::Prompt) => Color::Green,
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

        for (pos, tokens) in &self.tokens {
            if let Some(t) = tokens.last() {
                if pos.x <= self.offset.x
                    || pos.x > self.offset.x + self.visible.0
                    || pos.y <= self.offset.y
                    || pos.y > self.offset.y + self.visible.1
                {
                    continue;
                }
                let screen_x = area.x + pos.x - self.offset.x;
                let screen_y = area.y + pos.y - self.offset.y;
                buf[(screen_x, screen_y)]
                    .set_char(t.character)
                    .set_fg(t.fg_color)
                    .set_style(Modifier::BOLD);
            }
        }

        buf[(area.x + self.cursor.pos.x, area.y + self.cursor.pos.y)]
            .set_char(self.cursor.character)
            .set_fg(self.cursor.fg_color)
            .set_style(Modifier::BOLD);

        let title = Line::from(vec![
            self.get_mode_prompt().into(),
            " ".into(),
            self.filename.as_ref().map_or("Untitled", |f| f).into(),
        ]);

        let current_color = Line::from(format!("COLOR: {:?}", self.brush,));

        let block = Block::bordered()
            .title(title)
            .title_bottom(current_color)
            .border_style(self.get_mode_color())
            .border_set(border::THICK);
        block.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[derive(Serialize, Deserialize)]
    struct ColorWrapper(#[serde(with = "color_serde")] Color);

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

    #[test]
    fn test_token_repr_roundtrip() {
        let token = TokenRepr {
            position: Position { x: 3, y: 7 },
            character: 'G',
            fg_color: Color::Green,
        };
        let serialized = serde_json::to_string(&token).unwrap();
        let deserialized: TokenRepr = serde_json::from_str(&serialized).unwrap();
        assert_eq!(token, deserialized);
    }

    #[test]
    fn test_token_stack_order_preserved() {
        // Two tokens stacked at the same position — bottom first, top last.
        let pos = Position { x: 2, y: 2 };
        let ms = MapState {
            size: (10, 10),
            cells: vec![],
            tokens: vec![
                TokenRepr {
                    position: pos,
                    character: 'A',
                    fg_color: Color::Red,
                },
                TokenRepr {
                    position: pos,
                    character: 'B',
                    fg_color: Color::Blue,
                },
            ],
        };

        let json = serde_json::to_string(&ms).unwrap();
        let loaded: MapState = serde_json::from_str(&json).unwrap();

        // Rebuild the HashMap the same way load_map does.
        let mut tokens = HashMap::<Position, Vec<Token>>::new();
        for t in loaded.tokens {
            tokens.entry(t.position).or_default().push(Token {
                fg_color: t.fg_color,
                character: t.character,
            });
        }

        let stack = tokens.get(&pos).unwrap();
        assert_eq!(stack[0].character, 'A');
        assert_eq!(stack[1].character, 'B');
        assert_eq!(stack.last().unwrap().character, 'B');
    }
}
