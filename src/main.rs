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
struct Token {
    x: u16,
    y: u16,
    character: char,
    fg_color: Color,
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
    player_x: u16,
    player_y: u16,
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
                Drawing(c) => {
                    self.paint(c);
                }
                Deleting => {
                    self.paint(Color::Reset);
                }
                Moving(_) => todo!(),
                Normal => {}
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
        self.cells[self.player_x as usize - 1][self.player_y as usize - 1].bg_color = color
    }

    fn handle_key_press(&mut self, key_event: KeyEvent, size: Size) {
        match key_event.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Left | KeyCode::Char('h') => {
                if self.player_x > 1 {
                    self.player_x -= 1
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.player_x <= size.width - 3 {
                    self.player_x += 1
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.player_y > 1 {
                    self.player_y -= 1
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.player_y <= size.height - 3 {
                    self.player_y += 1
                }
            }
            KeyCode::BackTab => {
                if self.color_i == 0 {
                    self.color_i = PALETTE.len() - 1
                } else {
                    self.color_i -= 1
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
                self.tokens.push(Token {
                    x: self.player_x,
                    y: self.player_y,
                    character: 't',
                    fg_color: Color::Red,
                });
            }
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

        buf[(area.x + self.player_x, area.y + self.player_y)].set_char('@');

        let title = Line::from(if let Drawing(__) = self.state {
            "DRAWING"
        } else if self.state == Deleting {
            "DELETING "
        } else {
            "TEST"
        });

        let current_color = Line::from(format!("COLOR: {}", PALETTE[self.color_i]));
        let block = Block::bordered()
            .title(title)
            .title_bottom(current_color)
            .border_style(if let Drawing(__) = self.state {
                Color::Magenta
            } else if self.state == Deleting {
                Color::Red
            } else {
                Color::White
            })
            .border_set(border::THICK);
        block.render(area, buf);
    }
}

fn main() -> io::Result<()> {
    let mut app = App {
        exit: false,
        player_x: 1,
        player_y: 1,
        cells: Vec::new(),
        color_i: 0,
        state: Normal,
        tokens: Vec::new(),
    };
    ratatui::run(|terminal| app.run(terminal))
}
