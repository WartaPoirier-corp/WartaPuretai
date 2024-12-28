use crate::{Category, Config, Questions, Score, Session};
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use bincode::error::{DecodeError, EncodeError};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use enum_map::enum_map;
use rocket::form::Form;
use rocket::response::{Redirect, Responder};
use rocket::serde::Serialize;
use rocket::{get, post, uri, FromForm, State};
use rocket_dyn_templates::Template;
use std::process::Stdio;
use tokio::process::Command;

/// A struct designed to be [bincode]&base64-encoded into a "share string"
///
/// It should be as stable as possible to never break between versions. [bincode] uses varints
/// internally, so the bit size of integers isn't super important.
#[derive(Clone, Copy, Debug, bincode::Encode, bincode::BorrowDecode)]
struct EncodedV1<'a> {
    /// Unix timestamp in minutes instead of seconds
    timestamp: i64,

    trashness: i32,
    sex: i32,
    alcohol: i32,
    drugs: i32,

    player_name: &'a str,
}

impl<'a> EncodedV1<'a> {
    pub fn encode<'buf>(&self, full_buf: &'buf mut [u8]) -> Result<&'buf [u8], EncodeError> {
        if let [version, buf @ ..] = full_buf {
            *version = 0;
            let count = bincode::encode_into_slice(self, buf, bincode::config::standard())?;
            Ok(&full_buf[..1 + count])
        } else {
            Err(EncodeError::UnexpectedEnd)
        }
    }

    pub fn decode(buf: &'a [u8]) -> Result<Self, DecodeError> {
        if let [0, buf @ ..] = buf {
            bincode::borrow_decode_from_slice::<EncodedV1, _>(buf, bincode::config::standard())
                .and_then(|(encoded, bytes_read)| {
                    if bytes_read == buf.len() {
                        Ok(encoded)
                    } else {
                        Err(DecodeError::LimitExceeded)
                    }
                })
        } else {
            Err(DecodeError::Other("invalid version"))
        }
    }

    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        let timestamp_seconds = self.timestamp.checked_mul(60)?;
        DateTime::<Utc>::from_timestamp(timestamp_seconds, 0)
    }

    pub fn score(&self) -> Score {
        Score {
            map: enum_map! {
                Category::Trashness => self.trashness,
                Category::Sex => self.sex,
                Category::Alcohol => self.alcohol,
                Category::Drugs => self.drugs,
            },
        }
    }
}

#[derive(FromForm)]
pub struct ScoreSaveForm<'a> {
    #[field(validate = len(1..=50))]
    name: &'a str,
}

#[post("/score/share", data = "<form>")]
pub fn score_share(
    session: Session,
    questions: &State<Questions>,
    form: Form<ScoreSaveForm>,
) -> Redirect {
    // Should be large enough
    let mut encoded_raw = [0u8; 128];

    // FIXME: proper error handling
    let score = session.score(questions).unwrap_or_default();

    let encoded_raw = EncodedV1 {
        timestamp: Utc::now().timestamp() / 60,
        trashness: score.map[Category::Trashness],
        sex: score.map[Category::Sex],
        alcohol: score.map[Category::Alcohol],
        drugs: score.map[Category::Drugs],
        player_name: form.name,
    }
    .encode(&mut encoded_raw)
    .unwrap(); // *probably* cannot fail

    let share_string = BASE64_URL_SAFE_NO_PAD.encode(encoded_raw);
    Redirect::to(uri!(exported_score(&share_string)))
}

#[derive(Responder)]
enum ExportedScoreResponse {
    Ok(Template),

    #[response(status = 404)]
    CannotDecode(&'static str),
}

impl ExportedScoreResponse {
    fn cannot_decode() -> Self {
        Self::CannotDecode("Unknown share code")
    }
}

#[get("/score/<share_string>")]
pub fn exported_score<'a>(
    config: &State<Config>,
    share_string: &str,
) -> impl Responder<'a, 'static> {
    let Ok(encoded_raw) = BASE64_URL_SAFE_NO_PAD.decode(share_string) else {
        return ExportedScoreResponse::cannot_decode();
    };

    let Ok(encoded) = EncodedV1::decode(&encoded_raw) else {
        return ExportedScoreResponse::cannot_decode();
    };

    let Some(timestamp) = encoded.timestamp() else {
        return ExportedScoreResponse::cannot_decode();
    };

    let shared_at_rfc3339 = timestamp.to_rfc3339();
    let timestamp_paris = timestamp.with_timezone(&Tz::Europe__Paris);
    let shared_at = timestamp_paris.format("%d/%m/%Y à %Hh%M").to_string();

    ExportedScoreResponse::Ok(Template::render(
        "score",
        rocket_dyn_templates::context! {
            base_url: &config.base_url,
            scores: encoded.score(),
            shared_by: encoded.player_name,
            shared_at_rfc3339,
            shared_at,
            share_string,
        },
    ))
}

#[derive(Responder)]
pub enum ExportedScoreOgImageResponse {
    #[response(content_type = "image/png")]
    Ok(Vec<u8>),

    #[response(status = 404)]
    CannotDecode(&'static str),

    #[response(status = 503)]
    GenerationError(String),
}

impl ExportedScoreOgImageResponse {
    fn cannot_decode() -> Self {
        Self::CannotDecode("Unknown share code")
    }

    fn generation_error(err: String) -> Self {
        Self::GenerationError(err)
    }
}

#[get("/score/<share_string>/og.png")]
pub async fn exported_score_og_image(
    questions: &State<Questions>,
    share_string: &str,
) -> ExportedScoreOgImageResponse {
    let Ok(encoded_raw) = BASE64_URL_SAFE_NO_PAD.decode(share_string) else {
        return ExportedScoreOgImageResponse::cannot_decode();
    };

    let Ok(encoded) = EncodedV1::decode(&encoded_raw) else {
        return ExportedScoreOgImageResponse::cannot_decode();
    };

    let Some(timestamp) = encoded.timestamp() else {
        return ExportedScoreOgImageResponse::cannot_decode();
    };

    #[derive(Serialize)]
    struct Gauge {
        from: i32,
        to: i32,
        value: i32,
    }

    let make_gauge = |cat: Category| Gauge {
        from: questions.mins.map[cat],
        to: questions.maxes.map[cat],
        value: encoded.score().map[cat],
    };

    let inputs = rocket_dyn_templates::context! {
        name: &encoded.player_name,
        lang: "fr",
        trashness: make_gauge(Category::Trashness),
        sex: make_gauge(Category::Sex),
        alcohol: make_gauge(Category::Alcohol),
        drugs: make_gauge(Category::Drugs),
    };

    let inputs_json = serde_json::to_string(&inputs).unwrap();

    let typst = match Command::new("typst")
        .current_dir("og-image")
        .arg("compile")
        .arg("main.typ")
        .arg("--ignore-system-fonts")
        .arg("--font-path=.")
        .arg(format!("--input=wartapuretai-inputs={inputs_json}"))
        .arg(format!("--creation-timestamp={}", timestamp.timestamp()))
        .arg("--format=png")
        .arg("-")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(typst) => typst,
        Err(err) => return ExportedScoreOgImageResponse::generation_error(err.to_string()),
    };

    let output = match typst.wait_with_output().await {
        Ok(output) => output,
        Err(err) => return ExportedScoreOgImageResponse::generation_error(err.to_string()),
    };

    ExportedScoreOgImageResponse::Ok(output.stdout)
}
