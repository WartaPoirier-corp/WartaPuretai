mod sharing;

use enum_map::EnumMap;
use rocket::fs::FileServer;
use rocket::get;
use rocket::http::{Cookie, CookieJar};
use rocket::post;
use rocket::response::Redirect;
use rocket::uri;
use rocket::State;
use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::fs;

#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq, enum_map::Enum)]
enum Category {
    Trashness,
    Sex,
    Alcohol,
    Drugs,
}

#[derive(Deserialize, Serialize, Debug)]
struct Choice {
    text: String,
    score: HashMap<Category, i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Question {
    question: String,
    choices: Vec<Choice>,
    id: u32,
}

#[macro_export]
macro_rules! get_session {
    ($sessions:ident, $cookies:ident) => {{
        let session_id = $cookies.get("session").unwrap();
        let mut sessions = $sessions.lock().unwrap();
        sessions
            .iter_mut()
            .find(|x| x.cookie == session_id.value())
            .unwrap()
            .clone()
    }};
}

#[derive(Clone, Debug, Serialize)]
struct Session {
    cookie: String,
    score: EnumMap<Category, i32>,
}

#[rocket::launch]
async fn launch() -> _ {
    // initalisation serde
    let question_string = fs::read_to_string("questions.json").await.unwrap();
    let questions: Vec<Question> = serde_json::from_str(&question_string).unwrap();

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
            ],
        )
        .mount("/static", FileServer::from("static"))
        .attach(Template::fairing())
        .manage(questions)
        .manage(Mutex::new(Vec::<Session>::new()))
}

#[get("/")]
fn home(questions: &State<Vec<Question>>) -> Template {
    Template::render(
        "index",
        rocket_dyn_templates::context! {
            question_count: questions.len(),
        },
    )
}

#[post("/start")]
fn create_session(session: &State<Mutex<Vec<Session>>>, cookies: &CookieJar<'_>) -> Redirect {
    let mut session = session.lock().unwrap();
    let score = EnumMap::default();

    let sess_id = rand::random::<u32>().to_string();

    session.push(Session {
        cookie: sess_id.clone(),
        score,
    });

    cookies.add(Cookie::new("session", sess_id));
    Redirect::to(uri!(question(0)))
}

#[get("/<id_question>")]
fn question(id_question: usize, questions: &State<Vec<Question>>) -> Template {
    let question = &questions[id_question];
    Template::render("question", question)
}

#[get("/<id_question>/<id_rep>")]
fn register_answer(
    id_question: usize,
    id_rep: usize,
    cookies: &CookieJar<'_>,
    sessions: &State<Mutex<Vec<Session>>>,
    questions: &State<Vec<Question>>,
) -> Redirect {
    let session_id = cookies.get("session").unwrap();
    let mut sessions = sessions.lock().unwrap();
    let session = sessions
        .iter_mut()
        .find(|x| x.cookie == session_id.value())
        .unwrap();

    for (category, to_add) in questions[id_question].choices[id_rep].score.clone() {
        session.score[category] += to_add;
    }

    if id_question + 1 >= questions.len() {
        Redirect::to(uri!(score))
    } else {
        Redirect::to(uri!(question(id_question = id_question + 1)))
    }
}

#[get("/score")]
fn score(sessions: &State<Mutex<Vec<Session>>>, cookies: &CookieJar<'_>) -> Template {
    let session = get_session!(sessions, cookies);
    let mut template_vars = HashMap::new();
    template_vars.insert("scores", session.score);
    Template::render("score", template_vars)
}
