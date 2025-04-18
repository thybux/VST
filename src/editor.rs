use crate::mpsc;
use crate::{thread_safe_map::ThreadSafeMap, Harmonia, HarmoniaParams, Message as MainMessage};
use nih_plug::prelude::*;
use nih_plug_iced::pick_list::State as PickListState;
use nih_plug_iced::*;
use std::sync::{mpsc::Sender, Arc};

// crate pour l'ui
// le button génération
use crate::ui::style::generate_button::GenerateButton;

// le button potentiometre
use crate::ui::style::knob::ParamKnob;

// pour le background de la page principal
use crate::ui::style::main_page::MainPage;

// pour la pick_list (liste déroulante)
use crate::ui::style::pick_list::custom_pick_list;

use nih_plug_iced::widgets::ParamMessage;

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<IcedState> {
    IcedState::from_size(600, 400)
}

pub fn create(
    params: Arc<HarmoniaParams>,
    editor_state: Arc<IcedState>,
    debug_info: ThreadSafeMap<String, String>,
    _message_sender: mpsc::Sender<crate::Message>,
    main_thread_sender: Sender<MainMessage>,
) -> Option<Box<dyn Editor>> {
    create_iced_editor::<HarmoniaEditor>(editor_state, (params, debug_info, main_thread_sender))
}

struct HarmoniaEditor {
    params: Arc<HarmoniaParams>,
    context: Arc<dyn GuiContext>,
    // state pour le button de géneration pour le call modèle
    button_state: button::State,
    // state pour le button de download
    download_state: button::State,

    debug_info: ThreadSafeMap<String, String>,

    // info du button potentiometre
    knob_drag_state: bool,
    knob_last_y: f32,

    // state pour le style de la liste dropDown + style selected
    style_state: PickListState<Style>,
    selected_style: Option<Style>,

    note_state: PickListState<Note>,
    selected_note: Option<Note>,
    mode_state: PickListState<Mode>,
    selected_mode: Option<Mode>,

    // États pour le potentiomètre BPM
    bpm_knob_drag_state: bool,
    bpm_knob_last_y: f32,

    // États pour le potentiomètre Time Signature
    time_sig_knob_drag_state: bool,
    time_sig_knob_last_y: f32,

    main_thread_sender: Sender<MainMessage>,

    // section pour state du download
    show_popup: bool,
    download_file_path: Option<String>
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Style::Trap => "Trap",
            Style::Rock => "Rock",
            Style::Hyperpop => "Hyperpop",
        })
    }
}

impl IcedEditor for HarmoniaEditor {
    type Executor = executor::Default;
    type Message = Message;
    type InitializationFlags = (
        Arc<HarmoniaParams>,
        ThreadSafeMap<String, String>,
        Sender<MainMessage>,
    );

    fn new(
        (params, debug_info, main_thread_sender): Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Command<Self::Message>) {
        let editor = HarmoniaEditor {
            main_thread_sender,
            params,
            context,
            debug_info,
            button_state: button::State::new(),
            download_state: button::State::new(),
            knob_drag_state: false,
            knob_last_y: 0.0,
            bpm_knob_drag_state: false,
            bpm_knob_last_y: 0.0,
            time_sig_knob_drag_state: false,
            time_sig_knob_last_y: 0.0,
            style_state: PickListState::default(),
            selected_style: None,

            note_state: PickListState::default(),
            selected_note: None,
            mode_state: PickListState::default(),
            selected_mode: None,

            // initialisation du state de download
            show_popup: false,
            download_file_path: None,
        };
        (editor, Command::none())
    }

    fn context(&self) -> &dyn GuiContext {
        self.context.as_ref()
    }

