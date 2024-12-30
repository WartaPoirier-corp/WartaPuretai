#import "z-place.typ": z-place, z-place-container
#import "inputs.typ": inputs
#import "date.typ": *
#import "mapper.typ": f

#show: z-place-container

#set text(lang: "fr")

#let width = 1200
#let height = 630

#let ppi = 144
#let px(px) = px / ppi * 1in

#let date = datetime.today()

#let z-stack(..items) = {
  grid(
    columns: items.pos().len() * (1fr,),
    column-gutter: -100%,
    rows: 1,
    ..items
  )
}

#let background-whitening = 70%

#let background-pat = rect(width: 100%, height: 100%, stroke: none, fill: pattern(
  size: (px(512),) * 2,
  image("bg.png", width: px(512)),
))

#let background = z-stack(
  background-pat,
  rect(width: 100%, height: 100%, fill: white.transparentize(100% - background-whitening))
)

#set page(width: px(width), height: px(height), margin: px(80), background: background)

#set text(font: "Nerko One", size: px(40))

#let progress-split = 30%

#let stripes(color) = pattern(
  size: (px(20),) * 2,
  block(fill: color.lighten(50%), width: 100%, height: 100%, {
    set line(stroke: (thickness: px(8), paint: color.lighten(20%)))
    place(line(start: (-50%, 100%), end: (100%, -50%)))
    place(line(start: (0%, 150%), end: (150%, 0%)))
  })
)

#let gauge-label(body, side: right) = layout(l => context {
  let body = pad(body, x: 0.2em)
  let available-width = l.width
  let body-width = measure(body).width
  let body-height = measure(body).height

  if body-width > available-width {
    let dx = if side == right {
      -body-width
    } else {
      available-width
    }
    
    z-place(z-index: 1, {
      set text(fill: white)
      body
    }, dx: dx, dy: -body-height / 2)
  } else {
    body
  }
})

#let mapped-progress(gauge-name) = {
  let (from, to, value) = inputs.at(gauge-name)
  assert(value <= to)
  assert(value >= from)
  
  let x = if value >= 0 {
    value / to * 2
  } else {
    -value / from
  }
  
  let i = f(from: -1, to: 2, x)
  
  (value, i * 100%)
}

#let third = 100% / 3

#let progress(cat, color, stroke: px(3), bar-height: px(44)) = {
  let (value, amount) = mapped-progress(cat)
  
  (
    image(cat + ".png", height: px(64)),
    z-stack(
      rect(width: 100%, height: bar-height, fill: white, stroke: none),
      block(
        clip: false,
        height: bar-height,
        grid(
          columns: (1fr, 2fr),
          align: (right, left),
          if amount < 0% {
            grid(
              columns: (1fr, -amount),
              gauge-label(side: left)[#value],
              rect(fill: stripes(color), width: 100%, height: 100%),
            )
          },
          if amount >= 0% {
            grid(
              columns: (amount, 1fr),
              rect(fill: color, width: 100%, height: 100%),
              gauge-label[#value pts]
            )
          },
        ),
      ),
      rect(width: 100%, height: bar-height, stroke: stroke),
      place(line(start: (third, -50%), end: (third, 150%), stroke: (paint: red, thickness: px(6)))),
    ),
  )
}

#grid(
  columns: (auto, 1fr, 25%),
  rows: (auto, 1fr, 1fr, 1fr, 1fr),
  row-gutter: px(40),
  column-gutter: (px(12), px(72)),
  align: (x, y) => (
    (auto, left + horizon, center + horizon),
    (center + horizon, center + horizon, right + bottom),
  ).at(calc.min(1, y)).at(x),
  [],
  grid.cell(colspan: 2,{
    set par(leading: 0.4em)
    set text(size: px(60))
    [Impureté de #underline(inputs.name)\ le #date-format(date) :]
    v(px(40))
  }),
  grid.cell(x: 2, y: 1, rowspan: 4, {
    set par(leading: 0.2em)
    image("logo.png", height: px(140))
    text(size: px(54), fill: red)[WartaPureté\ ]
    text(size: px(24), fill: black.lighten(40%), font: "Montserrat")[pure.wp-corp.eu.org]
  }),
  ..progress("trashness", rgb("#050729")),
  ..progress("sex", rgb("#ff0086")),
  ..progress("alcohol", rgb("#ff7600")),
  ..progress("drugs", rgb("#0c9500")),
)
