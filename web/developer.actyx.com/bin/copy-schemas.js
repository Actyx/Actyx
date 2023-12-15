var copydir = require('copy-dir')
var mkdirp = require('mkdirp')

mkdirp('static/schemas/').then(() => {
  copydir('../../rust/actyx/ax-core/resources/json-schema', 'static/schemas/', {}, (err) => {
    if (err) {
      throw err
    }
  })
})
