const { exec } = require('child_process')
const { COSMOS_RELEASE_PATH } = require('./consts')
const getChanges = async (product, version) => {
  return new Promise((resolve, reject) => {
    try {
      exec(`"${COSMOS_RELEASE_PATH}" changes ${product} ${version}`, (err, stdout, stderr) => {
        if (err) {
          if (
            stderr.includes('no changes since') &&
            stderr.includes('is the very first release of')
          ) {
            resolve([])
          } else {
            reject(new Error(`XXX ${err}: ${stderr}`))
          }
        }
        resolve(
          stdout
            .trim()
            .split(/\r?\n/)
            .filter((l) => l !== ''),
        )
      })
    } catch (error) {
      reject(new Error(`unable to find changes for ${version} of ${product}: ${error}`))
    }
  })
}

module.exports = { getChanges }
