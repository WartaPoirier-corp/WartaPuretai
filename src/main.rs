mod schema;
mod sharing;

use crate::schema::{Category, Question, Questions, Score};
use arrayvec::ArrayString;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::response::Redirect;
use rocket::uri;
use rocket::State;
use rocket::{async_trait, post};
use rocket::{get, Request};
use rocket_dyn_templates::Template;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use tokio::fs;

#[derive(Clone, Debug, bincode::Encode, bincode::Decode)]
struct Session {
    questions_hash: u64,
    answers: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
enum SessionError {
    #[error("no active session")]
    Missing,

    #[error(transparent)]
    Base64(#[from] base64::DecodeSliceError),

    #[error(transparent)]
    Bincode(#[from] bincode::error::DecodeError),
}

impl Session {
    pub fn register_answer(&mut self, idx: usize, value: u8) {
        match idx.cmp(&self.answers.len()) {
            Ordering::Equal => self.answers.push(value),

            // Shouldn't occur in normal circumstances, but may happen when users "jump" questions
            // using their URL bar. We allow that as it makes debugging easier.
            Ordering::Less => self.answers[idx] = value,
            Ordering::Greater => {
                self.answers.resize(idx + 1, 255);
                self.answers[idx] = value;
            }
        }
    }

    pub fn score(&self, questions: &Questions) -> Option<Score> {
        if self.answers.len() != questions.questions.len() {
            None
        } else if self.questions_hash != questions.cached_hash {
            None
        } else {
            Some(
                questions
                    .iter()
                    .zip(&self.answers)
                    .filter_map(|(question, answer)| question.choices.get(*answer as usize))
                    .map(|c| c.score)
                    .sum(),
            )
        }
    }

    pub fn encode(&self) -> ArrayString<1024> {
        let mut bincoded = [0u8; 512];
        let bincoded_len =
            bincode::encode_into_slice(self, &mut bincoded, bincode::config::standard()).unwrap();

        let mut base64 = [0u8; 1024];
        let base64_len = BASE64_URL_SAFE_NO_PAD
            .encode_slice(&bincoded[..bincoded_len], &mut base64)
            .unwrap();

        ArrayString::from(std::str::from_utf8(&base64[..base64_len]).unwrap()).unwrap()
    }

    pub fn decode(base64: &str) -> Result<Self, SessionError> {
        let mut un_base64 = [0u8; 512];
        let un_base64_len = BASE64_URL_SAFE_NO_PAD
            .decode_slice(base64, &mut un_base64)
            .map_err(SessionError::Base64)?;

        bincode::decode_from_slice(&un_base64[..un_base64_len], bincode::config::standard())
            .map_err(SessionError::Bincode)
            .and_then(|(session, read)| {
                if read == un_base64_len {
                    Ok(session)
                } else {
                    Err(SessionError::Bincode(bincode::error::DecodeError::Other(
                        "remaining input",
                    )))
                }
            })
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = SessionError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let Some(cookie) = request.cookies().get("session") else {
            return Outcome::Error((Status::Forbidden, SessionError::Missing));
        };

        match Self::decode(cookie.value()) {
            Ok(session) => Outcome::Success(session),
            Err(err) => Outcome::Error((Status::BadRequest, err)),
        }
    }
}

impl From<Session> for Cookie<'static> {
    fn from(session: Session) -> Self {
        Cookie::new("session", session.encode().to_string())
    }
}

#[derive(Deserialize)]
struct Config {
    /// Base URL with scheme, without trailing slash, i.e. `https://pure.wp-corp.eu.org`
    base_url: String,
}

#[rocket::launch]
async fn launch() -> _ {
    // initalisation serde
    let question_string = fs::read_to_string("questions.json").await.unwrap();
    let questions: Vec<Question> = serde_json::from_str(&question_string).unwrap();
    let questions = Questions::from(questions);

    for (i, q) in questions.iter().enumerate() {
        if (i as u32) != q.id {
            println!("WARNING: question {} has id {}", i, q.id);
        }
    }

    rocket::build()
        .mount(
            "/",
            rocket::routes![
                home,
                create_session,
                question,
                register_answer,
                score,
                sharing::score_share,
                sharing::exported_score,
                sharing::exported_score_og_image,
            ],
        )
        .mount("/static", FileServer::from("static"))
        .attach(AdHoc::config::<Config>())
        .attach(Template::fairing())
        .manage(questions)
}

#[get("/")]
fn home(questions: &State<Questions>) -> Template {
    Template::render(
        "index",
        rocket_dyn_templates::context! {
            question_count: questions.len(),
        },
    )
}

#[post("/start")]
fn create_session(questions: &State<Questions>, cookies: &CookieJar<'_>) -> Redirect {
    let session = Session {
        questions_hash: questions.cached_hash,
        answers: Vec::new(),
    };

    cookies.add(session);
    Redirect::to(uri!(question(0)))
}

#[get("/<id_question>", rank = 20)]
fn question(id_question: usize, questions: &State<Questions>) -> Option<Template> {
    let question = questions.get(id_question)?;
    Some(Template::render("question", question))
}

#[get("/<id_question>/<id_rep>", rank = 20)]
fn register_answer(
    mut session: Session,
    id_question: usize,
    id_rep: usize,
    cookies: &CookieJar<'_>,
    questions: &State<Questions>,
) -> Redirect {
    session.register_answer(id_question, id_rep as u8);
    cookies.add(session);

    if id_question + 1 >= questions.len() {
        Redirect::to(uri!(score))
    } else {
        Redirect::to(uri!(question(id_question = id_question + 1)))
    }
}

#[get("/score")]
fn score(session: Session, questions: &State<Questions>) -> Template {
    let mut template_vars = HashMap::new();
    template_vars.insert("scores", session.score(questions));
    Template::render("score", template_vars)
}
