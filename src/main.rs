use std::collections::HashMap;
use std::fmt;
use std::io;

use crossterm::event::KeyModifiers;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::style::Modifier;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Rect, Size},
    style::Color,
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

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
    PlacingToken,
    MovingToken(usize),
}

#[derive(Debug, PartialEq)]
enum State {
    Normal,
    Active(Mode),
}
use State::{Active, Normal};

#[derive(Debug)]
pub struct App {
    exit: bool,
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

impl Cursor {
    const fn reset_cursor(&mut self) {
        self.pos.x = self.prev_position.x;
        self.pos.y = self.prev_position.y;
        self.character = '@';
    }

    const fn save_position(&mut self) {
        self.prev_position = Position {
            x: self.pos.x,
            y: self.pos.y,
        };
    }
}

impl App {
    /// # Errors
    ///
    /// It will return an error if it fails to read the terminal's size
    /// or if it cant draw itself in the buffer
    /// or if can't read events from crossterm
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let s = terminal.size()?;
        for _ in 1..s.width - 1 {
            let mut cs = Vec::<Cell>::new();
            for _ in 1..s.height - 1 {
                cs.push(Cell {
                    fg_color: Color::White,
                    bg_color: Color::Black,
                    character: ' ',
                });
            }
            self.cells.push(cs);
        }

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events(s)?;
            match self.state {
                Active(Mode::Drawing) => self.paint(self.brush, self.cursor.pos),
                Active(Mode::DeletingTerrain) => self.delete(),
                Active(Mode::MovingToken(i)) => {
                    self.tokens[i].pos.x = self.cursor.pos.x;
                    self.tokens[i].pos.y = self.cursor.pos.y;
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
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self, size: Size) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_press(key_event, size);
            }
            _ => {}
        }
        Ok(())
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
        self.tokens
            .iter()
            .position(|t| t.pos.x == self.cursor.pos.x && t.pos.y == self.cursor.pos.y)
    }

    const fn move_cursor(&mut self, dx: i16, dy: i16, size: Size) {
        let new_x = self.cursor.pos.x.cast_signed() + dx;
        let new_y = self.cursor.pos.y.cast_signed() + dy;
        if new_x >= 1 && new_x <= size.width.cast_signed() - 2 {
            self.cursor.pos.x = new_x.cast_unsigned();
        }
        if new_y >= 1 && new_y <= size.height.cast_signed() - 2 {
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
                    self.cursor.character = '@';
                }
                Mode::PlacingToken => todo!(),
            }
        }
        self.overlay.clear();
        self.state = Normal;
    }

    fn revert(&mut self) {
        if let Active(Mode::MovingToken(i)) = self.state {
            self.tokens[i].pos.x = self.cursor.prev_position.x;
            self.tokens[i].pos.y = self.cursor.prev_position.y;
            self.cursor.character = '@';
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

    fn handle_key_press(&mut self, key_event: KeyEvent, size: Size) {
        match key_event.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Left | KeyCode::Char('h') => self.move_cursor(-1, 0, size),
            KeyCode::Right | KeyCode::Char('l') => self.move_cursor(1, 0, size),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(0, -1, size),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(0, 1, size),
            KeyCode::Char('y') => self.move_cursor(-1, -1, size),
            KeyCode::Char('u') => self.move_cursor(1, -1, size),
            KeyCode::Char('b') => self.move_cursor(-1, 1, size),
            KeyCode::Char('n') => self.move_cursor(1, 1, size),
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
            // TODO
            // It seems that we can go from d to D, and maybe we don't want to
            // do that (or it should at least be smoother)
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
                if self.state == Normal {
                    self.tokens.push(Token {
                        pos: Position {
                            x: self.cursor.pos.x,
                            y: self.cursor.pos.y,
                        },
                        character: 't',
                        fg_color: Color::Red,
                    });
                }
            }
            KeyCode::Char('m') => {
                if let Some(i) = self.token_at()
                    && self.state == Normal
                {
                    self.cursor.save_position();
                    self.state = Active(Mode::MovingToken(i));
                    self.cursor.character = self.tokens[i].character;
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

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in 1..area.width - 1 {
            for y in 1..area.height - 1 {
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
            Active(Mode::PlacingToken) => todo!(),
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
                Active(Mode::PlacingToken) => todo!(),
            })
            .border_set(border::THICK);
        block.render(area, buf);
    }
}

fn main() -> io::Result<()> {
    let mut app = App {
        exit: false,
        cursor: Cursor {
            pos: Position { x: 1, y: 1 },
            prev_position: Position { x: 1, y: 1 },
            character: '@',
            fg_color: Color::Yellow,
        },
        cells: Vec::new(),
        overlay: HashMap::new(),
        tokens: Vec::new(),
        bg_color_i: 0,
        fg_color_i: 0,
        char_i: 0,
        state: Normal,
        brush: Brush::BgColor(Color::White),
    };
    ratatui::run(|terminal| app.run(terminal))
}
