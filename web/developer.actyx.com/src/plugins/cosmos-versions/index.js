// eslint-disable-next-line @typescript-eslint/no-var-requires
const fs = require('fs/promises')
const path = require('path')
const { constants: fsConstants } = require('fs')
const { getChanges } = require('./changes')
const { COSMOS_RELEASE_PATH } = require('./consts')
const { getVersionsAndCommits } = require('./versions')

const cosmosReleaseAvailable = async () =>
  fs
    .access(COSMOS_RELEASE_PATH, fsConstants.X_OK)
    .then(() => true)
    .catch(() => false)

const addRedirect = async (createData, addRoute, path, to) => {
  addRoute({
    path,
    component: `@site/src/components/page-templates/redirect.tsx`,
    modules: {
      to: await createData(`redirect_${path}.json`, JSON.stringify(to)),
    },
    exact: true,
  })
}

const addReleasePage = async (
  createData,
  addRoute,
  product,
  version,
  commit,
  changes,
  otherVersions,
) => {
  const data = {
    version,
    commit,
    changes,
    otherVersions,
  }

  addRoute({
    path: `/releases/${product}/${version}`,
    component: `@site/src/components/page-templates/releases/${product}.tsx`,
    modules: {
      data: await createData(`release_${product}_${version}.json`, JSON.stringify(data)),
    },
    exact: true,
  })
}

const plugin = () => ({
  name: 'cosmos-versions',
  loadContent: async () => {
    if (!(await cosmosReleaseAvailable())) {
      console.error(
        `ERR: *************************** DID NOT FIND COSMOS-RELEASE ***************************`,
      )
      console.error(
        `ERR: The cosmos-release program is required to build this site with up to date release`,
      )
      console.error(
        `ERR: information (product versions and downloads). I expected to find the executable at`,
      )
      console.error(`ERR: ${COSMOS_RELEASE_PATH}.`)
      console.error(`ERR:`)
      console.error(
        `ERR: Please have a look at why I couldn't find it. Maybe you just need to build cosmos-`,
      )
      console.error(`ERR: release by running 'cargo build --release' in the following folder:`)
      console.error(`ERR: ${path.join(COSMOS_RELEASE_PATH, '..', '..', '..')}`)
      console.error(
        `ERR: ***********************************************************************************`,
      )
      process.exit(1)
    }

    const versionsAndCommits = await getVersionsAndCommits()
    console.log(`cosmos-versions found the following product versions:`)
    Object.entries(versionsAndCommits).forEach(([name, vacs]) => {
      console.log(` - ${name}: ${vacs.map(([v, h]) => `${v}:${h}`).join(', ')}`)
    })

    /**
     *  Shape
     * {
     *  actyx: [
     *    {
     *      version: "2.2.1",
     *      commit: "abdas",
     *      changes: ["change a"]
     *    }
     *
     *  ]
     * }
     */
    const history = {}
    for (product of Object.keys(versionsAndCommits)) {
      history[product] = []
      for (vac of versionsAndCommits[product]) {
        const [version, commit] = vac
        history[product].push({
          version,
          commit,
          changes: await getChanges(product, version),
        })
      }
    }

    return { history }
  },
  contentLoaded: async ({ content: { history }, actions: { createData, addRoute } }) => {
    for (product of Object.keys(history)) {
      const allVersions = history[product].map(({ version }) => version)
      for ({ version, commit, changes } of history[product]) {
        const otherVersions = allVersions.filter((v) => v !== version)
        await addReleasePage(createData, addRoute, product, version, commit, changes, otherVersions)
      }
      if (allVersions.length > 0) {
        addRedirect(
          createData,
          addRoute,
          `/releases/${product}/latest`,
          `/releases/${product}/${allVersions[0]}`,
        )
      }
    }
    addRoute({
      path: `/releases`,
      component: `@site/src/components/page-templates/releases/index.tsx`,
      modules: {
        data: await createData(`releases_index.json`, JSON.stringify(history)),
      },
      exact: true,
    })
  },
})

module.exports = plugin
