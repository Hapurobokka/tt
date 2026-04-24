use std::fmt;
use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
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

#[derive(Debug, Default)]
pub struct Cell {
    bg_color: Color,
    character: char,
}

#[derive(Debug)]
struct Cursor {
    x: u16,
    y: u16,
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

#[derive(Debug, PartialEq)]
enum State {
    Normal,
    Drawing(Color),
    Deleting,
    Moving(usize),
}
use State::*;

#[derive(Debug)]
pub struct App {
    exit: bool,
    cursor: Cursor,
    cells: Vec<Vec<Cell>>,
    color_i: usize,
    state: State,
    tokens: Vec<Token>,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let s = terminal.size()?;
        for _ in 1..s.width - 1 {
            let mut cs = Vec::<Cell>::new();
            for _ in 1..s.height - 1 {
                cs.push(Cell {
                    bg_color: Color::Reset,
                    character: '.',
                });
            }
            self.cells.push(cs);
        }

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events(s)?;
            match self.state {
                Drawing(c) => self.paint(c),
                Deleting => self.paint(Color::Reset),
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

    fn paint(&mut self, color: Color) {
        self.cells[self.cursor.x as usize - 1][self.cursor.y as usize - 1].bg_color = color
    }

    fn token_at(&self) -> Option<usize> {
        self.tokens
            .iter()
            .position(|t| t.x == self.cursor.x && t.y == self.cursor.y)
    }

    fn handle_key_press(&mut self, key_event: KeyEvent, size: Size) {
        match key_event.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Left | KeyCode::Char('h') => {
                if self.cursor.x > 1 {
                    self.cursor.x -= 1
                }
                if let Moving(i) = self.state {
                    self.tokens[i].x = self.cursor.x;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.cursor.x <= size.width - 3 {
                    self.cursor.x += 1
                }
                if let Moving(i) = self.state {
                    self.tokens[i].x = self.cursor.x;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor.y > 1 {
                    self.cursor.y -= 1
                }
                if let Moving(i) = self.state {
                    self.tokens[i].y = self.cursor.y;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor.y <= size.height - 3 {
                    self.cursor.y += 1
                }
                if let Moving(i) = self.state {
                    self.tokens[i].y = self.cursor.y;
                }
            }
            KeyCode::BackTab => {
                if self.color_i == 0 {
                    self.color_i = PALETTE.len() - 1
                } else {
                    self.color_i -= 1
                }
                if let Drawing(_) = self.state {
                    self.state = Drawing(PALETTE[self.color_i])
                }
            }
            KeyCode::Tab => {
                if self.color_i == PALETTE.len() - 1 {
                    self.color_i = 0
                } else {
                    self.color_i += 1
                }
                if let Drawing(_) = self.state {
                    self.state = Drawing(PALETTE[self.color_i])
                }
            }
            KeyCode::Char(' ') => match self.state {
                Drawing(_) => self.state = Normal,
                Normal | Deleting => self.state = Drawing(PALETTE[self.color_i]),
                _ => {}
            },
            KeyCode::Char('x') => {
                if self.state == Deleting {
                    self.state = Normal
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
                        self.state = Moving(i);
                        self.cursor.character = self.tokens[i].character;
                    }
                }
            },
            KeyCode::Esc => self.state = Normal,
            _ => {}
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in 1..area.width - 1 {
            for y in 1..area.height - 1 {
                let (dx, dy) = (area.x + x, area.y + y);
                let (nx, ny) = (x as usize - 1, y as usize - 1);
                buf[(dx, dy)]
                    .set_char(self.cells[nx][ny].character)
                    .set_bg(self.cells[nx][ny].bg_color);
            }
        }

        for t in &self.tokens {
            buf[(t.x, t.y)].set_char(t.character).set_fg(t.fg_color);
        }

        buf[(area.x + self.cursor.x, area.y + self.cursor.y)]
            .set_char(self.cursor.character)
            .set_fg(self.cursor.fg_color);

        let title = Line::from(match self.state {
            Drawing(_) => "DRAWING",
            Deleting => "DELETING",
            Normal => "EXPLORING",
            Moving(_) => "MOVING",
        });

        let current_color = Line::from(format!(
            "COLOR: {} | {}",
            PALETTE[self.color_i],
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
            character: '@',
            fg_color: Color::Yellow,
        },
        cells: Vec::new(),
        tokens: Vec::new(),
        color_i: 0,
        state: Normal,
    };
    ratatui::run(|terminal| app.run(terminal))
}
