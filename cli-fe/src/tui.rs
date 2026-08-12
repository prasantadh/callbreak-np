//! The ratatui interface: four seats around a table, played cards clustered in
//! the centre, opponents holding face-down stand-in hands, the viewer's hand
//! along the bottom with legal cards raised, and the tricks they have won just
//! beneath it. Scores stay on screen at all times.
//!
//! It renders from a stream of [`Update`]s and emits `ClientMessage`s, so the
//! same loop can later drive a replay by swapping the channel's source. A future
//! debug mode can reveal opponents' hands — with every seat's transport owned by
//! `cli-fe`, that data is already observable (see `host_mode`). No tracing here:
//! logging to the terminal would corrupt the alternate screen.

use crate::game_view::{TableState, Update};
use callbreak::agent::{Action, ClientMessage};
use callbreak::game::{Call, Card, Rank, Suit};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use std::io;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

const CARD_W: u16 = 4;
const CARD_H: u16 = 2;

pub fn run(me: String, updates: Receiver<Update>, moves: Sender<ClientMessage>) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(me);
    let result = app.event_loop(&mut terminal, &updates, &moves);
    ratatui::restore();
    result
}

struct App {
    me: String,
    table: Option<TableState>,
    /// The viewer's hand and legal moves, from the last prompt.
    hand: Vec<Card>,
    playable: Vec<Card>,
    action: Action,
    /// True while the current prompt is unanswered.
    awaiting: bool,
    /// Cursor into `playable` while breaking.
    cursor: usize,
    /// Pending call value while calling.
    call_value: u8,
    game_over: bool,
    status: String,
}

impl App {
    fn new(me: String) -> Self {
        Self {
            me,
            table: None,
            hand: vec![],
            playable: vec![],
            action: Action::Call,
            awaiting: false,
            cursor: 0,
            call_value: 1,
            game_over: false,
            status: "Waiting for the game to start…".to_string(),
        }
    }

