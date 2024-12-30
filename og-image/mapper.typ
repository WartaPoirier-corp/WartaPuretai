#let _mk-bezier(degree) = {
  let params = "p0: 0, " + range(1, degree).map(d => "p" + str(d) + ": none").join(", ") + ", p" + str(degree) + ": 1"

  let pow(x, e) = {
    if x == 0 and e == 0 {
      1
    } else {
      calc.pow(x, e)
    }
  }
  
  let cs = range(degree + 1).map(d => {
    let comb = calc.binom(degree, d)
    let nmi = degree - d
    str(comb) + " * pow(1 - t, " + str(nmi) + ") * pow(t, " + str(d) + ")" + " * p" + str(d)
  }).join(" + ")

  let code = "pow => (" + params + ", t) => {" + cs + "}"
  
  eval(code)(pow)
}

#let bezier(t, degree: 2, ..args) = {
  assert(degree > 0)
  _mk-bezier(degree)(..args, t)
}

#let f(x, from: -1, to: 1) = {
  assert(from < 0 and to > 0)
  
  if x >= 0 {
    x /= to
    bezier(degree: 3, p1: 0.5 * to, p2: .9, x)
  } else {
    -bezier(degree: 3, p1: 0.5, p2: .9, -x)
  }
}

= Preview

#import "@preview/cetz:0.3.1"
#import "@preview/cetz-plot:0.1.0": plot

#cetz.canvas({
  import cetz.draw: *
  import cetz.plot

  let from = -1
  let to = 2
  
  plot.plot(size: (5, 5), axis-style: none, {
    plot.add(domain: (from, to), f.with(from: from, to: to))
    plot.add(((0, 0),), mark: "o")
  })
})
