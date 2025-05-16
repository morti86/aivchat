#![allow(dead_code)]

use iced::widget::{button, column, row, text_editor, Button,
text, combo_box, ComboBox, checkbox, container,
text_input, TextInput, scrollable
};
use iced::{Element, Subscription, Theme};
use tokio::sync::OnceCell;
use tracing::{debug, error, info};
use tokio::sync::RwLock;
use iced::widget::markdown;
use pv_recorder::PvRecorderBuilder;
use std::sync::Arc;
use iced::futures::channel::mpsc;
use iced::futures::sink::SinkExt;
use iced::task::{Never, Sipper, sipper};
use whisper_rs;
use std::collections::BTreeMap;

mod config;
mod vumeter;
mod utils;
mod transcribe;
mod chat;
mod voice;

use vumeter::VUMeter;
use config::Config;
use voice::{VoiceCommand, VoiceEvent};

const CHUNK: i32 = 1024;
const CONFIG: &str = "app.toml";
const DEFAULT_VOICE: &str = "pFZP5JQG7iQjIQuC4Bku";
const MAX_AMPLITUDE_F32: f32 = (u16::MAX / 2) as f32;

static DEFAULT_DEVICE: OnceCell<i32> = OnceCell::const_new();

make_enum!(Language, [EN,PL,DE,CN,FR,IT,ES,PT,RU,UA,JP,TR]);

#[derive(Debug, Clone)]
enum Message {
    EditAction(text_editor::Action),
    ToggleRecord,
    LinkClicked(markdown::Url),
    DeviceSelected(String),
    AppendResult(String),
    ToggleSettings,
    SaveSettings,
    PlayToggle(bool),
    Void,
    FontSizeChangedUp,
    FontSizeChangedDown,
    ThemeSelected(Theme),
    SetText(String),
    RecStreamEvent(RecEvent),
    UpdateState(RecState),
    LanguageSelected(Language),
    ShowError(String),
    HideModal,
    CtxInput(String),
    ChatSelected(String),
    ChatEventReceived(chat::ChatEvent),
    AskChat,
    ChatApiKeyChanged(String),
    ChatApiUrlChanged(String),
    //ChatApiNameChanged(String),
    ChatApiModelChanged(String),
    CopyResult,
    VoiceEventRec(VoiceEvent),
    CopyCode,
}

#[derive(Debug, Clone)]
enum RecEvent {
    Sending(Vec<i16>),
    Ready(mpsc::Sender<RecState>),
    SetSampleRate(usize),
    Error(String),
}

#[derive(Debug, Clone, Default)]
enum RecState {
    Recording(i32),
    #[default]
    Stopped,
}

#[derive(Debug)]
struct AiVChat {
    level: f32,
    vm: VUMeter,
    config: Arc<RwLock<Config>>,
    query_text: text_editor::Content,
    result_text: markdown::Content,
    result_raw: Vec<String>,
    themes: combo_box::State<Theme>,
    theme: Option<Theme>,
    play: bool,

    devices: combo_box::State<String>,
    device_sel: Option<String>,
    device_i: i32,

    settings: bool,

    audio_data: Arc<RwLock<Vec<f32>>>,

    font_size: String,
    font_size_u: u32,

    rec_state: RecState,
    rs_sender: Option<mpsc::Sender<RecState>>,

    tr_language: Option<Language>,
    tr_languages: combo_box::State<Language>,

    show_modal: bool,
    modal_text: String,

    prompt_context: String,
    ai_chat: Option<String>,
    ai_chats: combo_box::State<String>,
    ai_api: config::AiApi,
    ai_apis: Vec<config::AiApi>,
    ai_cmd: Option<mpsc::Sender<chat::ChatCommand>>,
    ai_api_k: String,
    ai_editor_dirty: bool,

    v_sender: Option<mpsc::Sender<VoiceCommand>>,
    voices: BTreeMap<String, String>,
}

