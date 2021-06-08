const { PRODUCTS, COSMOS_RELEASE_PATH } = require('./consts')
const { exec } = require('child_process')

const getVersionsAndCommitsForProduct = async (product) => {
  return new Promise((resolve, reject) => {
    try {
      exec(`"${COSMOS_RELEASE_PATH}" versions -c ${product}`, (err, stdout, stderr) => {
        if (err) {
          reject(new Error(`${err}: ${stderr} ${stdout}`))
        }
        resolve(
          stdout
            .trim()
            .split(/\r?\n/)
            .map((l) => l.split(' ')),
        )
      })
    } catch (error) {
      reject(new Error(`unable to find version of ${product}: ${error}`))
    }
  })
}

const getVersionsAndCommits = async () => {
  const versions = {}
  for (product of PRODUCTS) {
    versions[product] = await getVersionsAndCommitsForProduct(product)
  }
  return versions
}

module.exports = {
  getVersionsAndCommits,
}
