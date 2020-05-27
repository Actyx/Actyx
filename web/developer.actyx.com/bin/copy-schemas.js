var copydir = require('copy-dir');
var mkdirp = require('mkdirp');

mkdirp('static/schemas/').then(() => {
    copydir('../../protocols/json-schema/os','static/schemas/os/', {}, err => {
        if (err) { throw err }
    })
})