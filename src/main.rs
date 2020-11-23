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
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Hash, PartialEq, Eq)]
enum Category {
    Trashness,
    Sex,
    Alcohol,
    Drugs
}

#[derive(Serialize, Debug)]
struct Choice {
    text : String,
    score : HashMap<Category, i32>
}

#[derive(Debug, Serialize)]
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

lazy_static::lazy_static! {
    static ref QUESTIONS: Vec<Question> = vec![
        Question {
            question: "sa pluse??".to_string(),
            choices: vec![ 
                Choice {
                    text : "ui".to_string(),
                    score: map! {
                        Sex => 10,
                        Alcohol => 5
                    }
                },
                Choice {
                    text : "nope".to_string(),
                    score: map! {}
                }
            ],
            id: 0,
        }
    ];
}

#[derive(Clone, Debug, Serialize)]
struct Session {
    cookie : String,
    score : HashMap<Category, i32>
}

#[tokio::main]
async fn main() {
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
        .manage(Mutex::new(Vec::<Session>::new()))
        .launch()
        .await
        .unwrap();
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
        Sex => 69,
        Drugs => 420
    };
    session.push(Session { cookie : "🍪".to_string(), score : score});
    cookies.add(Cookie::new("session", "🍪"));
    Redirect::to(uri!(question : 0))
}

#[get("/<id_question>")]
fn question(id_question: usize) -> Template {
    let question = &QUESTIONS[id_question];
    Template::render("question", question)
}

#[get("/<id_question>/<id_rep>")]
fn register_answer(id_question: usize, id_rep: usize, cookies: Cookies, sessions: State<Mutex<Vec<Session>>>) -> Redirect {
    let session_id = cookies.get("session").unwrap();
    let mut sessions = sessions.lock().unwrap();
    let mut session = sessions.iter_mut().find(|x| x.cookie == session_id.value()).unwrap();
    for (category, to_add) in QUESTIONS[id_question].choices[id_rep].score.clone() {
        *session.score.entry(category).or_insert(0) += to_add;
    }

    if id_question + 1 >= QUESTIONS.len() {
        Redirect::to(uri!(score))
    }
    else {
        Redirect::to(uri!(question : id_question = id_question + 1))
    }
}

#[get("/score")]
fn score(sessions: State<Mutex<Vec<Session>>>, cookies: Cookies) -> Template {
    
    let session = get_session!(sessions, cookies);
    let mut nom_de_variable_j_ai_pas_d_inspi = HashMap::new();
    nom_de_variable_j_ai_pas_d_inspi.insert("session", session);
    Template::render("score", nom_de_variable_j_ai_pas_d_inspi)
}

