
// TODO:
// - faire l'ouverture du dossier a l'emplacement du telechargement
// - fair la popup au moment du telechargement


mod ui;
use nih_plug::prelude::*;
use nih_plug_iced::IcedState;
use requester::Requester;
use std::f32::consts::PI;
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use thread_safe_map::ThreadSafeMap;
use open;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started
use crate::editor::Message;
use crate::editor::Style;
mod editor;
mod requester;
mod thread_safe_map;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSignature {
    FourFour,  // 4/4
    ThreeFour, // 3/4
    SixEight,  // 6/8
}
impl Enum for TimeSignature {
    fn variants() -> &'static [&'static str] {
        &["4/4", "3/4", "6/8"]
    }

    fn to_index(self) -> usize {
        match self {
            TimeSignature::FourFour => 0,
            TimeSignature::ThreeFour => 1,
            TimeSignature::SixEight => 2,
        }
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => TimeSignature::FourFour,
            1 => TimeSignature::ThreeFour,
            2 => TimeSignature::SixEight,
            _ => TimeSignature::FourFour, // Valeur par défaut
        }
    }
    fn ids() -> Option<&'static [&'static str]> {
        None
    }
}

pub struct Harmonia {
    params: Arc<HarmoniaParams>,
    /// The current data for the peak meter. This is stored as an [`Arc`] so we can share it between
    /// the GUI and the audio processing parts. If you have more state to share, then it's a good
    /// idea to put all of that in a struct behind a single `Arc`.
    ///
    /// This is stored as voltage gain.
    current_tempo: f64,
    scale: String,
    time_sig_numerator: i32,
    time_sig_denominator: i32,

    phase: f32,
    sample_rate: f32,
    samples_per_beat: usize,

    selected_style: Option<Style>,

    download_link: Option<String>,
    downloads_folder: Option<PathBuf>,
    debug_info: ThreadSafeMap<String, String>,

    download_available: Arc<std::sync::atomic::AtomicBool>,


    requester: Requester,
    message_receiver: mpsc::Receiver<Message>,
    message_sender: mpsc::Sender<Message>,
}

#[derive(Params)]
struct HarmoniaParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "debug"]
    pub debug: BoolParam,

    #[persist = "editor-state"]
    editor_state: Arc<IcedState>,

    #[id = "bpm"]
    bpm: FloatParam,

    scale: String,

    time_signature: EnumParam<TimeSignature>,
}

impl Default for Harmonia {
    fn default() -> Self {
        let (message_sender, message_receiver) = mpsc::channel();
        Self {
            params: Arc::new(HarmoniaParams::default()),

            current_tempo: 120.0,
            time_sig_numerator: 4,
            scale: String::from("C#"),
            time_sig_denominator: 0,
            phase: 0.0,
            sample_rate: 44100.0,
            samples_per_beat: 0,

            selected_style: None,

            download_available: Arc::new(std::sync::atomic::AtomicBool::new(false)),

            download_link: None,
            downloads_folder: None,
            debug_info: ThreadSafeMap::new(),

            requester: Requester::new(
                String::from("https://harmonia-api.home.spyr.dev"),
                message_sender.clone(),
            ),
            message_sender,
            message_receiver,
        }
    }
}

impl Default for HarmoniaParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            bpm: FloatParam::new(
                "BPM",
                120.0,
                FloatRange::Linear {
                    min: 60.0,
                    max: 240.0,
                },
            )
            .with_unit(" BPM"),
            scale: String::from("C#"),
            time_signature: EnumParam::new(
                "Time Signature",
                TimeSignature::FourFour, // Utilisez une valeur de type TimeSignature comme valeur par défaut
            ),

            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            debug: BoolParam::new("Debug", false),
        }
    }
}

impl Plugin for Harmonia {
    const NAME: &'static str = "Harmonia";
    const VENDOR: &'static str = "Romain Spychala";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "spyr.dev@proton.me";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.

        self.sample_rate = buffer_config.sample_rate;

        self.samples_per_beat = (self.sample_rate * 60.0 / self.current_tempo as f32) as usize;