    fn event_loop(
        &mut self,
        terminal: &mut DefaultTerminal,
        updates: &Receiver<Update>,
        moves: &Sender<ClientMessage>,
    ) -> io::Result<()> {
        loop {
            match updates.try_recv() {
                Ok(update) => self.on_update(update),
                Err(TryRecvError::Empty) => {}
                // The host thread finished (or died): the game is over.
                Err(TryRecvError::Disconnected) => self.game_over = true,
            }

            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        code => self.on_key(code, moves),
                    }
                }
            }
        }
    }

    fn on_update(&mut self, update: Update) {
        match update {
            // Someone else acted: refresh the shared table, leave the prompt be.
            Update::Table(table) => self.table = Some(table),
            Update::Prompt {
                table,
                hand,
                action,
                playable,
            } => {
                self.table = Some(table);
                self.hand = hand;
                self.playable = playable;
                self.action = action;
                self.awaiting = true;
                self.cursor = 0;
                self.call_value = 1;
            }
        }
    }

    fn on_key(&mut self, code: KeyCode, moves: &Sender<ClientMessage>) {
        if !self.awaiting {
            return;
        }
        match self.action {
            // Only ever set from a prompt, so this seat is never waiting on an
            // update — and there is no key that would answer one.
            Action::Update => {}
            Action::Call => match code {
                KeyCode::Up => self.call_value = (self.call_value + 1).min(13),
                KeyCode::Down => self.call_value = self.call_value.saturating_sub(1).max(1),
                KeyCode::Enter => {
                    if let Ok(call) = Call::new(self.call_value) {
                        self.submit(ClientMessage::Call(call), moves);
                    }
                }
                _ => {}
            },
            Action::Break => {
                let count = self.playable.len();
                if count == 0 {
                    return;
                }
                match code {
                    KeyCode::Left => self.cursor = (self.cursor + count - 1) % count,
                    KeyCode::Right => self.cursor = (self.cursor + 1) % count,
                    KeyCode::Enter => {
                        let card = self.playable[self.cursor];
                        if self.submit(ClientMessage::Break(card), moves) {
                            // Show the play at once; the authoritative update follows.
                            self.hand.retain(|c| *c != card);
                            if let Some(table) = &mut self.table {
                                table.trick[table.me] = Some(card);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn submit(&mut self, message: ClientMessage, moves: &Sender<ClientMessage>) -> bool {
        if moves.send(message).is_ok() {
            self.awaiting = false;
            self.status = "Waiting for other players…".to_string();
            true
        } else {
            false
        }
    }

    fn render(&self, f: &mut Frame) {
        let outer = Block::default().borders(Borders::ALL).title(self.title());
        let inner = outer.inner(f.area());
        f.render_widget(outer, f.area());

        let rows = Layout::vertical([
            Constraint::Min(11),   // table + scoreboard
            Constraint::Length(6), // hand
            Constraint::Length(4), // tricks won
            Constraint::Length(1), // status
        ])
        .split(inner);

        let top = Layout::horizontal([Constraint::Min(30), Constraint::Length(36)]).split(rows[0]);
        self.render_table(f, top[0]);
        self.render_scores(f, top[1]);
        self.render_hand(f, rows[1]);
        self.render_won(f, rows[2]);
        self.render_status(f, rows[3]);
    }

    fn title(&self) -> String {
        match &self.table {
            Some(t) => format!(" Callbreak — Round {}/5 — you are {} ", t.round, self.me),
            None => " Callbreak ".to_string(),
        }
    }

    fn render_table(&self, f: &mut Frame, area: Rect) {
        let Some(table) = &self.table else {
            return;
        };
        let cx = area.x + area.width / 2;
        let cy = area.y + area.height / 2;

        // Centre trick, a tight diamond around the middle: [bottom (me), left,
        // top, right] following seat order from the viewer.
        let centre = [
            (cx.saturating_sub(CARD_W / 2), cy + 1),
            (cx.saturating_sub(CARD_W + 3), cy.saturating_sub(1)),
            (cx.saturating_sub(CARD_W / 2), cy.saturating_sub(3)),
            (cx + 3, cy.saturating_sub(1)),
        ];
        // Name/score labels near each edge.
        let label = [
            Rect::new(cx.saturating_sub(12), area.y + area.height - 1, 24, 1),
            Rect::new(area.x, cy.saturating_sub(4), 18, 1),
            Rect::new(cx.saturating_sub(12), area.y, 24, 1),
            Rect::new(
                area.x + area.width.saturating_sub(18),
                cy.saturating_sub(4),
                18,
                1,
            ),
        ];
        // Face-down stand-in hands for the three opponents.
        let stand_in = [
            None,
            Some((area.x + 1, cy.saturating_sub(3))),
            Some((cx.saturating_sub(3), area.y + 1)),
            Some((
                area.x + area.width.saturating_sub(CARD_W + 3),
                cy.saturating_sub(3),
            )),
        ];

        for offset in 0..4usize {
            let seat_idx = (table.me + offset) % 4;
            let seat = &table.seats[seat_idx];

            if let Some(card) = table.trick[seat_idx] {
                let winner = table.trick_winner == Some(seat_idx);
                let (x, y) = centre[offset];
                render_center_card(f, x, y, card, winner);
            }

            if let Some((x, y)) = stand_in[offset] {
                render_back_fan(f, x, y, seat.hand_size);
            }

            let style = if offset == 0 {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            f.render_widget(
                Paragraph::new(Line::styled(seat.name.clone(), style)).alignment(Alignment::Center),
                label[offset],
            );
        }

        // While calling there is no trick in the centre; show the pending call.
        if self.awaiting && matches!(self.action, Action::Call) {
            f.render_widget(
                Paragraph::new(Line::styled(
                    format!("Your call: {}", self.call_value),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
                .alignment(Alignment::Center),
                Rect::new(cx.saturating_sub(10), cy, 20, 1),
            );
        }
    }

    fn render_scores(&self, f: &mut Frame, area: Rect) {
        let Some(table) = &self.table else {
            return;
        };
        let block = Block::default().borders(Borders::ALL).title(" Scores ");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let header = |i: usize| table.seats[i].name.chars().take(6).collect::<String>();
        let row = |label: &str, cells: [String; 4]| {
            Line::raw(format!(
                "{:<5}{:>7}{:>7}{:>7}{:>7}",
                label, cells[0], cells[1], cells[2], cells[3]
            ))
        };

        let mut totals = [0f32; 4];
        for scores in &table.round_scores {
            for i in 0..4 {
                totals[i] += scores[i];
            }
        }

        let mut lines = vec![row("", std::array::from_fn(header))];
        for r in 0..5 {
            let cells = std::array::from_fn(|i| match table.round_scores.get(r) {
                Some(scores) => format!("{:+.1}", scores[i]),
                None => "-".to_string(),
            });
            lines.push(row(&format!("R{}", r + 1), cells));
        }
        lines.push(row(
            "Tot",
            std::array::from_fn(|i| format!("{:+.1}", totals[i])),
        ));
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_hand(&self, f: &mut Frame, area: Rect) {
        if self.hand.is_empty() {
            f.render_widget(
                Paragraph::new("Waiting for the game to start…").alignment(Alignment::Center),
                area,
            );
            return;
        }

        let breaking = matches!(self.action, Action::Break);
        let step = CARD_W + 1;
        let total = (self.hand.len() as u16)
            .saturating_mul(step)
            .saturating_sub(1);
        let start_x = area.x + area.width.saturating_sub(total) / 2;
        let base_y = area.y + 2;

        for (i, card) in self.hand.iter().enumerate() {
            let x = start_x + i as u16 * step;
            let raised = self.awaiting && breaking && self.playable.contains(card);
            let selected =
                self.awaiting && breaking && self.playable.get(self.cursor) == Some(card);
            render_hand_card(f, x, base_y, *card, raised, selected);
        }
    }

    fn render_won(&self, f: &mut Frame, area: Rect) {
        let Some(table) = &self.table else {
            return;
        };
        f.render_widget(
            Paragraph::new(Line::styled(
                format!("Your tricks: {}", table.my_won.len()),
                Style::default().fg(Color::DarkGray),
            )),
            Rect::new(area.x, area.y, area.width, 1),
        );

        // Every won trick as its four cards, the viewer's winning card starred.
        let card_step = CARD_W + 1;
        let group_w = 4 * card_step - 1;
        let mut x = area.x;
        for trick in &table.my_won {
            if x + group_w > area.x + area.width {
                break;
            }
            for (seat, card) in trick.iter().enumerate() {
                let Some(card) = card else { continue };
                let cx = x + seat as u16 * card_step;
                render_center_card(f, cx, area.y + 2, *card, seat == table.me);
            }
            x += group_w + 2;
        }
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let message = if self.game_over {
            "Game over — press q to quit".to_string()
        } else if self.awaiting {
            match self.action {
                Action::Call => format!(
                    "Your call: {}   ↑/↓ change · Enter confirm · [q] quit",
                    self.call_value
                ),
                Action::Break => "Your turn — ←/→ pick a card · Enter play · [q] quit".to_string(),
                Action::Update => format!("{}   [q] quit", self.status),
            }
        } else {
            format!("{}   [q] quit", self.status)
        };
        f.render_widget(Paragraph::new(message), area);
    }
}

/// A card as a two-line white block: suit top-left, rank bottom-right.
fn draw_face(f: &mut Frame, rect: Rect, card: Card) {
    let (glyph, color) = suit_glyph(card.get_suit());
    let top = format!("{:<width$}", glyph, width = rect.width as usize);
    let bottom = format!(
        "{:>width$}",
        rank_str(card.get_rank()),
        width = rect.width as usize
    );
    f.render_widget(
        Paragraph::new(vec![Line::raw(top), Line::raw(bottom)])
            .style(Style::default().bg(Color::White).fg(color)),
        rect,
    );
}

/// A face-down card.
fn draw_back(f: &mut Frame, rect: Rect) {
    let row = "▞".repeat(rect.width as usize);
    f.render_widget(
        Paragraph::new(vec![Line::raw(row.clone()), Line::raw(row)])
            .style(Style::default().bg(Color::Blue).fg(Color::White)),
        rect,
    );
}

fn render_hand_card(f: &mut Frame, x: u16, base_y: u16, card: Card, raised: bool, selected: bool) {
    let y = if raised {
        base_y.saturating_sub(1)
    } else {
        base_y
    };
    draw_face(f, Rect::new(x, y, CARD_W, CARD_H), card);
    if selected {
        f.render_widget(
            Paragraph::new(Line::styled(
                "▲".repeat(CARD_W as usize),
                Style::default().fg(Color::Yellow),
            )),
            Rect::new(x, base_y + CARD_H, CARD_W, 1),
        );
    }
}

fn render_center_card(f: &mut Frame, x: u16, y: u16, card: Card, winner: bool) {
    if winner {
        f.render_widget(
            Paragraph::new(Line::styled("★", Style::default().fg(Color::Yellow)))
                .alignment(Alignment::Center),
            Rect::new(x, y.saturating_sub(1), CARD_W, 1),
        );
    }
    draw_face(f, Rect::new(x, y, CARD_W, CARD_H), card);
}

/// A small fan of face-down cards standing in for an opponent's hand, with the
/// count of cards held.
fn render_back_fan(f: &mut Frame, x: u16, y: u16, count: usize) {
    for i in 0..count.min(3) as u16 {
        draw_back(f, Rect::new(x + i, y, CARD_W, CARD_H));
    }
    f.render_widget(
        Paragraph::new(Line::styled(
            count.to_string(),
            Style::default().fg(Color::DarkGray),
        )),
        Rect::new(x, y + CARD_H, CARD_W + 2, 1),
    );
}

fn suit_glyph(suit: Suit) -> (&'static str, Color) {
    match suit {
        Suit::Spades => ("♠", Color::Black),
        Suit::Clubs => ("♣", Color::Black),
        Suit::Hearts => ("♥", Color::Red),
        Suit::Diamonds => ("♦", Color::Red),
    }
}

fn rank_str(rank: Rank) -> String {
    rank.to_string()
}
