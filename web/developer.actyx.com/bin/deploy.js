const NetlifyClient = require("netlify");
const fs = require("fs");
const path = require("path");

const log = console.log;

const logOnProgress = (ev) => {
  log(ev.msg);
};

/**
 * This function deploys a directory to a Netlify site. If draft is true, it
 * will create a deploy preview (Netlify parlance), otherwise it will create
 * a production deploy. The function waits for the deploy to succeed before
 * returning.
 */
const deployDir = async (
  siteId,
  path,
  netlifyAccessToken,
  draft,
  deployMsg
) => {
  const client = new NetlifyClient(netlifyAccessToken);

  log(
    `Starting site deploy (siteId: ${siteId}, path: ${path}, draft: ${draft}).`
  );
  const res = await client.deploy(siteId, path, {
    draft, // draft deploy or production deploy
    message: deployMsg,
    deployTimeout: 1.2e6,
    parallelHash: 100,
    parallelUpload: 5,
    maxRetry: 5,
  });
  log(`Site deploy completed; permalink: ${res.deploy.links.permalink}`);
  if (!res.deploy.links.permalink) {
    throw new Error(
      `Netlify unexpectedly did not return a permalink to the deployed site.`
    );
  }
  log(`{{DEPLOY_PERMALINK=${res.deploy.links.permalink}}}`);
  return;
};

(async () => {
  const SITE_ID = "fffa1022-c8a0-4238-97c4-dd80979bf887";
  const PATH = "build";

  if (process.argv.length < 3) {
    throw new Error("Usage deploy.js <deploy-name> [--not-draft]");
  }
  DEPLOY_NAME = process.argv[2];
  console.log(`Deploy name: ${DEPLOY_NAME}`);
  const NOT_DRAFT =
    process.argv.length > 3 && process.argv[3] === "--not-draft";

  if (NOT_DRAFT) {
    console.error("IS NOT DRAFT RELEASE!");
    process.exit(1);
  }

  const NETLIFY_ACCESS_TOKEN = process.env.NETLIFY_ACCESS_TOKEN;

  if (!NETLIFY_ACCESS_TOKEN) {
    console.error(
      "Required environment variable NETLIFY_ACCESS_TOKEN not found"
    );
    process.exit(1);
  }

  if (!fs.existsSync(path.join(__dirname, "..", PATH, "index.html"))) {
    console.error(
      `Unable to find file ${path.join(
        PATH,
        "index.html"
      )}; are you sure the site has been built?`
    );
    process.exit(1);
  }

  try {
    await deployDir(
      SITE_ID,
      PATH,
      NETLIFY_ACCESS_TOKEN,
      !NOT_DRAFT,
      DEPLOY_NAME
    );
  } catch (error) {
    console.error("Error deploying directory");
    console.error(error);
    process.exit(1);
  }
})();
