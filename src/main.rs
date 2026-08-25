//! Eliza — TUI front end for the classic Creative Computing ELIZA engine.
//!
//! Mirrors the original program's screen: red title block, white Eliza text,
//! green user input (COLOR 10/12/15 in the BASIC), "? " INPUT prompt, and
//! "Shut up..." ending the program on a "SHUT"/"shut" input.



use eliza::{Eliza, Outcome};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::io;

const GREETING: &str = "Hi!  I'm Eliza.  What's your problem?";

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

struct App {
    engine: Eliza,
    /// (text, color) lines of the conversation, newest last.
    log: Vec<(String, Color)>,
    input: String,
    cursor: usize,
    quit: bool,
}

impl App {
    fn submit(&mut self) {
        let input = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.log.push((format!("? {input}"), Color::LightGreen));
        match self.engine.respond(&input) {
            Outcome::Say(text) => self.log.push((text, Color::White)),
            Outcome::ShutUp => {
                self.log.push(("Shut up...".to_string(), Color::White));
                self.quit = true;
            }
        }
    }
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App {
        engine: Eliza::new(),
        log: vec![(GREETING.to_string(), Color::White)],
        input: String::new(),
        cursor: 0,
        quit: false,
    };
    loop {
        terminal.draw(|f| draw(f, &mut app))?;
        if app.quit {
            return Ok(());
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc | KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(())
                }
                KeyCode::Enter => app.submit(),
                KeyCode::Backspace => {
                    if app.cursor > 0 {
                        app.cursor -= 1;
                        app.input.remove(app.cursor);
                    }
                }
                KeyCode::Delete => {
                    if app.cursor < app.input.len() {
                        app.input.remove(app.cursor);
                    }
                }
                KeyCode::Left => app.cursor = app.cursor.saturating_sub(1),
                KeyCode::Right => app.cursor = (app.cursor + 1).min(app.input.len()),
                KeyCode::Home => app.cursor = 0,
                KeyCode::End => app.cursor = app.input.len(),
                KeyCode::Char(c) => {
                    app.input.insert(app.cursor, c);
                    app.cursor += 1;
                }
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn draw(f: &mut Frame, app: &mut App) {
    let [header, chat, input] = Layout::vertical([
        Constraint::Length(4), // title block + one blank row
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(f.area());
    draw_header(f, header);
    draw_chat(f, chat, &app.log);
    draw_input(f, input, &app.input, app.cursor);
}

/// Title block, padded exactly like TAB(37)/TAB(31)/TAB(29) on the original
/// 80-column screen. Light red = COLOR 12.
fn draw_header(f: &mut Frame, area: Rect) {
    let style = Style::default().fg(Color::LightRed);
    let lines = vec![
        Line::raw(" ".repeat(36) + "Eliza"),
        Line::raw(" ".repeat(30) + "Creative Computing"),
        Line::raw(" ".repeat(28) + "Morristown, New Jersey"),
    ];
    f.render_widget(Paragraph::new(lines).style(style).block(Block::default()), area);
}

/// Conversation tail: wrap to width, keep the newest lines that fit.
fn draw_chat(f: &mut Frame, area: Rect, log: &[(String, Color)]) {
    let width = area.width.max(1) as usize;
    let mut lines: Vec<Line> = Vec::new();
    for (text, color) in log {
        for chunk in text.split('\n') {
            wrap(chunk, width, &mut lines, *color);
        }
    }
    let height = area.height as usize;
    let scroll = lines.len().saturating_sub(height) as u16;
    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

fn wrap(text: &str, width: usize, out: &mut Vec<Line>, color: Color) {
    let style = Style::default().fg(color);
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + width).min(text.len());
        while end > start && !text.is_char_boundary(end) {
            end -= 1;
        }
        out.push(Line::from(Span::styled(text[start..end].to_string(), style)));
        start = end;
    }
}

/// Input line: green "? " prompt + text, like INPUT's echo under COLOR 10.
fn draw_input(f: &mut Frame, area: Rect, input: &str, cursor: usize) {
    let style = Style::default().fg(Color::LightGreen);
    let line = Line::from(vec![
        Span::styled("? ", style),
        Span::styled(input, style),
    ]);
    let cols = area.x + "? ".len() as u16 + cursor as u16;
    f.set_cursor_position((cols, area.y));
    f.render_widget(Paragraph::new(line).style(style), area);
}