pub fn run(theme: &str) -> Result<(), iced::Error> {
    for e in iced::Theme::ALL {

        debug!("Run with theme {}", theme);
        if theme.to_string() == e.to_string() {
            return iced::application(AiVChat::default, AiVChat::update, AiVChat::view)
                .theme(AiVChat::theme)
                .subscription(AiVChat::subscription)
                .run();
        }
    }
    iced::application(AiVChat::default, AiVChat::update, AiVChat::view)
        .subscription(AiVChat::subscription)
        .run()
}

impl Default for AiVChat {

    fn default() -> Self {
        Self::new()
    }
}

impl AiVChat {
    fn title(&self) -> String {
        "Canvas test".to_string()
    }

    fn new() -> Self {
        let vm = VUMeter::new(44100, CHUNK);
        let config: Arc<RwLock<Config>> = Arc::new(RwLock::new( toml::from_str( std::fs::read_to_string( CONFIG ).unwrap().as_str() ).unwrap() ));
        let devices = PvRecorderBuilder::new(CHUNK)
            .get_available_devices()
            .unwrap_or_else(|e| {
                error!("Cannot obtain the list of record devices!: {}", e.to_string());
                vec![]
            });

        let c = config.blocking_read().clone();
        let lang = c.tr_lang;
        let ai_chat = c.sel_chat;
        let prompt_context = c.prompt_context;
        let mut ai_api = config::AiApi::default();
        let mut ai_chats = vec![];
        let mut ai_apis = vec![];
        let mut ai_api_k = String::new();

        c.ai_chats.iter()
            .for_each(|(n,v)| {
                ai_chats.push(v.name.clone());
                ai_apis.push(v.clone());
                if let Some(ai_chat) = &ai_chat {
                    if ai_chat.eq_ignore_ascii_case(v.name.as_str()) {
                        debug!("Setting AiApi[{}]: {:?}", n, v);
                        ai_api = v.clone();
                        ai_api_k = n.clone();
                    }
                }
            });
        let s_ai_chats = combo_box::State::new(ai_chats);

        let theme = Theme::ALL.iter().find(|t|
            t.to_string() == c.theme)
            .cloned();

        let d = c.rec_device.clone().unwrap_or_default();
        let mut device_sel = None;
        let mut device_i = -1;
        if let Some(idx) = devices.iter().position(|x| d.eq_ignore_ascii_case(x)) {
            device_i = idx as i32;
            let _ = DEFAULT_DEVICE.set(device_i);
            device_sel = Some(d);
            info!("Set default device to: {}", device_i);
        }

        let font_size_u = c.font_size;
        let font_size = format!("{}", font_size_u);

        let s_devices = combo_box::State::new(devices);
        let s_themes = Theme::ALL.to_vec();
        let s_lang = Language::ALL.to_vec();
        let voices = c.voices;

        Self { 
            level: -50.0,
            vm,
            config,
            query_text: text_editor::Content::new(),
            result_text: markdown::Content::new(),
            result_raw: vec![],
            theme,
            themes: combo_box::State::new(s_themes),
            play: false,

            devices: s_devices,
            device_sel,
            device_i,

            settings: false,
            audio_data: Arc::new(RwLock::new(vec![])),

            font_size,
            font_size_u,

            rec_state: RecState::Stopped,
            rs_sender: None,

            tr_language: Some(lang.into()),
            tr_languages: combo_box::State::new(s_lang),

            show_modal: false,
            modal_text: String::new(),

            prompt_context: prompt_context.unwrap_or_default(),

            ai_chat,
            ai_chats: s_ai_chats,
            ai_api,
            ai_cmd: None,
            ai_apis,
            ai_api_k,
            ai_editor_dirty: false,

            v_sender: None,
            voices,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        //let vm = VUMeter::new(44100,CHUNK);
        let config = self.config.blocking_read();
        let w = config.width;
        let h = config.height;

        // Settings panel
        if self.settings {
            let label_w = 180.0;
            let ids_dev = text("Select record device").width(label_w);
            let idc_dev: ComboBox<'_, String, Message> = combo_box(&self.devices, "select device", self.device_sel.as_ref(), Message::DeviceSelected);

            let ids_font = text("Font size").width(label_w);
            let idc_font: TextInput<Message> = text_input("", &self.font_size)
                .width(50.0);

            let m = if self.font_size_u < 32 { Some(Message::FontSizeChangedUp) } else { None };
            let idc_font_up: Button<Message> = button("Up")
                .on_press_maybe(m)
                .width(60.0);
            let m = if self.font_size_u > 8 { Some(Message::FontSizeChangedDown) } else { None };
            let idc_font_down: Button<Message> = button("Down")
                .on_press_maybe(m)
                .width(60.0);


            let ids_theme = text("Theme").width(label_w);
            let idc_theme: ComboBox<'_, Theme, Message> = combo_box(&self.themes, "select theme", self.theme.as_ref(), Message::ThemeSelected);

            let ids_lang = text("Transcription language").width(label_w);
            let idc_lang:ComboBox<'_, Language, Message> = combo_box(&self.tr_languages, "", self.tr_language.as_ref(), Message::LanguageSelected);

            let ids_ctx = text("Prompt context").width(label_w);
            let idc_ctx: TextInput<Message> = text_input("", &self.prompt_context)
                .on_input(Message::CtxInput);

            let ids_chat = text("AI Chat").width(label_w);
            let idc_chat:ComboBox<'_, String, Message> = combo_box(&self.ai_chats, "", self.ai_chat.as_ref(), Message::ChatSelected);

            let ids_chat_key = text("Api Key").width(label_w);
            let idc_chat_key: TextInput<Message> = text_input("Api key", &self.ai_api.key )
                .on_input(Message::ChatApiKeyChanged);

            let ids_chat_url = text("Api Url").width(label_w);
            let idc_chat_url: TextInput<Message> = text_input("Api Url", &self.ai_api.url )
                .on_input(Message::ChatApiUrlChanged);

            let ids_chat_model = text("Api Model").width(label_w);
            let idc_chat_model: TextInput<Message> = text_input("Api Model", &self.ai_api.model )
                .on_input(Message::ChatApiUrlChanged);

            let idc_close: Button<Message> = button("Cancel").on_press(Message::ToggleSettings);
            let idc_save: Button<Message> = button("Save").on_press(Message::SaveSettings);

            return column![
                row![ids_dev, idc_dev].spacing(15.0).padding(5.0),
                row![ids_font, idc_font, idc_font_up, idc_font_down].spacing(15.0).padding(5.0),
                row![ids_theme, idc_theme].spacing(15.0).padding(5.0),
                row![ids_lang, idc_lang].spacing(15.0).padding(5.0),
                row![ids_ctx, idc_ctx].spacing(15.0).padding(5.0),
                row![ids_chat, idc_chat].spacing(15.0).padding(5.0),
                row![ids_chat_key, idc_chat_key].spacing(15.0).padding(5.0),
                row![ids_chat_url, idc_chat_url].spacing(15.0).padding(5.0),
                row![ids_chat_model, idc_chat_model].spacing(15.0).padding(5.0),
                row![idc_save, idc_close].spacing(15.0).padding(5.0),
            ].padding(25.0).into();
        } else if self.show_modal {
            let alert = container(
                column![ 
                    text(self.modal_text.as_str()),
                    button(text("OK")).on_press(Message::HideModal) 
                ].align_x(iced::Alignment::Center)
                .spacing(10)
                ).width(w).height(h).padding(10).align_x(iced::Alignment::Center).align_y(iced::Alignment::Center);
            return alert.into();
        }

        let idc_text: Element<'_, Message> = text_editor(&self.query_text)
            .placeholder("Paste text here")
            .on_action(Message::EditAction)
            .height(config.height * 0.3)
            .size(config.font_size)
            .into();
        let b_c = match self.rec_state {
            RecState::Recording(_) => "Stop",
            RecState::Stopped => "Record",
        };
        let text_empty = self.query_text.text().is_empty();
        let b_up: Button<Message> = button(b_c)
            .width(70.0)
            .on_press(Message::ToggleRecord);
        let idc_play: checkbox::Checkbox<'_, Message> = checkbox("Play", self.play)
            .text_line_height(2.0)
            .on_toggle(Message::PlayToggle);

        let idc_settings: Button<Message> = button("Settings").on_press(Message::ToggleSettings);
        let ask_m = if self.ai_api.name.is_empty() || text_empty {
            None
        } else {
            Some(Message::AskChat)
        };

        let idc_ask: Button<Message> = button("Ask").on_press_maybe(ask_m);
        let idc_copy: Button<Message> = button("Copy result").on_press(Message::CopyResult);
        let m_cc = if self.result_raw.is_empty() {
            None
        } else {
            Some(Message::CopyCode)
        };
        let idc_cc: Button<Message> = button("Code").on_press_maybe(m_cc);

        let button_row = row![
            b_up.padding(5.0),
            text(" VU Meter "),
            iced::widget::canvas(&self.vm).width(350.0),
            text(" "),
            idc_play,
            text(" "),
            idc_settings.padding(5.0),
            idc_ask.padding(5.0),
            idc_copy.padding(5.0),
            idc_cc.padding(5.0),
        ].padding(5.0).spacing(5.0);

        let idc_result = markdown::view(self.result_text.items(), self.theme())
            .map(Message::LinkClicked);

        let controls = column![
            idc_text,
            button_row,
            scrollable(idc_result)
        ];

        controls.into()
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone().unwrap_or(Theme::Light)
    }

    fn subscription(&self) -> Subscription<Message> {
        let record = Subscription::run(s_recorder)
            .map(Message::RecStreamEvent);
        //record
        let chat_stream = Subscription::run(chat::connect)
            .map(Message::ChatEventReceived);
        let voice_stream = Subscription::run(voice::connect)
            .map(Message::VoiceEventRec);

        let b = Subscription::batch(vec![chat_stream, record, voice_stream]);
        b
    }

    fn display_av(&mut self, msg: &str) {
        self.modal_text = msg.to_string();
        self.show_modal = true;
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::ToggleRecord => {
                if let Some(sx) = self.rs_sender.as_ref() {
                    let mut ss = sx.clone();
                    let i = self.device_i;
                    let rs = self.rec_state.clone();
                    debug!("Toggle record: {:?}", rs);

                    return iced::Task::perform(async move {
                        match rs {
                            RecState::Stopped => {
                                if let Err(e) = ss.send(RecState::Recording(i)).await {
                                    error!("Error sending: {}", e.to_string());
                                }
                                RecState::Recording(i)
                            }
                            RecState::Recording(_) => {
                                RecState::Stopped
                            }
                        }
                    }, |m| {
                        Message::UpdateState(m)
                    });
                }
                iced::Task::none()

            }
            Message::CopyResult => {
                let result = self.result_raw.join("");
                iced::clipboard::write(result)
                //iced::Task::none()
            }
            Message::CopyCode => {
                let result = self.result_raw.join("");
                let result = utils::substring_between(result.as_str(), "```");
                debug!("Copy: {:?}", result);
                if let Some(result) = result {
                    iced::clipboard::write(result.to_string())
                } else {
                    iced::Task::none()
                }
            }
            Message::UpdateState(s) => {
                debug!(">>> State: {:?}", s);
                self.rec_state = s;
                if let RecState::Recording(_) = &self.rec_state {
                    self.audio_data.blocking_write().clear();
                } else {
                    let au = self.audio_data.clone();
                    let lang = self.tr_language.unwrap_or(Language::EN);
                    let c = self.config.clone();
                    return iced::Task::perform(async move {
                        let model = c.read().await.tr_model.clone();
                        let lang = lang.as_str();
                        transcribe::au_to_text(au, lang, model.as_str()).await
                    }, |s| {
                        match s {
                            Ok(s) => Message::SetText(s),
                            Err(e) => Message::ShowError(e.to_string()), // TODO: Add error dialog
                        }
                    });
                }
                iced::Task::none()
            }
            Message::LanguageSelected(lang) => {
                self.tr_language = Some(lang);
                iced::Task::none()
            }
            Message::SetText(s) => {
                self.query_text = text_editor::Content::with_text(s.as_str());
                iced::Task::none()
            }
            Message::EditAction(a) => {
                match a {
                    text_editor::Action::Select(_) | text_editor::Action::Drag(_) => {
                        self.query_text.perform(a);
                        self.result_text = markdown::Content::new();
                    }
                    _ => self.query_text.perform(a),
                }
                iced::Task::none()
            }
            Message::LinkClicked(e) => {
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(e.to_string()).output();
                #[cfg(target_os = "windows")]
                utils::open_link(e.to_string().as_str());
                iced::clipboard::write(e.to_string())
            }
            Message::DeviceSelected(d) => {
                self.audio_data.blocking_write().clear();
                self.device_sel = Some(d.clone());
                if let Some(idx) = self.devices.options().iter().position(|x| x == &d) {
                    debug!("Device index: {}", idx);
                    self.device_i = idx as i32;
                    if let Some(rs) = self.rs_sender.as_mut() {
                        let mut rs = rs.clone();
                        let i = self.device_i;
                        return iced::Task::perform(async move {
                            rs.send(RecState::Recording(i)).await
                        }, |e| {
                            match e {
                                Ok(_) => Message::Void,
                                Err(e) => Message::ShowError(e.to_string()),
                            }
                        });
                    }
                } else {
                    debug!("Device index not found");
                    self.device_i = -1;
                }

                iced::Task::none()
            }
            Message::AppendResult(s) => {
                self.result_text.push_str(s.as_str());
                iced::Task::none()
            }
            Message::PlayToggle(p) => {
                self.play = p;
                iced::Task::none()
            }
            Message::ToggleSettings => {
                self.settings = !self.settings;
                iced::Task::none()
            }
            Message::ChatSelected(s) => {
                self.ai_chat = Some(s.clone());
                let c = self.config.blocking_read();
                let ai_api_k = c.ai_chats.iter()
                    .find(|(_,a)| a.name == s);
                if let Some((k,a)) = ai_api_k {
                    debug!("Api list key: {}", k);
                    self.ai_api_k = k.clone();
                    self.ai_api = a.clone();
                }
                debug!("Selected: {}", self.ai_api.name);
                iced::Task::none()
            }
            Message::SaveSettings => {
                let c = self.config.clone();
                let sel = self.device_sel.clone();
                let fsize = self.font_size_u;
                let chat = self.ai_chat.clone();
                let chat_ctx = self.prompt_context.clone();
                let api = self.ai_api.clone();
                debug!("Api to save: {:?}", api);
                let api_n = self.ai_api_k.clone();
                debug!("Api name: {}, chat: {:?}", api_n, chat);
                let theme = if let Some(t) = self.theme.clone() {
                    t.to_string()
                } else {
                    String::new()
                };

                let lang = self.tr_language.unwrap_or(Language::PL).to_string();
                iced::Task::perform(async move {
                    let mut config = c.write().await;
                    config.rec_device = sel;
                    config.font_size = fsize;
                    config.theme = theme;
                    config.sel_chat = chat;
                    config.prompt_context = Some(chat_ctx);
                    config.ai_chats.entry(api_n)
                        .and_modify(|e| *e = api);
                    config.tr_lang = lang;
                    if let Ok(s_conf) = toml::to_string(&config.clone()) {
                        match tokio::fs::write(CONFIG, s_conf).await {
                            Ok(_) => debug!("Config saved"),
                            Err(e) => error!("Error saving config data: {}", e.to_string()),
                        }
                    }
                }, |_| {
                    Message::ToggleSettings
                })
            }
            Message::FontSizeChangedUp => {
                self.font_size_u += 1;
                self.font_size = format!("{}", self.font_size_u);
                iced::Task::none()
            }
            Message::FontSizeChangedDown => {
                self.font_size_u -= 1;
                self.font_size = format!("{}", self.font_size_u);
                iced::Task::none()
            }
            Message::Void => {
                iced::Task::none()
            }
            Message::ThemeSelected(theme) => {
                self.theme = Some(theme);
                iced::Task::none()
            }
            Message::ChatEventReceived(e) => {
                match e {
                    chat::ChatEvent::ChatReady(r) => {
                        let mut s = r.clone();
                        self.ai_cmd = Some(r);
                        if !self.ai_api.url.is_empty() {
                            let api = self.ai_api.clone();
                            return iced::Task::perform(async move {
                                    s.send(chat::ChatCommand::SetChat(api)).await
                                }, |res| {
                                    if let Err(e) = res {
                                        Message::ShowError(e.to_string())
                                    } else {
                                        Message::Void
                                    }
                            });
                        }
                    }
                    chat::ChatEvent::StreamEnded => {
                    }
                    chat::ChatEvent::ChatError(e) => {
                        self.display_av(e.as_str());
                    }
                    chat::ChatEvent::ChatMessage(m) => {
                        self.result_text.push_str(m.as_str());
                        let d = format!("{}", m.as_str());
                        self.result_raw.push(m);
                        if !self.play {
                            return iced::Task::none();
                        }

                        if let Some(s) = self.v_sender.as_mut() {
                            let mut sx = s.clone();
                            return iced::Task::perform(async move {
                                sx.send(VoiceCommand::Read(d)).await
                            }, |e| {
                                if let Err(e) = e {
                                    Message::ShowError(e.to_string())
                                } else {
                                    Message::Void
                                }
                            });
                        }
                    }
                }
                iced::Task::none()
            }
            Message::RecStreamEvent(v) => {
                match v {
                    RecEvent::Sending(v) => {
                        let level = v.iter()
                            .map(|&x| x.wrapping_abs())
                            .max()
                            .unwrap_or(0);
                        self.level = level as f32 / MAX_AMPLITUDE_F32;
                        self.vm.update(self.level);
                        let rs = self.rec_state.clone();
                        let au = self.audio_data.clone();

                        return iced::Task::perform(async move {

                            let mut inter_samples = vec![Default::default(); v.len()];
                            if let Err(e) = whisper_rs::convert_integer_to_float_audio(&v, &mut inter_samples) {
                                error!("Error converting: {}", e.to_string());
                            }

                            if let RecState::Recording(_) = rs {
                                au.write().await.append(&mut inter_samples);
                            }
                        }, |_| {
                            Message::Void
                        });

                    }
                    RecEvent::Ready(r) => {
                        self.rs_sender = Some(r);
                        debug!("Ready");
                    }
                    RecEvent::Error(e) => {
                        self.display_av(e.as_str());
                    }
                    RecEvent::SetSampleRate(sr) => {
                        debug!("New SR: {}", sr);
                        self.vm.sample_rate(sr);
                    }
                }

                iced::Task::none()
            }
            Message::HideModal => {
                self.show_modal = false;
                iced::widget::focus_next()
            }
            Message::ShowError(e) => {
                self.display_av(e.as_str());
                iced::Task::none()
            }
            Message::CtxInput(s) => {
                self.prompt_context = s;
                iced::Task::none()
            }
            Message::AskChat => {
                let chat = self.ai_api.clone();
                debug!("Asking AI: {}", chat.name);
                self.result_raw.clear();
                self.result_text = markdown::Content::new();
                let prompt = self.query_text.text();
                let context = self.prompt_context.clone();
                if let Some(mut cmd) = self.ai_cmd.clone() {
                    iced::Task::perform(async move {
                        if !context.is_empty() {
                            if let Err(e) = cmd.send(chat::ChatCommand::SetContext(context)).await {
                                return Err(e)
                            }
                        }
                        cmd.send(chat::ChatCommand::Prompt(prompt)).await
                    }, |e| {
                        if let Err(e) = e {
                            Message::ShowError(e.to_string())
                        } else {
                            Message::Void
                        }
                    })
                } else {
                    self.display_av("No channel to chat established");
                    iced::Task::none()
                }
            }
            Message::ChatApiKeyChanged(key) => {
                self.ai_api.key = key.clone();
                iced::Task::none()
            }
            Message::ChatApiUrlChanged(url) => {
                self.ai_api.url = url;
                iced::Task::none()
            }
            Message::ChatApiModelChanged(model) => {
                self.ai_api.model = model;
                iced::Task::none()
            }
            Message::VoiceEventRec(ve) => {
                match ve {
                    VoiceEvent::Ready(r) => {
                        self.v_sender = Some(r);
                        let el_api = self.ai_apis.iter()
                            .find(|e| e.name == String::from("Elevenlabs"));
                        if let Some(el_api) = el_api {
                            
                            let model = self.voices.get(&el_api.model).unwrap().clone();
                            let key = el_api.key.clone();
                            let mut sx = self.v_sender.clone().unwrap();
                            return iced::Task::perform(async move {
                                let a = sx.send(VoiceCommand::SetKey(key)).await;
                                let b = sx.send(VoiceCommand::SetVoice(model)).await;
                                (a,b)
                            }, |(a,b)| {
                                let mut e: Vec<String> = vec![];

                                if let Err(a) = a {
                                    e.push(a.to_string());
                                }
                                if let Err(b) = b {
                                    e.push(b.to_string());
                                }

                                if e.is_empty() {
                                    Message::Void
                                } else {
                                    Message::ShowError(e.join("; "))
                                }
                            });

                        }
                    }
                    VoiceEvent::Error(e) => {
                        self.display_av(e.as_str());
                        self.play = false;
                    }
                    VoiceEvent::Finished => {
                    }
                }
                iced::Task::none()
            }
        }
    }
}

