use elevenlabs_rs::*;
use elevenlabs_rs::endpoints::genai::tts::{TextToSpeechBody, TextToSpeech};
use iced::futures::channel::mpsc;
use iced::task::{Never, Sipper, sipper};
use tracing::{debug, error, info};
use elevenlabs_rs::utils::play;

#[derive(Debug, Clone)]
pub enum VoiceCommand {
    SetKey(String),
    SetVoice(String),
    Read(String),
}

#[derive(Debug, Clone)]
pub enum VoiceEvent {
    Ready(mpsc::Sender<VoiceCommand>),
    Error(String),
    Finished,
}

pub fn connect() -> impl Sipper<Never, VoiceEvent> {
    sipper(async |mut output| {
        let (sender, mut receiver) = mpsc::channel::<VoiceCommand>(100);
        output.send(VoiceEvent::Ready(sender)).await;

        let mut key: Option<String> = None;
        let mut voice: Option<String> = None;
        let mut ready = false;
        let mut buf = vec![];
        
        loop {
            let msg = receiver.next().await;
            match msg {
                Some(msg) => {
                    match msg {                    
                        VoiceCommand::SetKey(k) => {
                            debug!("Set key: {}", k);
                            key = Some(k);
                        }
                        VoiceCommand::SetVoice(v) => {
                            debug!("Set voice: {}", v);
                            voice = Some(v);
                        }
                        VoiceCommand::Read(s) => {
                            if key.is_none() || voice.is_none() {
                                output.send(VoiceEvent::Error( String::from("No key/voice no read" ))).await;
                                continue;
                            }

                            if ready {
                                let client = ElevenLabsClient::new(key.as_ref().unwrap());
                                let text = buf.join(" ");
                                info!("Text to read is: {}, key={:?}", text, key);
                            
                                let body = TextToSpeechBody::new(text.as_str())
                                    .with_model_id(Model::ElevenMultilingualV2);
                                let endpoint = TextToSpeech::new(voice.as_ref().unwrap(), body);
                                let speech = client.hit(endpoint).await;
                                match speech {
                                    Ok(speech) => {
                                        if let Err(e) = play(speech) {
                                            output.send(VoiceEvent::Error( e.to_string() )).await;
                                        }
                                    }
                                    Err(e) => {
                                        output.send(VoiceEvent::Error( e.to_string() )).await;

                                    }
                                }
                                buf.clear();
                                ready = false;
                            }

                            if check_buf(&buf) && (s.eq(".") || s.eq("。")) {
                                buf.push(s);
                                ready = true;
                            } else {
                                buf.push(s);
                            }

                        }
                    }
                }
                None => {
                    error!("Should not be there");
                }
            }
        }
    })
}

fn check_buf(buf: &Vec<String>) -> bool {
    let a = buf.len() > 1;
    let b = a && buf.iter().any(|s| s.len() > 4);
    b
}
