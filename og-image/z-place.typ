#let z-placed = state("z-placed", (:))

#let _z-placed-release(cond) = {
  z-placed
    .final()
    .pairs()
    .map(((k, v)) => (int(k), v))
    .filter(((k, v)) => cond(k))
    .sorted(key: ((k, v)) => k)
    .map(((k, placed)) => placed)
    .flatten()
    .map(placed => {
      let args = placed.fields()
      let dx = args.dx
      let dy = args.dy
      let _ = args.remove("body")
      place(placed.body, ..args, dx: dx, dy: dy)
    })
    .join()
}

#let z-place-container(body) = {
  set page(foreground: context {
    block(width: 100%, height: 100%, _z-placed-release(k => k >= 0))
  })
  
  body
}

#let z-place(z-index: 0, dx: 0pt, dy: 0pt, ..args) = context {
  assert(type(z-index) == int)

  let pos = here().position()
  let placed = place(..args, dx: dx + pos.x, dy: dy + pos.y)
  
  if z-index == 0 {
    return placed
  }

  z-placed.update(z-placed => {
    let list = z-placed.at(str(z-index), default: ())
    list.push(placed)
    z-placed.insert(str(z-index), list)
    z-placed
  })
}