fn main() -> Result<(), iced::Error> {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_max_level(tracing::Level::INFO)
        .init();

    run("Tokyo Night")
}

fn s_recorder() -> impl Sipper<Never, RecEvent> {
    sipper(async |mut output| {
        let (sender, mut receiver) = mpsc::channel::<RecState>(100);
        output.send(RecEvent::Ready(sender)).await;
        let mut prev_idx = *DEFAULT_DEVICE.get().unwrap();
        debug!("Starting device: {}", prev_idx);
        let mut recorder = PvRecorderBuilder::new(CHUNK).device_index(prev_idx).init().unwrap();
        let sr = recorder.sample_rate();
        output.send(RecEvent::SetSampleRate(sr)).await;
        if let Err(e) = recorder.start() {
            error!("Error starting: {}", e.to_string());
            output.send(RecEvent::Error(e.to_string())).await;
        }

        loop {
            let input = receiver.try_next();
            match input {
                Ok(r) => {
                    if let Some(r) = r {
                        match r {
                            RecState::Recording(idx) => {
                                debug!("Change recording device {}", idx);
                                if idx != prev_idx  {
                                    if let Err(e) = recorder.stop() {
                                        error!("Error stopping: {}", e.to_string());
                                        output.send(RecEvent::Error(e.to_string())).await;
                                    }
                                    recorder = PvRecorderBuilder::new(CHUNK).device_index(idx).init().unwrap();
                                    let sr = recorder.sample_rate();
                                    output.send(RecEvent::SetSampleRate(sr)).await;
                                    if let Err(e) = recorder.start() {
                                        error!("Error starting: {}", e.to_string());
                                        output.send(RecEvent::Error(e.to_string())).await;
                                    }
                                }
                                prev_idx = idx;

                            }
                            _ => {
                                debug!("Why");
                            }
                        }
                    } else {
                        error!("Dupa");
                        continue;
                    }

                }
                Err(_e) => {
                    let frame = recorder.read().expect("Failed to read audio frame");
                    //debug!("TryRecv: {}", frame.len());
                    output.send(RecEvent::Sending(frame)).await;
                }

            }
        }
    })
}
