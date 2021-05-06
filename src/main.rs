use rocket_contrib::templates::Template;
use rocket_contrib::serve::StaticFiles;
use rocket::get;
use rocket::post;
use rocket::State;
use rocket::response::Redirect;
use rocket::uri;
use rocket::http::{Cookie, Cookies};
use std::collections::HashMap;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
enum Category {
    Trashness,
    Sex,
    Alcohol,
    Drugs
}

#[derive(Deserialize, Serialize, Debug)]
struct Choice {
    text : String,
    score : HashMap<Category, i32>
}

#[derive(Debug, Serialize, Deserialize)]
struct Question {
    question : String,
    choices : Vec<Choice>,
    id : u32,
}

macro_rules! map {
    ($($cat:ident => $score:expr),*) => {
        {
            #[allow(unused_mut)]
            let mut hm = HashMap::new();
            
             $(hm.insert(Category::$cat, $score);)*

            hm
        }
    };
}

macro_rules! get_session {
    ($sessions:ident, $cookies:ident) => {
        {
            let session_id = $cookies.get("session").unwrap();
            let mut sessions = $sessions.lock().unwrap();
            sessions.iter_mut().find(|x| x.cookie == session_id.value()).unwrap().clone()
        }
    };
}

#[derive(Clone, Debug, Serialize)]
struct Session {
    cookie : String,
    score : HashMap<Category, i32>
}

#[tokio::main]
async fn main() {
    // initalisation serde
    let question_string = fs::read_to_string("questions.json").await.unwrap();
    let questions: Vec<Question> = serde_json::from_str(&question_string).unwrap();

    for (i, q) in questions.iter().enumerate() {
        if (i as u32) != q.id {
            println!("WARNING: question {} has id {}", i, q.id);
        }
    }

    rocket::ignite()
        .mount("/", rocket::routes![
            home,
            create_session,
            question,
            register_answer,
            score,
        ])
        .mount("/static", StaticFiles::from("static"))
        .attach(Template::fairing())
        .manage(Mutex::new(questions))
        .manage(Mutex::new(Vec::<Session>::new()))
        .launch()
        .await
        .unwrap();
}

#[get("/")]
fn home() -> Template {
    let truc = HashMap::<i32, i32>::new();
    Template::render("index", truc)
}

#[post("/start")]
fn create_session(session : State<Mutex<Vec<Session>>>, mut cookies: Cookies) -> Redirect {
    let mut session = session.lock().unwrap();
    let score = map!{
        Trashness => 0,
        Sex => 0,
        Alcohol => 0,
        Drugs => 0
    };
    session.push(Session { cookie : "🍪".to_string(), score : score});
    cookies.add(Cookie::new("session", "🍪"));
    Redirect::to(uri!(question : 0))
}

#[get("/<id_question>")]
fn question(id_question: usize, questions: State<Mutex<Vec<Question>>>) -> Template {
    let questions_locked = questions.lock().unwrap();
    let question = &questions_locked[id_question];
    Template::render("question", question)
}

#[get("/<id_question>/<id_rep>")]
fn register_answer(
    id_question: usize,
    id_rep: usize,
    cookies: Cookies,
    sessions: State<Mutex<Vec<Session>>>,
    questions: State<Mutex<Vec<Question>>>
) -> Redirect {
    let session_id = cookies.get("session").unwrap();
    let mut sessions = sessions.lock().unwrap();
    let session = sessions.iter_mut().find(|x| x.cookie == session_id.value()).unwrap();
    let questions_locked = questions.lock().unwrap();
    for (category, to_add) in questions_locked[id_question].choices[id_rep].score.clone() {
        *session.score.entry(category).or_insert(0) += to_add;
    }

    if id_question + 1 >= questions_locked.len() {
        Redirect::to(uri!(score))
    } else {
        Redirect::to(uri!(question : id_question = id_question + 1))
    }
}

#[get("/score")]
fn score(sessions: State<Mutex<Vec<Session>>>, cookies: Cookies) -> Template {
    let session = get_session!(sessions, cookies);
    let mut template_vars = HashMap::new();
    template_vars.insert("session", session);
    Template::render("score", template_vars)
}

