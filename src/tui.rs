use crate::app::App;
use crate::highlight::Styler;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub enum UiEvent {
    Response(String),
    Done,
}

pub async fn run_ui<F, Fut>(
    app: &mut App,
    cancel_token: CancellationToken,
    mut on_submit: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(String, CancellationToken, UnboundedSender<UiEvent>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let styler = Styler::default();
    let ollama_prefix = format!("{}:", &styler.ollama_label);
    let you_prefix = format!("{}:", &styler.you_label);
    let mut idx = 0;
    let mut last_tick = Instant::now();
    let border_style = Style::default().fg(Color::DarkGray);

    loop {
        while let Ok(evt) = rx.try_recv() {
            match evt {
                UiEvent::Response(chunk) => {
                    if let Some(last) = app.messages.last_mut() {
                        if last.starts_with(&ollama_prefix) {
                            last.push_str(&chunk);
                        } else {
                            app.messages.push(format!("{} {}", ollama_prefix, chunk));
                        }
                    } else {
                        app.messages.push(format!("{} {}", ollama_prefix, chunk));
                    }
                    app.scroll = u16::MAX;
                }
                UiEvent::Done => {
                    app.loading = false;
                }
            }
        }

        if app.loading && last_tick.elapsed() >= Duration::from_millis(100) {
            idx = (idx + 1) % spinner.len();
            last_tick = Instant::now();
        }

        terminal.draw(|f| {
            let size = f.size();
            f.render_widget(Clear, size);

            let title = if app.loading {
                format!(" Chat {} ", spinner[idx])
            } else {
                " Chat ".to_string()
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(size);
            let chat_area = chunks[0];
            let input_area = chunks[1];

            let inner = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title.clone())
                .inner(chat_area);
            let wrap_width = inner.width as usize;

            let mut lines = Vec::new();
            for msg in &app.messages {
                lines.extend(styler.style_message(msg, wrap_width));
            }

            let visible = inner.height as u16;
            let total = lines.len() as u16;
            let max_scroll = total.saturating_sub(visible);
            app.scroll = if app.scroll == u16::MAX {
                max_scroll
            } else {
                app.scroll.min(max_scroll)
            };

            f.render_widget(
                Paragraph::new(lines).scroll((app.scroll, 0)).block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_style),
                ),
                chat_area,
            );

            f.render_widget(
                Paragraph::new(app.input.as_ref()).block(
                    Block::default()
                        .title(" Input ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                ),
                input_area,
            );

            let x = input_area.x + 1 + app.input.len() as u16;
            let y = input_area.y + 1;
            f.set_cursor(x, y);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let CEvent::Key(k) = event::read()? {
                match (k.code, k.modifiers) {
                    (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => break,
                    // (KeyCode::Esc, _) => break,
                    (KeyCode::Up, _) => app.scroll_up(),
                    (KeyCode::Down, _) => app.scroll_down(),
                    (KeyCode::PageUp, _) => (0..5).for_each(|_| app.scroll_up()),
                    (KeyCode::PageDown, _) => (0..5).for_each(|_| app.scroll_down()),
                    (KeyCode::Char(c), _) => app.input.push(c),
                    (KeyCode::Backspace, _) => {
                        app.input.pop();
                    }
                    (KeyCode::Enter, _) => {
                        let inp = std::mem::take(&mut app.input);
                        app.add_message(format!("{} {}", you_prefix, inp));
                        app.loading = true;
                        app.scroll = u16::MAX;

                        let tx_stream = tx.clone();
                        let tx_done = tx.clone();
                        let ct2 = cancel_token.clone();
                        let fut = on_submit(inp, ct2, tx_stream);

                        tokio::spawn(async move {
                            fut.await;
                            let _ = tx_done.send(UiEvent::Done);
                        });
                    }
                    _ => {}
                }
            }
        }

        if cancel_token.is_cancelled() {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
