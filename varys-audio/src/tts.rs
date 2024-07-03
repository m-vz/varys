use log::{debug, info, trace};
use std::time::Instant;

#[cfg(not(target_os = "macos"))]
use cpal::SampleRate;
#[cfg(not(target_os = "macos"))]
use std::io::Write;
#[cfg(not(target_os = "macos"))]
use std::process::{Command, Stdio};

#[cfg(target_os = "macos")]
use cocoa_foundation::{
    base::id,
    foundation::{NSDefaultRunLoopMode, NSRunLoop},
};
#[cfg(target_os = "macos")]
use lerp::Lerp;
#[cfg(target_os = "macos")]
use log::debug;
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use std::sync::mpsc::{channel, TryRecvError};
#[cfg(target_os = "macos")]
use tts::{Features, Tts, Voice};

use crate::error::Error;

/// A speaker that can synthesize voices.
pub struct Speaker {
    #[cfg(target_os = "macos")]
    tts: Tts,
    #[cfg(target_os = "macos")]
    available_voices: Vec<Voice>,
    #[cfg(not(target_os = "macos"))]
    speaker: usize,
}

impl Speaker {
    /// Create a new speaker and load all available voices.
    pub fn new() -> Result<Self, Error> {
        #[cfg(target_os = "macos")]
        {
            let tts = Tts::default()?;

            let Features {
                utterance_callbacks,
                voice,
                ..
            } = tts.supported_features();
            for (available, name) in [
                (utterance_callbacks, "utterance callbacks"),
                (voice, "voices"),
            ] {
                if !available {
                    return Err(Error::UnsupportedFeature(name.to_string()));
                }
            }

            let available_voices = tts.voices()?;
            let speaker = Speaker {
                tts,
                available_voices,
            };

            debug!(
                "Available voices: {}",
                speaker
                    .available_voices
                    .iter()
                    .map(|voice| voice.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            Ok(speaker)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Self { speaker: 0 })
        }
    }

    /// Create a new speaker and set the voice that should be spoken with.
    ///
    /// # Arguments
    ///
    /// * `id`: The id or name of the voice to use.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(target_os = "macos")]
    /// # {
    /// # use varys_audio::tts::Speaker;
    /// assert!(Speaker::with_voice("Ava").is_ok());
    /// # }
    /// ```
    ///
    /// ```
    /// # use varys_audio::error::Error;
    /// # use varys_audio::tts::Speaker;
    /// let invalid_speaker = Speaker::with_voice("Invalid Name");
    ///
    /// if let Err(Error::VoiceNotAvailable(text)) = invalid_speaker {
    ///     assert_eq!(text, "Invalid Name");
    /// } else {
    ///     panic!("Must return `Error::VoiceNotAvailable`");
    /// }
    /// ```
    pub fn with_voice(id: &str) -> Result<Self, Error> {
        let mut speaker = Self::new()?;

        speaker.set_voice(id)?;

        Ok(speaker)
    }

    /// Set the voice that should be spoken with.
    ///
    /// Returns an error if a voice with the given id or name is not available on the current platform.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(target_os = "macos")]
    /// # {
    /// # use varys_audio::tts::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    ///
    /// assert!(speaker.set_voice("Ava").is_ok());
    /// # }
    /// ```
    ///
    /// ```
    /// # use varys_audio::error::Error;
    /// # use varys_audio::tts::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// let invalid = speaker.set_voice("Invalid Name");
    ///
    /// if let Err(Error::VoiceNotAvailable(text)) = invalid {
    ///     assert_eq!(text, "Invalid Name");
    /// } else {
    ///     panic!("Must return `Error::VoiceNotAvailable`");
    /// }
    /// ```
    pub fn set_voice(&mut self, id: &str) -> Result<(), Error> {
        #[cfg(target_os = "macos")]
        {
            let voice = self
                .available_voices
                .iter()
                .find(|v| v.id() == id || v.name() == id);

            if let Some(voice) = voice {
                self.tts.set_voice(voice)?;

                info!("Using voice {}", id);

                Ok(())
            } else {
                Err(Error::VoiceNotAvailable(id.to_string()))
            }
        }
        #[cfg(not(target_os = "macos"))]
        if let Some((index, _)) = AVAILABLE_VOICES
            .iter()
            .enumerate()
            .find(|(_, voice)| **voice == id)
        {
            self.speaker = index;

            Ok(())
        } else {
            Err(Error::VoiceNotAvailable(id.to_string()))
        }
    }

    /// Say a phrase in the current voice, rate and volume. Returns the time in milliseconds it took
    /// to say the phrase.
    ///
    /// Interrupts any previous speaking.
    ///
    /// This blocks the current thread until speaking has finished.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_audio::tts::Speaker;
    /// let speaker = Speaker::new().unwrap();
    /// let speaking_duration = speaker.say("").unwrap();
    /// ```
    pub fn say(&self, text: &str) -> Result<i32, Error> {
        info!("Saying \"{text}\"");

        #[cfg(not(target_os = "macos"))]
        self.generate_wav(text, VOICE_OUTPUT_PATH)?;

        let start = Instant::now();

        #[cfg(target_os = "macos")]
        {
            let (sender, receiver) = channel();
            self.tts.on_utterance_end(Some(Box::new(move |_| {
                let _ = sender.send(());
            })))?;

            self.tts.clone().speak(text, true)?;

            unsafe {
                let run_loop: id = NSRunLoop::currentRunLoop();
                let date: id = msg_send![class!(NSDate), distantFuture];
                while receiver.try_recv() == Err(TryRecvError::Empty) {
                    let _: () = msg_send![run_loop, runMode:NSDefaultRunLoopMode beforeDate:date];
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        self.play_wav(VOICE_OUTPUT_PATH)?;

        let duration = start.elapsed().as_millis() as i32;
        trace!("Spoke for {duration}ms");

        Ok(duration)
    }

    #[cfg(not(target_os = "macos"))]
    fn generate_wav<P: AsRef<std::path::Path>>(&self, text: &str, path: P) -> Result<(), Error> {
        debug!("Writing audio to {}", path.as_ref().display());

        let mut piper = Command::new("piper")
            .stdin(Stdio::piped())
            .arg("--model")
            .arg(VOICE_MODEL_PATH)
            .arg("--speaker")
            .arg(self.speaker.to_string())
            .arg("--quiet")
            .arg("--output_file")
            .arg(VOICE_OUTPUT_PATH)
            .spawn()
            .map_err(|err| Error::Tts(err.to_string()))?;
        piper
            .stdin
            .as_mut()
            .ok_or(Error::Tts("No stdin found".to_string()))?
            .write_all(text.as_bytes())
            .map_err(|err| Error::Tts(err.to_string()))?;
        piper.wait().map_err(|err| Error::Tts(err.to_string()))?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    fn play_wav<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Error> {
        debug!("Playing audio from {}", path.as_ref().display());

        Command::new("aplay")
            .arg("--quiet")
            .arg("-r")
            .arg(VOICE_SAMPLE_RATE.0.to_string())
            .arg("-f")
            .arg("S16_LE")
            .arg("-t")
            .arg("wav")
            .arg(path.as_ref())
            .spawn()
            .map_err(|err| Error::Tts(err.to_string()))?
            .wait()
            .map_err(|err| Error::Tts(err.to_string()))?;

        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
const VOICE_MODEL_PATH: &str = "data/voices/en_US-libritts-high.onnx";

#[cfg(not(target_os = "macos"))]
const VOICE_OUTPUT_PATH: &str = "data/voices/output.wav";

#[cfg(not(target_os = "macos"))]
const VOICE_SAMPLE_RATE: SampleRate = SampleRate(22050);

#[cfg(not(target_os = "macos"))]
const AVAILABLE_VOICES: [&str; 904] = [
    "p3922", "p8699", "p4535", "p6701", "p3638", "p922", "p2531", "p1638", "p8848", "p6544",
    "p3615", "p318", "p6104", "p1382", "p5400", "p5712", "p2769", "p2573", "p1463", "p6458",
    "p3274", "p4356", "p8498", "p5570", "p176", "p339", "p28", "p5909", "p3869", "p4899", "p64",
    "p3368", "p3307", "p5618", "p3370", "p7704", "p8506", "p8410", "p6904", "p5655", "p2204",
    "p501", "p7314", "p1027", "p5054", "p534", "p2853", "p5935", "p2404", "p7874", "p816", "p2053",
    "p8066", "p16", "p4586", "p1923", "p2592", "p1265", "p6189", "p100", "p6371", "p4957", "p4116",
    "p3003", "p7739", "p1752", "p5717", "p5012", "p5062", "p7481", "p4595", "p2299", "p7188",
    "p93", "p4145", "p8684", "p7594", "p2598", "p3540", "p7717", "p6426", "p4148", "p335", "p1379",
    "p2512", "p242", "p8855", "p8118", "p369", "p6575", "p6694", "p8080", "p1283", "p7434",
    "p5290", "p1731", "p2401", "p459", "p192", "p7910", "p114", "p5660", "p1313", "p203", "p7460",
    "p207", "p6497", "p6696", "p7766", "p6233", "p3185", "p2010", "p2056", "p3717", "p5802",
    "p5622", "p2156", "p4243", "p1422", "p5039", "p4110", "p1093", "p1776", "p7995", "p6877",
    "p5635", "p54", "p288", "p4592", "p7276", "p688", "p8388", "p8152", "p8194", "p7000", "p8527",
    "p5126", "p3923", "p1054", "p3927", "p5029", "p4098", "p1789", "p56", "p7240", "p5538",
    "p1903", "p6538", "p3380", "p6643", "p7495", "p8718", "p8050", "p126", "p7245", "p2517",
    "p4438", "p4945", "p7145", "p724", "p9022", "p6637", "p6927", "p6937", "p8113", "p5724",
    "p6006", "p3584", "p2971", "p2230", "p7982", "p1649", "p3994", "p7720", "p6981", "p781",
    "p4973", "p6206", "p2481", "p3157", "p1509", "p510", "p7540", "p8887", "p7120", "p2882",
    "p7128", "p8142", "p7229", "p2787", "p8820", "p2368", "p4331", "p4967", "p4427", "p6054",
    "p3728", "p274", "p7134", "p1603", "p1383", "p1165", "p4363", "p512", "p5985", "p7967",
    "p2060", "p7752", "p7484", "p8643", "p3549", "p5731", "p7881", "p667", "p6828", "p5740",
    "p3483", "p718", "p6341", "p1913", "p3228", "p7247", "p7705", "p1018", "p8193", "p6098",
    "p3989", "p7828", "p5876", "p7754", "p4719", "p8011", "p7939", "p5975", "p2004", "p6139",
    "p8183", "p3482", "p3361", "p4289", "p231", "p7789", "p4598", "p5239", "p2638", "p6300",
    "p8474", "p2194", "p7832", "p1079", "p1335", "p188", "p1195", "p5914", "p1401", "p7318",
    "p5448", "p1392", "p3703", "p2113", "p7783", "p8176", "p6519", "p7933", "p7938", "p7802",
    "p6120", "p224", "p209", "p5656", "p3032", "p6965", "p258", "p4837", "p5489", "p272", "p3851",
    "p7140", "p2562", "p1472", "p79", "p2775", "p3046", "p2532", "p8266", "p6099", "p4425",
    "p5293", "p7981", "p2045", "p920", "p511", "p7416", "p835", "p1289", "p8195", "p7833", "p8772",
    "p968", "p1641", "p7117", "p1678", "p5809", "p8028", "p500", "p6505", "p7868", "p14", "p2238",
    "p4744", "p3733", "p7515", "p699", "p5093", "p6388", "p7959", "p98", "p3914", "p5246", "p2570",
    "p8396", "p3513", "p882", "p7994", "p5968", "p8591", "p806", "p5261", "p1271", "p899", "p3945",
    "p8404", "p249", "p3008", "p7139", "p6395", "p6215", "p6080", "p4054", "p7825", "p6683",
    "p8725", "p3230", "p4138", "p6160", "p666", "p6510", "p3551", "p8075", "p225", "p7169",
    "p1851", "p5984", "p2960", "p8329", "p175", "p6378", "p480", "p7538", "p479", "p5519", "p8534",
    "p4856", "p101", "p3521", "p2256", "p3083", "p4278", "p8713", "p1226", "p4222", "p8494",
    "p8776", "p731", "p6574", "p5319", "p8605", "p5583", "p6406", "p4064", "p4806", "p3972",
    "p7383", "p5133", "p597", "p1025", "p7313", "p5304", "p8758", "p1050", "p6499", "p6956",
    "p770", "p4108", "p2774", "p3864", "p4490", "p4848", "p1826", "p6294", "p7949", "p1446",
    "p7867", "p8163", "p953", "p8138", "p353", "p7553", "p8825", "p5189", "p2012", "p948", "p205",
    "p1535", "p8008", "p1112", "p7926", "p4039", "p716", "p3967", "p7932", "p7525", "p7316",
    "p3448", "p2393", "p6788", "p6550", "p7011", "p8791", "p8119", "p1777", "p6014", "p1046",
    "p6269", "p6188", "p5266", "p3490", "p8786", "p8824", "p589", "p576", "p1121", "p1806",
    "p7294", "p3119", "p2688", "p1012", "p4807", "p7498", "p3905", "p7384", "p2992", "p30", "p497",
    "p227", "p4226", "p5007", "p1066", "p8222", "p7688", "p6865", "p6286", "p8225", "p3224",
    "p8635", "p1348", "p3645", "p1961", "p8190", "p6032", "p7286", "p5389", "p3105", "p1028",
    "p6038", "p764", "p7437", "p6555", "p8875", "p2074", "p7809", "p2240", "p2827", "p5386",
    "p6763", "p3009", "p6339", "p1825", "p7569", "p359", "p7956", "p2137", "p8677", "p4434",
    "p329", "p3289", "p4290", "p2999", "p2427", "p637", "p2229", "p1874", "p3446", "p9023",
    "p3114", "p6235", "p4860", "p4519", "p561", "p70", "p4800", "p2294", "p6115", "p2582", "p8464",
    "p5139", "p6918", "p337", "p5810", "p8401", "p303", "p5206", "p2589", "p7061", "p2269",
    "p2758", "p3389", "p4629", "p707", "p5606", "p1513", "p2473", "p664", "p5092", "p5154",
    "p6288", "p6308", "p4731", "p3328", "p7816", "p3221", "p8687", "p7030", "p476", "p4257",
    "p5918", "p6317", "p204", "p8006", "p6895", "p1264", "p2494", "p112", "p1859", "p398", "p1052",
    "p3294", "p1460", "p8573", "p5684", "p8421", "p5883", "p7297", "p246", "p8057", "p3835",
    "p1748", "p3816", "p3357", "p1053", "p409", "p868", "p3118", "p7520", "p6686", "p1241",
    "p5190", "p166", "p1482", "p5604", "p1212", "p2741", "p1259", "p984", "p6492", "p6167", "p296",
    "p6567", "p6924", "p2272", "p7085", "p345", "p2388", "p1705", "p1343", "p7241", "p451",
    "p5401", "p6446", "p612", "p594", "p7555", "p7069", "p2577", "p5333", "p8742", "p6727",
    "p1571", "p4734", "p7258", "p3977", "p373", "p5723", "p1365", "p7285", "p580", "p836", "p6782",
    "p3654", "p1974", "p6258", "p925", "p949", "p2790", "p698", "p6373", "p2785", "p1222", "p2751",
    "p3825", "p5115", "p1827", "p3171", "p119", "p850", "p3258", "p7909", "p1322", "p8097", "p22",
    "p7478", "p1349", "p4854", "p2929", "p7335", "p5868", "p454", "p7945", "p2654", "p3493",
    "p1060", "p8545", "p6509", "p5002", "p7732", "p3082", "p1779", "p2709", "p7398", "p8879",
    "p639", "p598", "p5672", "p6553", "p4111", "p1417", "p7991", "p380", "p8459", "p8347", "p1769",
    "p2673", "p3330", "p7051", "p1337", "p4057", "p4839", "p6060", "p7095", "p278", "p1445",
    "p6518", "p2364", "p1958", "p548", "p4010", "p3072", "p6993", "p8575", "p2149", "p240",
    "p2920", "p5588", "p1885", "p6082", "p9026", "p340", "p159", "p7730", "p7962", "p1987",
    "p3876", "p8771", "p5123", "p3866", "p3546", "p7777", "p115", "p5337", "p475", "p1724",
    "p6359", "p4260", "p2110", "p1845", "p4335", "p4133", "p783", "p8479", "p1448", "p1160",
    "p7647", "p2618", "p3630", "p4013", "p5242", "p7957", "p3852", "p3889", "p1387", "p439",
    "p1425", "p2061", "p7395", "p7837", "p5147", "p2319", "p3781", "p1311", "p4733", "p8705",
    "p3094", "p2823", "p1914", "p954", "p4381", "p4044", "p593", "p8300", "p7558", "p6494",
    "p6330", "p5940", "p7126", "p1061", "p6352", "p5186", "p1944", "p2285", "p6673", "p5746",
    "p208", "p492", "p216", "p979", "p1668", "p6620", "p711", "p7733", "p8619", "p5157", "p829",
    "p3180", "p3979", "p1556", "p3379", "p5727", "p596", "p2127", "p581", "p2652", "p2628",
    "p1849", "p4238", "p606", "p1224", "p1629", "p1413", "p957", "p8592", "p2254", "p1323", "p122",
    "p2093", "p1100", "p81", "p323", "p815", "p2581", "p543", "p6037", "p2397", "p5513", "p4495",
    "p5776", "p17", "p4590", "p8228", "p708", "p3792", "p3790", "p7090", "p1943", "p4246", "p559",
    "p3738", "p2167", "p1933", "p2162", "p549", "p3025", "p1182", "p4358", "p636", "p986", "p8490",
    "p3340", "p90", "p1487", "p1639", "p1547", "p4152", "p1498", "p1740", "p6157", "p217", "p2201",
    "p362", "p2146", "p1801", "p5063", "p7339", "p663", "p38", "p1336", "p3215", "p210", "p6075",
    "p55", "p2411", "p7445", "p5767", "p2812", "p472", "p803", "p4236", "p7665", "p1607", "p1316",
    "p7475", "p3001", "p1473", "p3537", "p3070", "p1390", "p1290", "p2499", "p154", "p7518",
    "p408", "p1811", "p1734", "p7342", "p8722", "p1754", "p7657", "p583", "p830", "p6690", "p1552",
    "p2498", "p1296", "p3686", "p157", "p487", "p6119", "p4926", "p4846", "p1536", "p2674",
    "p1645", "p3187", "p1058", "p2039", "p4071", "p4433", "p1175", "p434", "p1001", "p2816",
    "p820", "p2696", "p4681", "p2085",
];
