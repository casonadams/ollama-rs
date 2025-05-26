pub struct App {
    pub input: String,
    pub messages: Vec<String>,
    pub loading: bool,
    pub scroll: u16,
}

impl App {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            loading: false,
            scroll: 0,
        }
    }

    pub fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
        self.scroll = u16::MAX;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll != u16::MAX && self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll != u16::MAX {
            self.scroll = self.scroll.saturating_add(1);
        }
    }
}

