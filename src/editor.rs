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

use crate::ui::style::waiting_button::WatingButton;

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
    download_available: Arc<std::sync::atomic::AtomicBool>,
) -> Option<Box<dyn Editor>> {
    create_iced_editor::<HarmoniaEditor>(editor_state, (params, debug_info, main_thread_sender, download_available))
}

struct HarmoniaEditor {
    params: Arc<HarmoniaParams>,
    context: Arc<dyn GuiContext>,
    // state pour le button de géneration pour le call modèle
    button_state: button::State,
    // state pour le button de download
    download_state: button::State,

    download_link_available: bool,

    debug_info: ThreadSafeMap<String, String>,

    // info du button potentiometre
    knob_drag_state: bool,
    knob_last_y: f32,

    download_available: Arc<std::sync::atomic::AtomicBool>,

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

impl IcedEditor for HarmoniaEditor {
    type Executor = executor::Default;
    type Message = Message;
    type InitializationFlags = (
        Arc<HarmoniaParams>,
        ThreadSafeMap<String, String>,
        Sender<MainMessage>,
        Arc<std::sync::atomic::AtomicBool>,
    );

    fn new(
        (params, debug_info, main_thread_sender, download_available): Self::InitializationFlags,
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
            download_link_available: false,
            knob_last_y: 0.0,
            bpm_knob_drag_state: false,
            bpm_knob_last_y: 0.0,
            time_sig_knob_drag_state: false,
            time_sig_knob_last_y: 0.0,
            style_state: PickListState::default(),
            selected_style: None,

            download_available,

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

            Message::DownloadLinkAvailableEditor(isAvailable) => {
                println!("EDITOR: Received DownloadLinkAvailable({})", isAvailable);
                self.download_link_available = isAvailable;
                return Command::perform(async {}, |_| Message::RefreshUI);
            }

            Message::RefreshUI => {

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
        let mut styles = vec![];
        for instrument_id in 0..128 {
            // Conversion de l'ID d'instrument en variant Style
            // Cette partie dépend de votre implémentation - vous devrez créer
            // une fonction qui convertit un ID (0-127) en variant Style correspondant
            if let Some(instrument_style) = Style::from_id(instrument_id) {
                styles.push(instrument_style);
            }
        }

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
                            Text::new("Selection de l'instrument")
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
                        .push({
                            let element: Element<'_, Message> = if self.download_available.load(std::sync::atomic::Ordering::SeqCst) {
                                Button::new(
                                    &mut self.download_state,
                                    Text::new("Download")
                                        .font(assets::NOTO_SANS_BOLD)
                                        .horizontal_alignment(alignment::Horizontal::Center),
                                )
                                    .style(GenerateButton)
                                    .on_press(Message::Download)
                                    .width(Length::Units(120))
                                    .into()
                            }  else {
                                      Button::new(
                                          &mut self.download_state,
                                          Text::new("Waiting...")
                                              .font(assets::NOTO_SANS_LIGHT)
                                              .horizontal_alignment(alignment::Horizontal::Center),
                                      )
                                          .style(WatingButton)
                                          .width(Length::Units(120))
                                          .into()
                                  };
                                  element
                              }
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
    RefreshUI,
    SelectStyle(Style),
    DownloadLinkAvailableEditor(bool),
    ParamUpdate(ParamMessage),
    Download,
    DownloadProgress(u8),
    DownloadError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    // Pianos
    AcousticGrand,
    BrightAcoustic,
    ElectricGrand,
    HonkyTonk,
    ElectricPiano1,
    ElectricPiano2,
    Harpsichord,
    Clav,
    // Chromatic Percussion
    Celesta,
    Glockenspiel,
    MusicBox,
    Vibraphone,
    Marimba,
    Xylophone,
    TubularBells,
    Dulcimer,
    // Organs
    DrawbarOrgan,
    PercussiveOrgan,
    RockOrgan,
    ChurchOrgan,
    ReedOrgan,
    Accordion,
    Harmonica,
    TangoAccordion,
    // Guitars
    AcousticGuitarNylon,
    AcousticGuitarSteel,
    ElectricGuitarJazz,
    ElectricGuitarClean,
    ElectricGuitarMuted,
    OverdrivenGuitar,
    DistortionGuitar,
    GuitarHarmonics,
    // Basses
    AcousticBass,
    ElectricBassFinger,
    ElectricBassPick,
    FretlessBass,
    SlapBass1,
    SlapBass2,
    SynthBass1,
    SynthBass2,
    // Strings
    Violin,
    Viola,
    Cello,
    Contrabass,
    TremoloStrings,
    PizzicatoStrings,
    OrchestralHarp,
    Timpani,
    // Ensemble
    StringEnsemble1,
    StringEnsemble2,
    SynthStrings1,
    SynthStrings2,
    ChoirAahs,
    VoiceOohs,
    SynthVoice,
    OrchestraHit,
    // Brass
    Trumpet,
    Trombone,
    Tuba,
    MutedTrumpet,
    FrenchHorn,
    BrassSection,
    SynthBrass1,
    SynthBrass2,
    // Reed
    SopranoSax,
    AltoSax,
    TenorSax,
    BaritoneSax,
    Oboe,
    EnglishHorn,
    Bassoon,
    Clarinet,
    // Pipe
    Piccolo,
    Flute,
    Recorder,
    PanFlute,
    BlownBottle,
    Shakuhachi,
    Whistle,
    Ocarina,
    // Synth Lead
    Lead1Square,
    Lead2Sawtooth,
    Lead3Calliope,
    Lead4Chiff,
    Lead5Charang,
    Lead6Voice,
    Lead7Fifths,
    Lead8BassLead,
    // Synth Pad
    Pad1NewAge,
    Pad2Warm,
    Pad3Polysynth,
    Pad4Choir,
    Pad5Bowed,
    Pad6Metallic,
    Pad7Halo,
    Pad8Sweep,
    // Synth Effects
    FX1Rain,
    FX2Soundtrack,
    FX3Crystal,
    FX4Atmosphere,
    FX5Brightness,
    FX6Goblins,
    FX7Echoes,
    FX8SciFi,
    // Ethnic
    Sitar,
    Banjo,
    Shamisen,
    Koto,
    Kalimba,
    Bagpipe,
    Fiddle,
    Shanai,
    // Percussive
    TinkleBell,
    Agogo,
    SteelDrums,
    Woodblock,
    TaikoDrum,
    MelodicTom,
    SynthDrum,
    ReverseCymbal,
    // Sound Effects
    GuitarFretNoise,
    BreathNoise,
    Seashore,
    BirdTweet,
    TelephoneRing,
    Helicopter,
    Applause,
    Gunshot,
}


impl Style {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Style::AcousticGrand),
            1 => Some(Style::BrightAcoustic),
            2 => Some(Style::ElectricGrand),
            3 => Some(Style::HonkyTonk),
            4 => Some(Style::ElectricPiano1),
            5 => Some(Style::ElectricPiano2),
            6 => Some(Style::Harpsichord),
            7 => Some(Style::Clav),
            8 => Some(Style::Celesta),
            9 => Some(Style::Glockenspiel),
            10 => Some(Style::MusicBox),
            11 => Some(Style::Vibraphone),
            12 => Some(Style::Marimba),
            13 => Some(Style::Xylophone),
            14 => Some(Style::TubularBells),
            15 => Some(Style::Dulcimer),
            16 => Some(Style::DrawbarOrgan),
            17 => Some(Style::PercussiveOrgan),
            18 => Some(Style::RockOrgan),
            19 => Some(Style::ChurchOrgan),
            20 => Some(Style::ReedOrgan),
            21 => Some(Style::Accordion),
            22 => Some(Style::Harmonica),
            23 => Some(Style::TangoAccordion),
            24 => Some(Style::AcousticGuitarNylon),
            25 => Some(Style::AcousticGuitarSteel),
            26 => Some(Style::ElectricGuitarJazz),
            27 => Some(Style::ElectricGuitarClean),
            28 => Some(Style::ElectricGuitarMuted),
            29 => Some(Style::OverdrivenGuitar),
            30 => Some(Style::DistortionGuitar),
            31 => Some(Style::GuitarHarmonics),
            32 => Some(Style::AcousticBass),
            33 => Some(Style::ElectricBassFinger),
            34 => Some(Style::ElectricBassPick),
            35 => Some(Style::FretlessBass),
            36 => Some(Style::SlapBass1),
            37 => Some(Style::SlapBass2),
            38 => Some(Style::SynthBass1),
            39 => Some(Style::SynthBass2),
            40 => Some(Style::Violin),
            41 => Some(Style::Viola),
            42 => Some(Style::Cello),
            43 => Some(Style::Contrabass),
            44 => Some(Style::TremoloStrings),
            45 => Some(Style::PizzicatoStrings),
            46 => Some(Style::OrchestralHarp),
            47 => Some(Style::Timpani),
            48 => Some(Style::StringEnsemble1),
            49 => Some(Style::StringEnsemble2),
            50 => Some(Style::SynthStrings1),
            51 => Some(Style::SynthStrings2),
            52 => Some(Style::ChoirAahs),
            53 => Some(Style::VoiceOohs),
            54 => Some(Style::SynthVoice),
            55 => Some(Style::OrchestraHit),
            56 => Some(Style::Trumpet),
            57 => Some(Style::Trombone),
            58 => Some(Style::Tuba),
            59 => Some(Style::MutedTrumpet),
            60 => Some(Style::FrenchHorn),
            61 => Some(Style::BrassSection),
            62 => Some(Style::SynthBrass1),
            63 => Some(Style::SynthBrass2),
            64 => Some(Style::SopranoSax),
            65 => Some(Style::AltoSax),
            66 => Some(Style::TenorSax),
            67 => Some(Style::BaritoneSax),
            68 => Some(Style::Oboe),
            69 => Some(Style::EnglishHorn),
            70 => Some(Style::Bassoon),
            71 => Some(Style::Clarinet),
            72 => Some(Style::Piccolo),
            73 => Some(Style::Flute),
            74 => Some(Style::Recorder),
            75 => Some(Style::PanFlute),
            76 => Some(Style::BlownBottle),
            77 => Some(Style::Shakuhachi),
            78 => Some(Style::Whistle),
            79 => Some(Style::Ocarina),
            80 => Some(Style::Lead1Square),
            81 => Some(Style::Lead2Sawtooth),
            82 => Some(Style::Lead3Calliope),
            83 => Some(Style::Lead4Chiff),
            84 => Some(Style::Lead5Charang),
            85 => Some(Style::Lead6Voice),
            86 => Some(Style::Lead7Fifths),
            87 => Some(Style::Lead8BassLead),
            88 => Some(Style::Pad1NewAge),
            89 => Some(Style::Pad2Warm),
            90 => Some(Style::Pad3Polysynth),
            91 => Some(Style::Pad4Choir),
            92 => Some(Style::Pad5Bowed),
            93 => Some(Style::Pad6Metallic),
            94 => Some(Style::Pad7Halo),
            95 => Some(Style::Pad8Sweep),
            96 => Some(Style::FX1Rain),
            97 => Some(Style::FX2Soundtrack),
            98 => Some(Style::FX3Crystal),
            99 => Some(Style::FX4Atmosphere),
            100 => Some(Style::FX5Brightness),
            101 => Some(Style::FX6Goblins),
            102 => Some(Style::FX7Echoes),
            103 => Some(Style::FX8SciFi),
            104 => Some(Style::Sitar),
            105 => Some(Style::Banjo),
            106 => Some(Style::Shamisen),
            107 => Some(Style::Koto),
            108 => Some(Style::Kalimba),
            109 => Some(Style::Bagpipe),
            110 => Some(Style::Fiddle),
            111 => Some(Style::Shanai),
            112 => Some(Style::TinkleBell),
            113 => Some(Style::Agogo),
            114 => Some(Style::SteelDrums),
            115 => Some(Style::Woodblock),
            116 => Some(Style::TaikoDrum),
            117 => Some(Style::MelodicTom),
            118 => Some(Style::SynthDrum),
            119 => Some(Style::ReverseCymbal),
            120 => Some(Style::GuitarFretNoise),
            121 => Some(Style::BreathNoise),
            122 => Some(Style::Seashore),
            123 => Some(Style::BirdTweet),
            124 => Some(Style::TelephoneRing),
            125 => Some(Style::Helicopter),
            126 => Some(Style::Applause),
            127 => Some(Style::Gunshot),
            _ => None,
        }
    }
}


impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Style::AcousticGrand => "Acoustic Grand",
            Style::BrightAcoustic => "Bright Acoustic",
            Style::ElectricGrand => "Electric Grand",
            Style::HonkyTonk => "Honky-Tonk",
            Style::ElectricPiano1 => "Electric Piano 1",
            Style::ElectricPiano2 => "Electric Piano 2",
            Style::Harpsichord => "Harpsichord",
            Style::Clav => "Clav",
            Style::Celesta => "Celesta",
            Style::Glockenspiel => "Glockenspiel",
            Style::MusicBox => "Music Box",
            Style::Vibraphone => "Vibraphone",
            Style::Marimba => "Marimba",
            Style::Xylophone => "Xylophone",
            Style::TubularBells => "Tubular Bells",
            Style::Dulcimer => "Dulcimer",
            Style::DrawbarOrgan => "Drawbar Organ",
            Style::PercussiveOrgan => "Percussive Organ",
            Style::RockOrgan => "Rock Organ",
            Style::ChurchOrgan => "Church Organ",
            Style::ReedOrgan => "Reed Organ",
            Style::Accordion => "Accordion",
            Style::Harmonica => "Harmonica",
            Style::TangoAccordion => "Tango Accordion",
            Style::AcousticGuitarNylon => "Acoustic Guitar(nylon)",
            Style::AcousticGuitarSteel => "Acoustic Guitar(steel)",
            Style::ElectricGuitarJazz => "Electric Guitar(jazz)",
            Style::ElectricGuitarClean => "Electric Guitar(clean)",
            Style::ElectricGuitarMuted => "Electric Guitar(muted)",
            Style::OverdrivenGuitar => "Overdriven Guitar",
            Style::DistortionGuitar => "Distortion Guitar",
            Style::GuitarHarmonics => "Guitar Harmonics",
            Style::AcousticBass => "Acoustic Bass",
            Style::ElectricBassFinger => "Electric Bass(finger)",
            Style::ElectricBassPick => "Electric Bass(pick)",
            Style::FretlessBass => "Fretless Bass",
            Style::SlapBass1 => "Slap Bass 1",
            Style::SlapBass2 => "Slap Bass 2",
            Style::SynthBass1 => "Synth Bass 1",
            Style::SynthBass2 => "Synth Bass 2",
            Style::Violin => "Violin",
            Style::Viola => "Viola",
            Style::Cello => "Cello",
            Style::Contrabass => "Contrabass",
            Style::TremoloStrings => "Tremolo Strings",
            Style::PizzicatoStrings => "Pizzicato Strings",
            Style::OrchestralHarp => "Orchestral Harp",
            Style::Timpani => "Timpani",
            Style::StringEnsemble1 => "String Ensemble 1",
            Style::StringEnsemble2 => "String Ensemble 2",
            Style::SynthStrings1 => "SynthStrings 1",
            Style::SynthStrings2 => "SynthStrings 2",
            Style::ChoirAahs => "Choir Aahs",
            Style::VoiceOohs => "Voice Oohs",
            Style::SynthVoice => "Synth Voice",
            Style::OrchestraHit => "Orchestra Hit",
            Style::Trumpet => "Trumpet",
            Style::Trombone => "Trombone",
            Style::Tuba => "Tuba",
            Style::MutedTrumpet => "Muted Trumpet",
            Style::FrenchHorn => "French Horn",
            Style::BrassSection => "Brass Section",
            Style::SynthBrass1 => "SynthBrass 1",
            Style::SynthBrass2 => "SynthBrass 2",
            Style::SopranoSax => "Soprano Sax",
            Style::AltoSax => "Alto Sax",
            Style::TenorSax => "Tenor Sax",
            Style::BaritoneSax => "Baritone Sax",
            Style::Oboe => "Oboe",
            Style::EnglishHorn => "English Horn",
            Style::Bassoon => "Bassoon",
            Style::Clarinet => "Clarinet",
            Style::Piccolo => "Piccolo",
            Style::Flute => "Flute",
            Style::Recorder => "Recorder",
            Style::PanFlute => "Pan Flute",
            Style::BlownBottle => "Blown Bottle",
            Style::Shakuhachi => "Shakuhachi",
            Style::Whistle => "Whistle",
            Style::Ocarina => "Ocarina",
            Style::Lead1Square => "Lead 1 (square)",
            Style::Lead2Sawtooth => "Lead 2 (sawtooth)",
            Style::Lead3Calliope => "Lead 3 (calliope)",
            Style::Lead4Chiff => "Lead 4 (chiff)",
            Style::Lead5Charang => "Lead 5 (charang)",
            Style::Lead6Voice => "Lead 6 (voice)",
            Style::Lead7Fifths => "Lead 7 (fifths)",
            Style::Lead8BassLead => "Lead 8 (bass+lead)",
            Style::Pad1NewAge => "Pad 1 (new age)",
            Style::Pad2Warm => "Pad 2 (warm)",
            Style::Pad3Polysynth => "Pad 3 (polysynth)",
            Style::Pad4Choir => "Pad 4 (choir)",
            Style::Pad5Bowed => "Pad 5 (bowed)",
            Style::Pad6Metallic => "Pad 6 (metallic)",
            Style::Pad7Halo => "Pad 7 (halo)",
            Style::Pad8Sweep => "Pad 8 (sweep)",
            Style::FX1Rain => "FX 1 (rain)",
            Style::FX2Soundtrack => "FX 2 (soundtrack)",
            Style::FX3Crystal => "FX 3 (crystal)",
            Style::FX4Atmosphere => "FX 4 (atmosphere)",
            Style::FX5Brightness => "FX 5 (brightness)",
            Style::FX6Goblins => "FX 6 (goblins)",
            Style::FX7Echoes => "FX 7 (echoes)",
            Style::FX8SciFi => "FX 8 (sci-fi)",
            Style::Sitar => "Sitar",
            Style::Banjo => "Banjo",
            Style::Shamisen => "Shamisen",
            Style::Koto => "Koto",
            Style::Kalimba => "Kalimba",
            Style::Bagpipe => "Bagpipe",
            Style::Fiddle => "Fiddle",
            Style::Shanai => "Shanai",
            Style::TinkleBell => "Tinkle Bell",
            Style::Agogo => "Agogo",
            Style::SteelDrums => "Steel Drums",
            Style::Woodblock => "Woodblock",
            Style::TaikoDrum => "Taiko Drum",
            Style::MelodicTom => "Melodic Tom",
            Style::SynthDrum => "Synth Drum",
            Style::ReverseCymbal => "Reverse Cymbal",
            Style::GuitarFretNoise => "Guitar Fret Noise",
            Style::BreathNoise => "Breath Noise",
            Style::Seashore => "Seashore",
            Style::BirdTweet => "Bird Tweet",
            Style::TelephoneRing => "Telephone Ring",
            Style::Helicopter => "Helicopter",
            Style::Applause => "Applause",
            Style::Gunshot => "Gunshot",
        })
    }
}
