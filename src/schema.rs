use core::fmt;
use enum_map::EnumMap;
use serde::de::MapAccess;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::iter::Sum;
use std::ops::{Add, Deref};
use vec1::Vec1;

#[derive(
    Copy, Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq, Ord, PartialOrd, enum_map::Enum,
)]
pub enum Category {
    Trashness,
    Sex,
    Alcohol,
    Drugs,
}

#[derive(Clone, Copy, Debug, Default, Hash, Serialize)]
#[serde(transparent)]
pub struct Score {
    pub map: EnumMap<Category, i32>,
}

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            map: EnumMap::from_fn(|cat| self.map[cat] + rhs.map[cat]),
        }
    }
}

struct HumanReadableVisitor;

impl<'de> de::Visitor<'de> for HumanReadableVisitor {
    type Value = EnumMap<Category, i32>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a map")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
        let mut entries = EnumMap::default();
        while let Some((key, value)) = access.next_entry()? {
            entries[key] = value;
        }
        Ok(entries)
    }
}

impl Sum for Score {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Score::default(), Add::add)
    }
}

impl<'de> Deserialize<'de> for Score {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_map(HumanReadableVisitor)
            .map(|map| Self { map })
    }
}

#[derive(Debug, Hash, Serialize, Deserialize)]
pub struct Choice {
    pub text: String,
    pub score: Score,
}

#[derive(Debug, Hash, Serialize, Deserialize)]
pub struct Question {
    pub question: String,
    pub choices: Vec1<Choice>,
    pub id: u32,
}

impl Question {
    fn score_reduce(&self, mut f: impl FnMut(i32, i32) -> i32) -> Score {
        Score {
            map: EnumMap::from_fn(|cat| {
                self.choices
                    .iter()
                    .map(|c| c.score.map[cat])
                    .reduce(&mut f)
                    .unwrap() // Vec1 always has at least one element
            }),
        }
    }

    pub fn score_min(&self) -> Score {
        self.score_reduce(std::cmp::min)
    }

    pub fn score_max(&self) -> Score {
        self.score_reduce(std::cmp::max)
    }
}

pub struct Questions {
    pub questions: Vec<Question>,

    /// Minimum possible score, for use in the OpenGraph image gauges
    pub mins: Score,

    /// Maximum possible score, for use in the OpenGraph image gauges
    pub maxes: Score,

    /// See [`Session::questions_hash`]
    pub cached_hash: u64,
}

impl From<Vec<Question>> for Questions {
    fn from(questions: Vec<Question>) -> Self {
        let mut hasher = DefaultHasher::new();
        questions.hash(&mut hasher);

        let mins = questions.iter().map(Question::score_min).sum();
        let maxes = questions.iter().map(Question::score_max).sum();

        Self {
            mins,
            maxes,
            questions,
            cached_hash: hasher.finish(),
        }
    }
}

impl Hash for Questions {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.cached_hash);
    }
}

impl Deref for Questions {
    type Target = [Question];

    fn deref(&self) -> &Self::Target {
        &self.questions
    }
}
