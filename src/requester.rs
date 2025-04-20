use crate::Message as MainMessage;
use crate::Style;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;
use std::{fs, path::PathBuf, sync::mpsc, thread};

#[derive(Clone)]
pub struct Requester {
    api_url: String,
    main_thread_sender: mpsc::Sender<MainMessage>,
    client: Client,
}

#[derive(Serialize, Deserialize, Debug)]
struct GenerateRequest {
    bpm: f64,
    duration: u32, // not currently used
    scale: String,
    style: String,
    #[serde(rename = "timeSignatureNum")]
    time_signature_num: i32,
    #[serde(rename = "timeSignatureDen")]
    time_signature_den: i32,
}

#[derive(Serialize, Deserialize)]
pub struct GenerationResponse {
    #[serde(rename = "downloadLink")]
    pub download_link: String,
    pub preview: Vec<EventGroup>,
}

// TODO: maybe move this elsewhere when the events will be handled
#[derive(Serialize, Deserialize)]
pub struct EventGroup {
    pub events: Vec<MusicEvent>,
    pub time: f64,
}

#[derive(Serialize, Deserialize)]
pub struct MusicEvent {
    pub channel: u8,
    pub duration: f64,
    pub note: u8,
    pub time: f64,
    pub track: u8,
    pub velocity: u8,
}

impl Requester {
    pub fn new(api_url: String, main_thread_sender: mpsc::Sender<MainMessage>) -> Self {
        Requester {
            api_url,
            main_thread_sender,
            client: Client::new(),
        }
    }

    pub fn generate(
        &self,
        bpm: f64,
        style: Option<Style>,
        scale: String,
        time_signature_num: i32,
        time_signature_den: i32,
    ) -> Result<String, String> {
        let style = style.ok_or("No style selected")?;

        let request = GenerateRequest {
            bpm,
            duration: 128,
            scale: scale,
            style: style.to_string(),
            time_signature_num,
            time_signature_den,
        };
        
        let body = serde_json::to_string(&request).map_err(|e| e.to_string())?;

        let response = self
            .client
            .post(self.api_url.to_owned() + "/predictions")
            .body(body)
            .send()
            .map_err(|e| e.to_string())?;

        println!("generation: {:?}", response);

        let parsed: GenerationResponse =
            serde_json::from_reader(response).map_err(|e| e.to_string())?;

        Ok(parsed.download_link)
    }

    pub fn download_midi(&self, link: String, download_folder: &PathBuf) {
        println!(
            "Downloading {} to {}",
            link,
            download_folder.to_str().unwrap()
        );
        let file_path = download_folder.join("example.mid");
        let sender = self.main_thread_sender.clone();

        let requester_clone = self.clone();
        thread::spawn(move || {
            if let Err(err) = Self::download_file(requester_clone, &link, &file_path, &sender) {
                sender
                    .send(MainMessage::DownloadError(err.to_string()))
                    .unwrap_or_else(|e| eprintln!("Failed to send error message: {}", e));
            }
        });
    }

    fn download_file(
        self,
        link: &str,
        file_path: &PathBuf,
        sender: &mpsc::Sender<MainMessage>,
    ) -> Result<(), Box<dyn Error>> {
        let _file =
            fs::File::create(file_path).map_err(|e| format!("Failed to create file: {}", e))?;

        let response = self
            .client
            .get(link)
            .send()
            .map_err(|e| format!("Failed to download file: {}", e))?;

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read response bytes: {}", e))?;

        fs::write(file_path, bytes).map_err(|e| format!("Failed to write file: {}", e))?;

        sender
            .send(MainMessage::DownloadProgress(255))
            .map_err(|e| format!("Failed to send progress message: {}", e))?;

        Ok(())
    }
}
