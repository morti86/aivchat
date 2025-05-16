use iced::futures::channel::mpsc;
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole}, Credentials
};
use iced::task::{Never, Sipper, sipper};
use crate::config::AiApi;
use tracing::{debug, error, info};
use tokio::sync::mpsc::error::TryRecvError;

#[derive(Debug, Clone)]
pub enum ChatCommand {
    SetChat(AiApi),
    SetContext(String),
    Prompt(String),
    Stop,
}

#[derive(Debug, Clone)]
pub enum ChatEvent {
    ChatReady(mpsc::Sender<ChatCommand>),
    ChatMessage(String),
    ChatError(String),
    StreamEnded,
}

pub fn connect() -> impl Sipper<Never, ChatEvent> {
    sipper(async |mut output| {
        let (sender, mut receiver) = mpsc::channel::<ChatCommand>(100);
        output.send(ChatEvent::ChatReady(sender)).await;
        let mut context: Option<String> = None;
        let mut ch: Option<AiApi> = None;
        let mut cc = None;

        loop {
            let msg = receiver.try_next();
            match msg {
                Ok(m) => {
                    match m {
                        Some(ChatCommand::Prompt(pr)) => {
                            info!("Received prompt: {}", pr);
                            let ch = ch.as_ref().unwrap();
                            let c = Credentials::new(ch.key.as_str(), ch.url.as_str());
                            let mut messages = vec![];
                            if let Some(context) = &context {
                                messages.push(ChatCompletionMessage {
                                    role: ChatCompletionMessageRole::System,
                                    content: Some(context.clone()),
                                    name: None,
                                    function_call: None,
                                    tool_call_id: None, 
                                    tool_calls: None 
                                });
                            }

                            messages.push(ChatCompletionMessage { 
                                role: ChatCompletionMessageRole::User,
                                content: Some(pr), 
                                name: None, 
                                function_call: None, 
                                tool_call_id: None, 
                                tool_calls: None 
                            });

                            cc = Some(ChatCompletion::builder(ch.model.as_str(), messages.clone())
                                .credentials(c.clone())
                                .stream(true)
                                .create_stream()
                                .await);
                        }
                        Some(ChatCommand::Stop) => {
                            info!("Told to stop chat");
                            cc = None;
                        }
                        Some(ChatCommand::SetChat(chat)) => {
                            ch = Some(chat);
                        }
                        Some(ChatCommand::SetContext(ctx)) => {
                            context = Some(ctx);
                        }
                        _ => {
                            debug!("Should not happen to be None");
                        }
                    }
                }
                Err(_e) => { // We receive messages here: No commands
                    if let Some(ccs) = cc.as_mut() {
                        match ccs.as_mut() {
                            Ok(ccs) => {
                                let mut d = true;
                                while d {
                                    let r = ccs.try_recv();
                                    match r {
                                        Ok(r) => {
                                            let choice = &r.choices[0];
                                            if let Some(content) = &choice.delta.content {
                                                debug!("Received content: {}", content);
                                                output.send(ChatEvent::ChatMessage(content.clone())).await;
                                            }
                                        }
                                        Err(TryRecvError::Empty) => {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(350)).await;
                                        }
                                        Err(TryRecvError::Disconnected) => {
                                            debug!("** DC **");
                                            d = false;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error requesting: {}", e.to_string());
                                output.send(ChatEvent::ChatError(e.to_string())).await;
                            }
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
                    cc = None;
                }
            }
        }
    })
}
