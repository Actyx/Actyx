var copydir = require('copy-dir')
var mkdirp = require('mkdirp')

mkdirp('static/schemas/').then(() => {
  copydir('../../protocols/json-schema', 'static/schemas/', {}, (err) => {
    if (err) {
      throw err
    }
  })
})
