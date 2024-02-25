use crate::{Category, Session};
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use bincode::error::{DecodeError, EncodeError};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use enum_map::{enum_map, EnumMap};
use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::{Redirect, Responder};
use rocket::serde::Serialize;
use rocket::{get, post, uri, FromForm, State};
use rocket_dyn_templates::Template;
use std::sync::Mutex;

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
}

#[derive(FromForm)]
pub struct ScoreSaveForm<'a> {
    #[field(validate = len(1..=50))]
    name: &'a str,
}

#[post("/score/share", data = "<form>")]
pub fn score_share(
    sessions: &State<Mutex<Vec<Session>>>,
    cookies: &CookieJar<'_>,
    form: Form<ScoreSaveForm>,
) -> Redirect {
    let session = super::get_session!(sessions, cookies);
    // Should be large enough
    let mut encoded_raw = [0u8; 128];

    let encoded_raw = EncodedV1 {
        timestamp: Utc::now().timestamp() / 60,
        trashness: session.score[Category::Trashness],
        sex: session.score[Category::Sex],
        alcohol: session.score[Category::Alcohol],
        drugs: session.score[Category::Drugs],
        player_name: form.name,
    }
    .encode(&mut encoded_raw)
    .unwrap();

    let share_string = BASE64_URL_SAFE_NO_PAD.encode(encoded_raw);
    Redirect::to(uri!(exported_score(&share_string)))
}

#[derive(Serialize)]
struct ScoreTemplateShared {
    scores: EnumMap<Category, i32>,
    shared_by: String,
    shared_at_rfc3339: String,
    shared_at: String,
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
pub fn exported_score(share_string: &str) -> impl Responder {
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
        ScoreTemplateShared {
            scores: enum_map! {
                Category::Trashness => encoded.trashness,
                Category::Sex => encoded.sex,
                Category::Alcohol => encoded.alcohol,
                Category::Drugs => encoded.drugs,
            },
            shared_by: encoded.player_name.to_string(),
            shared_at_rfc3339,
            shared_at,
        },
    ))
}
