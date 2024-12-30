#let dummy-inputs = (
  name: "Edgar",
  trashness: (from: -20, to: 100, value: 90),
  sex: (from: -30, to: 100, value: -10),
  alcohol: (from: -1000, to: 200, value: -50),
  drugs: (from: -20, to: 100, value: -20),
  lang: "fr"
)

#let inputs = if not sys.inputs.keys().contains("wartapuretai-inputs") {
  dummy-inputs
} else {
  json.decode(sys.inputs.wartapuretai-inputs)
}
