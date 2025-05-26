use crate::{config::load_config, tui::UiEvent};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Deserialize)]
struct StreamChunk {
    message: InnerMessage,
    done: Option<bool>,
}

#[derive(Deserialize)]
struct InnerMessage {
    content: String,
}

pub async fn stream_to_ollama(
    prompt: String,
    cancel: CancellationToken,
    tx: UnboundedSender<UiEvent>,
) {
    let cfg = match load_config() {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(UiEvent::Response(format!("Config error: {}", e)));
            return;
        }
    };

    let endpoint = format!("{}/api/chat", cfg.uri);
    let mut messages = Vec::new();
    if let Some(sys) = cfg.system.as_ref() {
        messages.push(Message {
            role: "system".into(),
            content: sys.clone(),
        });
    }
    messages.push(Message {
        role: "user".into(),
        content: prompt,
    });

    let client = Client::new();
    let request = ChatRequest {
        model: &cfg.model,
        messages,
        stream: true,
    };

    let resp = match client.post(&endpoint).json(&request).send().await {
        Ok(r) => r,
        Err(_) => {
            let _ = tx.send(UiEvent::Response("Error starting stream".into()));
            return;
        }
    };

    let mut stream = resp.bytes_stream();
    while let Some(Ok(chunk)) = stream.next().await {
        if cancel.is_cancelled() {
            break;
        }
        let text = String::from_utf8_lossy(&chunk);
        for frame in text.split('\n').filter(|f| !f.trim().is_empty()) {
            if let Ok(parsed) = serde_json::from_str::<StreamChunk>(frame) {
                let _ = tx.send(UiEvent::Response(parsed.message.content));
                if parsed.done.unwrap_or(false) {
                    return;
                }
            }
        }
    }
}
