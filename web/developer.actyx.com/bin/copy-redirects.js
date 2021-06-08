var copydir = require('copy-dir')

copydir('netlify/', 'build/', {}, (err) => {
  if (err) {
    throw err
  }
})
