const NetlifyClient = require('netlify')
const fs = require('fs')
const path = require('path')

const log = console.log

const logOnProgress = (ev) => {
  log(ev.msg)
}

/**
 * This function deploys a directory to a Netlify site. If draft is true, it
 * will create a deploy preview (Netlify parlance), otherwise it will create
 * a production deploy. The function waits for the deploy to succeed before
 * returning.
 */
const deployDir = async (siteId, path, netlifyAccessToken, draft, deployMsg) => {
  const client = new NetlifyClient(netlifyAccessToken)

  log(`Starting site deploy (siteId: ${siteId}, path: ${path}, draft: ${draft}).`)
  const res = await client.deploy(siteId, path, {
    draft, // draft deploy or production deploy
    message: deployMsg,
    deployTimeout: 1.2e6,
    parallelHash: 100,
    parallelUpload: 5,
    maxRetry: 5,
  })
  log(`Site deploy completed; permalink: ${res.deploy.links.permalink}`)
  if (!res.deploy.links.permalink) {
    throw new Error(`Netlify unexpectedly did not return a permalink to the deployed site.`)
  }
  log(`{{DEPLOY_PERMALINK=${res.deploy.links.permalink}}}`)
  return
}

;(async () => {
  const SITE_ID = 'fffa1022-c8a0-4238-97c4-dd80979bf887'
  const PATH = 'build'
  const NETLIFY_ACCESS_TOKEN = process.env.NETLIFY_ACCESS_TOKEN

  const DRAFT = process.argv.includes('--draft')

  if (!DRAFT) {
    throw new Error('IS NOT DRAFT RELEASE!')
  }

  if (!NETLIFY_ACCESS_TOKEN) {
    throw new Error('Required environment variable NETLIFY_ACCESS_TOKEN not found')
  }

  if (!fs.existsSync(path.join(PATH, 'index.html'))) {
    throw new Error(
      `Unable to find file ${path.join(PATH, 'index.html')}; are you sure the site has been built?`,
    )
  }

  await deployDir(SITE_ID, PATH, NETLIFY_ACCESS_TOKEN, DRAFT, 'testing release2')
})()
