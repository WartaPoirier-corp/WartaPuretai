#let _months-fr = ("janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre")

#let date-format(date) = context {
  let lang = text.lang

  if lang == "fr" {
    date.display("[day] month [year]").replace("month", _months-fr.at(date.month() - 1))
  } else if lang == "en" {
    date.display("[month repr:long] [day], [year]")
  } else {
    panic("unhandled language: " + lang)
  }
}