        // Creating the folder containing the generated files
        let folder = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Harmonia")
            .join("Downloads");

        if let Ok(_) = fs::create_dir_all(&folder) {
            _ = self
                .debug_info
                .insert(String::from("Folder"), folder.to_string_lossy().to_string());

            self.downloads_folder = Some(folder);
        }

        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    // Call the GUI
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.params.editor_state.clone(),
            self.debug_info.clone(),
            self.message_sender.clone(),
            self.message_sender.clone(),
            self.download_available.clone(),
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let transport = context.transport();

        if let Some(tempo) = transport.tempo {
            if (tempo - self.current_tempo).abs() > f64::EPSILON {
                self.current_tempo = tempo;
                self.samples_per_beat =
                    (self.sample_rate * 60.0 / self.current_tempo as f32) as usize;
            }
        }

        if let Some(time_sig_num) = transport.time_sig_numerator {
            if self.time_sig_numerator != time_sig_num {
                self.time_sig_numerator = time_sig_num;
            }
        }

        if let Some(time_sig_den) = transport.time_sig_denominator {
            if self.time_sig_denominator != time_sig_den {
                self.time_sig_denominator = time_sig_den;
            }
        }

        if self.params.debug.value() {
            let _ = self.debug_info.insert(
                String::from("Transport"),
                format!(
                    "Playing: {}, Tempo: {}, Samples per Beat: {}",
                    transport.playing, self.current_tempo, self.samples_per_beat
                ),
            );
        }

        let time_sig_value = self.params.time_signature.value();


        // Receive messages
        match self.message_receiver.try_recv() {
            Ok(message) => {
                println!("Just received a message, {:?} lib.rs", message);

                match message {

                    Message::Generate => {
                        println!("Generating message");
                        println!("current tempo: {:?}", self.current_tempo);
                        println!("selected_style: {:?}", self.selected_style);
                        println!("sample rate: {:?}", self.time_sig_numerator);
                        println!("sample rate: {:?}", self.time_sig_denominator);

                        match self.requester.generate(
                            self.current_tempo,
                            self.selected_style,
                            self.scale.clone(),
                            self.time_sig_numerator,
                            self.time_sig_denominator,
                        ) {
                            Ok(download_link) => {
                                self.download_link = Some(download_link);
                                self.download_available.store(true, std::sync::atomic::Ordering::SeqCst);
                                let _ = self.message_sender.send(Message::DownloadLinkAvailableEditor(true));
                            },
                            Err(message) => {
                                self.download_link = None;
                                println!("error generating: {}", message)
                            }
                        }
                        println!("download link: {:?}", self.download_link);
                    }

                    Message::Download => match &self.download_link {
                        Some(link) => {
                            self.requester.download_midi(link.clone(), self.downloads_folder.as_ref().unwrap());
                            open::that(self.downloads_folder.as_ref().unwrap()).unwrap();
                        }
                        None => {
                            println!("No download link available");
                        }
                    },
                    Message::DownloadProgress(progress) => {
                        println!("Download progress: {}", progress)
                    }
                    Message::DownloadError(error) => {
                        println!("Error downloading: {}", error);
                    }
                    Message::SelectStyle(style) => {
                        self.selected_style = Some(style);
                    }
                    Message::SelectMode(mode) => {
                        println!("mode selctionner : {}", mode)
                    }
                    Message::SelectNote(note) => {
                        println!("note selectionner : {}", note);
                        self.scale = note.to_string();
                    }
                    Message::ParamUpdate(params) => {
                        println!("paramètre update : {:?}", params)
                    }
                    _ => {}
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => println!("Got disconnected from the channel"),
            Err(mpsc::TryRecvError::Empty) => (),
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Harmonia {
    const CLAP_ID: &'static str = "dev.spyr.harmonia";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("The power of music generation inside your DAW");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::NoteEffect,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Harmonia {
    const VST3_CLASS_ID: [u8; 16] = *b"HarmoniaEpitech!";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Generator,
        Vst3SubCategory::Tools,
    ];
}

nih_export_clap!(Harmonia);
nih_export_vst3!(Harmonia);