    fn update(
        &mut self,
        _window: &mut WindowQueue,
        message: Self::Message,
    ) -> Command<Self::Message> {
        match message {
            Message::Generate => {
                println!("Generate button pressed !");
                let result = self.main_thread_sender.send(MainMessage::Generate);
                println!("Message envoyé avec succès: {}", result.is_ok());
            }

            // message pour le style de musique
            Message::SelectStyle(style) => {
                println!("Selected {:?}", style);
                let result = self.main_thread_sender.send(MainMessage::SelectStyle(style));
                self.selected_style = Some(style);
            }

            Message::Download => {
                println!("EDITOR: Bouton Download cliqué!");
                let result = self.main_thread_sender.send(MainMessage::Download);
                println!("EDITOR: Résultat de l'envoi du message Download: {}", result.is_ok());
            }

            Message::SelectNote(note) => {
                println!("Selected note {:?}", note);
                self.selected_note = Some(note);
            }

            Message::SelectMode(mode) => {
                println!("Selected mode {:?}", mode);
                self.selected_mode = Some(mode);
            }


            // ajout du message pour le button potentiomètre
            Message::ParamUpdate(msg) => {
                match msg {
                    ParamMessage::SetParameterNormalized(param_ptr, value) => unsafe {
                        self.context.raw_set_parameter_normalized(param_ptr, value);
                    },
                    ParamMessage::BeginSetParameter(param_ptr) => unsafe {
                        self.context.raw_begin_set_parameter(param_ptr);
                    },
                    ParamMessage::EndSetParameter(param_ptr) => unsafe {
                        self.context.raw_end_set_parameter(param_ptr);
                    },
                }
                println!("Param updated: {:?}", msg);
            }
            Message::DownloadProgress(_) | Message::DownloadError(_) => todo!(),
        }
        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        // definition de la valeur de départ du "gain"
        let gain_param_ptr = self.params.gain.as_ptr();
        let bpm_param_ptr = self.params.bpm.as_ptr();
        let time_sig_param_ptr = self.params.time_signature.as_ptr();

        // definition du vecteur de style
        let styles = vec![Style::Trap, Style::Rock, Style::Hyperpop];

        //
        //pour les infos de debug
        //

        let visual_debug_info = if self.params.debug.value() {
            Text::new(self.debug_info.to_string())
        } else {
            Text::new("")
        };

        //
        //pour les picklists
        //

        let styles_owned = styles.clone();

        let _style_pick_list = custom_pick_list(
            &mut self.style_state,
            &styles_owned,
            self.selected_style.clone(),
            |selected| selected,
        )
        .map(|selected| Message::SelectStyle(selected));
        let notes = vec![
            Note::C,
            Note::CSharp,
            Note::D,
            Note::DSharp,
            Note::E,
            Note::F,
            Note::FSharp,
            Note::G,
            Note::GSharp,
            Note::A,
            Note::ASharp,
            Note::B,
        ];

        let modes = vec![Mode::Major, Mode::Minor];

        let notes_owned = notes.clone();
        let modes_owned = modes.clone();

        let _note_pick_list = custom_pick_list(
            &mut self.note_state,
            &notes_owned,
            self.selected_note.clone(),
            |selected| selected,
        )
        .map(|selected| Message::SelectNote(selected));

        let _mode_pick_list = custom_pick_list(
            &mut self.mode_state,
            &modes_owned,
            self.selected_mode.clone(),
            |selected| selected,
        )
        .map(|selected| Message::SelectMode(selected));

        let title = Text::new("Harmonia")
            .font(assets::NOTO_SANS_LIGHT)
            .size(40)
            .width(Length::Fill)
            .horizontal_alignment(alignment::Horizontal::Left)
            .vertical_alignment(alignment::Vertical::Center);

        let version = Text::new(format!("v{}", Harmonia::VERSION))
            .font(assets::NOTO_SANS_LIGHT)
            .size(40)
            .width(Length::Fill)
            .horizontal_alignment(alignment::Horizontal::Right)
            .vertical_alignment(alignment::Vertical::Center);

        let _central_element = Row::new()
            .align_items(Alignment::Center)
            .spacing(20)
            // Potentiomètre existant
            .push(Element::<'_, Message>::from(
                ParamKnob::new(
                    gain_param_ptr,
                    &mut self.knob_drag_state,
                    &mut self.knob_last_y,
                )
                .size(50)
                .label("Gain")
                .map(Message::ParamUpdate),
            ))
            .push(Space::with_width(10.into()))
            // Potentiomètre BPM
            .push(Element::<'_, Message>::from(
                ParamKnob::new(
                    bpm_param_ptr,
                    &mut self.bpm_knob_drag_state,
                    &mut self.bpm_knob_last_y,
                )
                .size(50)
                .label("BPM")
                .map(Message::ParamUpdate),
            ))
            .push(Space::with_width(10.into()))
            // Potentiomètre Time Signature
            .push(Element::<'_, Message>::from(
                ParamKnob::new(
                    time_sig_param_ptr,
                    &mut self.time_sig_knob_drag_state,
                    &mut self.time_sig_knob_last_y,
                )
                .size(60)
                .label("Time Signature")
                .map(Message::ParamUpdate),
            ));

        Container::new(
            Column::new()
                .padding(25)
                .align_items(Alignment::Center)
                .push(Row::new().push(title).push(version))
                .push(Space::with_height(20.into()))
                .push(
                    Row::new()
                        .push(
                            Text::new("Select a style")
                                .height(20.into())
                                .width(Length::Fill)
                                .horizontal_alignment(alignment::Horizontal::Left)
                                .vertical_alignment(alignment::Vertical::Center),
                        )
                        .push(_style_pick_list)
                        .align_items(Alignment::Center),
                )
                .push(Space::with_height(10.into()))
                .push(
                    Row::new()
                        .align_items(Alignment::Center)
                        .push(
                            Text::new("Mode: ")
                                .font(assets::NOTO_SANS_BOLD)
                                .horizontal_alignment(alignment::Horizontal::Center),
                        )
                        .push(Space::with_width(Length::Units(20)))
                        .push(_mode_pick_list)
                        .push(Space::with_width(Length::Units(40)))
                        .push(
                            Text::new("Note: ")
                                .font(assets::NOTO_SANS_BOLD)
                                .horizontal_alignment(alignment::Horizontal::Center),
                        )
                        .push(Space::with_width(Length::Units(20)))
                        .push(_note_pick_list),
                )
                .push(Space::with_height(30.into()))
                .push(_central_element)
                .push(visual_debug_info)
                .push(Space::new(Length::Fill, Length::Fill))
                .push(
                    Row::new()
                        .push(Space::with_width(Length::Fill))
                        .push(
                            Button::new(
                                &mut self.button_state,
                                Text::new("Generate")
                                    .font(assets::NOTO_SANS_BOLD)
                                    .horizontal_alignment(alignment::Horizontal::Center),
                            )
                            .style(GenerateButton)
                            .on_press(Message::Generate)
                            .width(Length::Units(120)),
                        )
                        .push(Space::with_width(Length::Units(20)))
                        .push(
                            Button::new(
                                &mut self.download_state,
                                Text::new("Download")
                                    .font(assets::NOTO_SANS_BOLD)
                                    .horizontal_alignment(alignment::Horizontal::Center),
                            )
                            .style(GenerateButton)
                            .on_press(Message::Download)
                            .width(Length::Units(120)),
                        )
                        .push(Space::with_width(Length::Fill))
                        .align_items(Alignment::Center),
                ),
        )
        .style(MainPage)
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Major,
    Minor,
}
impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Note::C => "C",
            Note::CSharp => "C#",
            Note::D => "D",
            Note::DSharp => "D#",
            Note::E => "E",
            Note::F => "F",
            Note::FSharp => "F#",
            Note::G => "G",
            Note::GSharp => "G#",
            Note::A => "A",
            Note::ASharp => "A#",
            Note::B => "B",
        })
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Mode::Major => "Major",
            Mode::Minor => "Minor",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectMode(Mode),
    SelectNote(Note),
    Generate,
    SelectStyle(Style),
    ParamUpdate(ParamMessage),
    Download,
    DownloadProgress(u8),
    DownloadError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Trap,
    Rock,
    Hyperpop,
}
