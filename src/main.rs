use std::collections::HashMap;
use std::fmt;
use std::io;

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
    Color::Black,
    Color::Red,
    Color::Blue,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Gray,
];

const TERRAIN: [char; 8] = ['.', '#', '|', '"', '-', '+', '<', '>'];

#[derive(Debug, Default, Clone)]
pub struct Cell {
    bg_color: Color,
    fg_color: Color,
    character: char,
}

#[derive(Debug)]
struct Cursor {
    x: u16,
    y: u16,
    prev_position: (u16, u16),
    character: char,
    fg_color: Color,
}

#[derive(Debug)]
struct Token {
    x: u16,
    y: u16,
    character: char,
    fg_color: Color,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' ({}, {})", self.character, self.x, self.y)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Brush {
    BgColor(Color),
    FgColor(Color),
    Char(char),
}

#[derive(Debug, PartialEq)]
enum State {
    Normal,
    Drawing(Brush),
    Deleting,
    Moving(usize),
}
use State::{Deleting, Drawing, Moving, Normal};

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
        self.x = self.prev_position.0;
        self.y = self.prev_position.1;
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
                    bg_color: Color::Reset,
                    character: ' ',
                });
            }
            self.cells.push(cs);
        }

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events(s)?;
            match self.state {
                Drawing(b) => self.paint(b),
                Deleting => self.delete(),
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

    fn paint(&mut self, brush: Brush) {
        let nx = self.cursor.x as usize - 1;
        let ny = self.cursor.y as usize - 1;
        let curr_cell = self
            .overlay
            .get(&(nx, ny))
            .cloned()
            .unwrap_or_else(|| self.cells[nx][ny].clone());
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

    fn delete(&mut self) {
        match self.brush {
            Brush::BgColor(_) => self.paint(Brush::BgColor(Color::Reset)),
            Brush::FgColor(_) => self.paint(Brush::BgColor(Color::White)),
            Brush::Char(_) => self.paint(Brush::Char(' ')),
        }
    }

    fn token_at(&self) -> Option<usize> {
        self.tokens
            .iter()
            .position(|t| t.x == self.cursor.x && t.y == self.cursor.y)
    }

    fn move_cursor(&mut self, dx: i16, dy: i16, size: Size) {
        let new_x = self.cursor.x.cast_signed() + dx;
        let new_y = self.cursor.y.cast_signed() + dy;
        if new_x >= 1 && new_x <= size.width.cast_signed() - 2 {
            self.cursor.x = new_x.cast_unsigned();
        }
        if new_y >= 1 && new_y <= size.height.cast_signed() - 2 {
            self.cursor.y = new_y.cast_unsigned();
        }
        if let Moving(i) = self.state {
            self.tokens[i].x = self.cursor.x;
            self.tokens[i].y = self.cursor.y;
        }
    }

    const fn sync_brush(&mut self) {
        if let Drawing(_) = self.state {
            self.state = Drawing(self.brush);
        }
    }

    fn commit_overlay(&mut self) {
        for ((x, y), v) in &self.overlay {
            self.cells[*x][*y] = v.clone();
        }
        self.overlay.clear();
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
            KeyCode::Tab => {
                match self.brush {
                    Brush::BgColor(_) => self.brush = Brush::FgColor(PALETTE[self.fg_color_i]),
                    Brush::FgColor(_) => self.brush = Brush::Char(TERRAIN[self.char_i]),
                    Brush::Char(_) => self.brush = Brush::BgColor(PALETTE[self.bg_color_i]),
                }

                self.sync_brush();
            }
            KeyCode::Char(' ') => match self.state {
                Drawing(_) => {
                    self.commit_overlay();
                    self.state = Normal;
                }
                Normal | Deleting => {
                    self.cursor.prev_position = (self.cursor.x, self.cursor.y);
                    self.state = Drawing(self.brush);
                }
                Moving(_) => {}
            },
            KeyCode::Char('x') => {
                if self.state == Deleting {
                    self.state = Normal;
                } else {
                    self.state = Deleting;
                }
            }
            KeyCode::Char('t') => {
                if self.state == Normal {
                    self.tokens.push(Token {
                        x: self.cursor.x,
                        y: self.cursor.y,
                        character: 't',
                        fg_color: Color::Red,
                    });
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.token_at()
                    && self.state == Normal
                {
                    self.tokens.remove(i);
                }
            }
            KeyCode::Char('m') => match self.state {
                Moving(_) => {
                    self.state = Normal;
                    self.cursor.character = '@';
                }
                _ => {
                    if let Some(i) = self.token_at() {
                        self.cursor.prev_position = (self.cursor.x, self.cursor.y);
                        self.state = Moving(i);
                        self.cursor.character = self.tokens[i].character;
                    }
                }
            },
            KeyCode::Char(c @ '1'..='8') => {
                let i = (c as usize) - ('1' as usize);
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
                self.sync_brush();
            }
            KeyCode::Esc => {
                if let Moving(i) = self.state {
                    self.tokens[i].x = self.cursor.prev_position.0;
                    self.tokens[i].y = self.cursor.prev_position.1;
                    self.cursor.character = '@';
                }
                self.overlay.clear();
                self.cursor.reset_cursor();
                self.state = Normal;
            }
            _ => {}
        }
    }
}

macro_rules! key_hints {
    ($(($key:expr, $desc:expr)),+) => {
        Line::from(vec![
            $(
                format!("[{}]: {} ", $key, $desc).into(),
            )+
        ])
    }
}

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
            buf[(t.x, t.y)]
                .set_char(t.character)
                .set_fg(t.fg_color)
                .set_style(Modifier::BOLD);
        }

        buf[(area.x + self.cursor.x, area.y + self.cursor.y)]
            .set_char(self.cursor.character)
            .set_fg(self.cursor.fg_color)
            .set_style(Modifier::BOLD);

        let title = Line::from(match self.state {
            Drawing(_) => "DRAWING",
            Deleting => "DELETING",
            Normal => "EXPLORING",
            Moving(_) => "MOVING",
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
                Drawing(_) => Color::Magenta,
                Deleting => Color::Red,
                Moving(_) => Color::Yellow,
                Normal => Color::White,
            })
            .border_set(border::THICK);
        block.render(area, buf);
    }
}

fn main() -> io::Result<()> {
    let mut app = App {
        exit: false,
        cursor: Cursor {
            x: 1,
            y: 1,
            prev_position: (1, 1),
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
