// eslint-disable-next-line @typescript-eslint/no-var-requires
const crNodeUtil = require('cosmos-release/lib/node-util')
// eslint-disable-next-line @typescript-eslint/no-var-requires
const crTypes = require('cosmos-release/lib/types')
// eslint-disable-next-line @typescript-eslint/no-var-requires
const debug = require('debug')

// We are not using the winston logger here because this is actually
// run in another process (the docusaurus site build process)
const log = debug('cosmos-release:docusaurus:plugin')

const plugin = ({ siteConfig }, options) => ({
  name: 'release-management',
  loadContent: async () => {
    log(`Docusaurus cosmos-release plugin loading releases information`)

    const { releasesYml } = options
    log(`Loading release information from '${releasesYml}'`)

    if (!releasesYml || typeof releasesYml !== 'string') {
      const msg = `Missing or invalid property 'releasesYml' in plugin (got: '${releasesYml}').`
      log(msg)
      throw new Error(msg)
    }

    const releases = await crNodeUtil.decodeYamlFromFile(releasesYml, crTypes.Releases)
    if (releases._tag === 'Left') {
      log(releases.left.message)
      log(releases.left.stack)
      if (process.env.NODE_ENV === 'production') {
        throw releases.left
      } else {
        log(`Did not find releases file; ignoring because not in production mode`)
        return null
      }
    }

    log(`Got information about ${releases.right.length} releases`)
    log(JSON.stringify(releases.right, null, 2))

    log('loadContent')
    log('siteConfig')
    log(JSON.stringify(siteConfig))
    log('options')
    log(JSON.stringify(options))
    return {
      releases: releases.right,
    }
  },
  contentLoaded: async ({ content, actions }) => {
    log(`loadContent()`)
    if (content === null && process.env.NODE_ENV === 'production') {
      throw new Error('ARM plugin did not receive content (incl. information about releases)')
    }
    // We skip if not in production
    if (content === null) {
      log(
        `Did not get ARM plugin content (incl. information about releases); ignoring because not in production mode`,
      )
    }
    const releases = content === null ? [] : content.releases
    const { addRoute, createData } = actions

    log(`Releases data:`)
    log(JSON.stringify(releases))

    addRoute({
      path: '/releases',
      component: '@site/src/components/ReleasesPage.tsx',
      modules: {
        releases: await createData(
          'releases.json',
          JSON.stringify(crTypes.Releases.encode(releases)),
        ),
      },
      exact: true,
    })
  },
})

module.exports = plugin
