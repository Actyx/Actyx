const NetlifyClient = require('netlify')
const fs = require('fs')
const path = require('path')

const log = console.log

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

  if (process.argv.length < 4) {
    console.error('*******************************************************************************')
    console.error('** Invalid usage of deploy script. If you are using `npm run deploy:prod` or **')
    console.error('** `npm run deploy:draft`, add a name as shown in the following example:     **')
    console.error('** `npm run deploy:draft -- "My draft deployment"`                           **')
    console.error('*******************************************************************************')
    throw new Error('Usage deploy.js <deploy-type> <deploy-name>')
  }
  DEPLOY_TYPE = process.argv[2]
  DEPLOY_NAME = process.argv[3]
  console.log(`Deploy type: ${DEPLOY_TYPE}`)
  console.log(`Deploy name: ${DEPLOY_NAME}`)
  if (!['draft', 'prod'].includes(DEPLOY_TYPE)) {
    throw new Error("<deploy-type> must be one of 'draft' or 'prod'")
  }
  if (!DEPLOY_NAME) {
    throw new Error('<deploy-name> is not set or empty')
  }

  const NETLIFY_ACCESS_TOKEN = process.env.NETLIFY_ACCESS_TOKEN

  if (!NETLIFY_ACCESS_TOKEN) {
    console.error('Required environment variable NETLIFY_ACCESS_TOKEN not found')
    process.exit(1)
  }

  if (!fs.existsSync(path.join(__dirname, '..', PATH, 'index.html'))) {
    console.error(
      `Unable to find file ${path.join(PATH, 'index.html')}; are you sure the site has been built?`,
    )
    process.exit(1)
  }

  try {
    await deployDir(SITE_ID, PATH, NETLIFY_ACCESS_TOKEN, !(DEPLOY_TYPE === 'prod'), DEPLOY_NAME)
  } catch (error) {
    console.error('Error deploying directory')
    console.error(error)
    process.exit(1)
  }
})()
