use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AiApi {
    pub name: String,
    pub key: String,
    pub url: String,
    pub model: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub ai_chats: HashMap<String, AiApi>,
    pub sel_chat: Option<String>,

    pub height: f32,
    pub width: f32,

    pub font_size: u32,
    pub rec_device: Option<String>,
    pub theme: String,
    pub tr_model: String,
    pub tr_lang: String,

    pub prompt_context: Option<String>,
    pub voices: BTreeMap<String, String>,
}
