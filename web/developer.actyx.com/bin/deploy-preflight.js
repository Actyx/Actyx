const fs = require('fs')
const path = require('path')

if (!process.env.NETLIFY_AUTH_TOKEN) {
  console.error('Required environment variable NETLIFY_AUTH_TOKEN not found')
  process.exit(1)
}

if (!process.env.NETLIFY_SITE_ID) {
  console.error('Required environment variable NETLIFY_SITE_ID not found')
  process.exit(1)
}

const buildDir = path.join(__dirname, '..', 'build')

if (!fs.existsSync(path.join(buildDir, 'index.html'))) {
  console.error(
    `Unable to find file ${path.join(
      buildDir,
      'index.html',
    )}; are you sure the site has been built?`,
  )
  process.exit(1)
}

console.log(`Deploy pre-flight succeeded!`)
