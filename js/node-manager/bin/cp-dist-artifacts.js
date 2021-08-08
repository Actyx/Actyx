const mkdirp = require('mkdirp')
const cpy = require('cpy')
const path = require('path')
const fs = require('fs')
const ROOT = path.join(__dirname, '..')
const DIST = path.join(ROOT, 'dist')
const ARTIFACTS = path.join(DIST, 'artifacts')
const version = require('../package.json').version

const possibleDistArtifacts = [
  // Windows MSI build (`make node-manager-win` when run on default linux build agent)
  'actyx-node-manager-windows-x64.msi',
  // Mac DMG (`make node-manager-mac-linux` when run on mac build agent)
  'ActyxNodeManager-x64.dmg',
  // Linux .deb/.rpm (`make node-manager-mac-linux` when run on default linux build agent)
  'actyx-node-manager-amd64.deb',
  'actyx-node-manager-x86_64.rpm',
]

;(async () => {
  console.log(`creating artifacts dir ${ARTIFACTS}`)
  mkdirp.sync(ARTIFACTS)

  for (const artifact of possibleDistArtifacts.map((a) => path.join(DIST, a))) {
    if (fs.existsSync(artifact)) {
      console.log(`found artifact at ${artifact}`)
      console.log(`copying artifact to ${ARTIFACTS}`)
      await cpy(artifact, ARTIFACTS)
    } else {
      console.log(`did not find artifact at ${artifact}`)
    }
  }
})()
