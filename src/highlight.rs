use crate::config::load_config;
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use syntect_assets::assets::HighlightingAssets;
use textwrap::wrap as text_wrap;
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
};
use whoami;

pub enum Segment {
    Text(String),
    Fence(String),
    Code { lang: Option<String>, code: String },
}

pub struct Styler {
    ps: SyntaxSet,
    theme: Theme,
    pub ollama_label: String,
    pub you_label: String,
    pub ollama: Style,
    pub you: Style,
    pub normal: Style,
    pub fence: Style,
    pub think: Style,
}

impl Styler {
    pub fn default() -> Self {
        let cfg = load_config().unwrap_or_default();
        let model_label = cfg
            .model
            .split(&[':', '@', '/'][..])
            .next()
            .unwrap_or(&cfg.model)
            .to_string();

        let you_label = whoami::username();

        let ps = SyntaxSet::load_defaults_newlines();
        let assets = HighlightingAssets::from_binary();
        let theme: Theme = assets.get_theme("Visual Studio Dark+").clone();

        Styler {
            ps,
            theme,
            ollama_label: model_label.clone(),
            you_label: you_label.clone(),
            ollama: Style::default().fg(Color::Blue),
            you: Style::default().fg(Color::Gray),
            normal: Style::default(),
            fence: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            think: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        }
    }

    fn parse_fences(&self, input: &str) -> Vec<Segment> {
        let mut segs = Vec::new();
        let mut in_code = false;
        let mut buf = String::new();
        let mut current_lang: Option<String> = None;

        for line in input.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("```") {
                if !buf.is_empty() {
                    if in_code {
                        segs.push(Segment::Code {
                            lang: current_lang.clone(),
                            code: buf.clone(),
                        });
                    } else {
                        segs.push(Segment::Text(buf.clone()));
                    }
                    buf.clear();
                }
                segs.push(Segment::Fence(format!("{}\n", line)));
                if in_code {
                    current_lang = None;
                } else {
                    let lang = rest.trim();
                    current_lang = if lang.is_empty() {
                        None
                    } else {
                        Some(lang.to_string())
                    };
                }
                in_code = !in_code;
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        }
        if !buf.is_empty() {
            if in_code {
                segs.push(Segment::Code {
                    lang: current_lang,
                    code: buf,
                });
            } else {
                segs.push(Segment::Text(buf));
            }
        }
        segs
    }

    fn split_think(&self, text: &str) -> Vec<(String, bool)> {
        let mut parts = Vec::new();
        let mut rest = text;
        let open = "<think>";
        let close = "</think>";
        while let Some(start) = rest.find(open) {
            let (before, after) = rest.split_at(start);
            if !before.is_empty() {
                parts.push((before.to_string(), false));
            }
            if let Some(end) = after.find(close) {
                let inner = &after[open.len()..end];
                parts.push((inner.to_string(), true));
                rest = &after[end + close.len()..];
            } else {
                let inner = &after[open.len()..];
                parts.push((inner.to_string(), true));
                rest = "";
                break;
            }
        }
        if !rest.is_empty() {
            parts.push((rest.to_string(), false));
        }
        parts
    }

    pub fn style_message(&self, msg: &str, wrap_width: usize) -> Vec<Spans<'static>> {
        let origin = if msg.starts_with(&format!("{}:", self.ollama_label)) {
            self.ollama
        } else if msg.starts_with(&format!("{}:", self.you_label)) {
            self.you
        } else {
            self.normal
        };

        let mut out = Vec::new();
        for seg in self.parse_fences(msg) {
            match seg {
                Segment::Text(text) => {
                    for (chunk, is_think) in self.split_think(&text) {
                        let style = if is_think { self.think } else { origin };
                        for line in chunk.lines() {
                            for part in text_wrap(line, wrap_width) {
                                out.push(Spans::from(Span::styled(part.into_owned(), style)));
                            }
                        }
                    }
                }
                Segment::Fence(fline) => {
                    for part in text_wrap(&fline, wrap_width) {
                        out.push(Spans::from(Span::styled(part.into_owned(), self.fence)));
                    }
                }
                Segment::Code { lang, code } => {
                    let syntax = lang
                        .as_ref()
                        .and_then(|l| self.ps.find_syntax_by_token(l))
                        .unwrap_or_else(|| self.ps.find_syntax_plain_text());
                    let mut h = HighlightLines::new(syntax, &self.theme);
                    for raw in LinesWithEndings::from(&code) {
                        if let Ok(regions) = h.highlight_line(raw, &self.ps) {
                            let spans: Vec<Span> = regions
                                .into_iter()
                                .map(|(st, slice)| {
                                    let fg = Color::Rgb(
                                        st.foreground.r,
                                        st.foreground.g,
                                        st.foreground.b,
                                    );
                                    Span::styled(slice.to_string(), Style::default().fg(fg))
                                })
                                .collect();
                            out.push(Spans::from(spans));
                        } else {
                            out.push(Spans::from(Span::raw(raw.to_string())));
                        }
                    }
                }
            }
        }
        out
    }
}
