const https = require('https')

const postResultToSlack = (score) =>
  new Promise((resolve, reject) => {
    const data = JSON.stringify({
      username: 'NPS Scores from developer.actyx.com',
      text: `Received NPS score of ${score}`,
    })
    const SLACK_WEBHOOK_HOSTNAME = 'hooks.slack.com'
    const SLACK_WEBHOOK_PATH = '/services/T04MNN9V9/B020U156ZCZ/bRTB8LEMJcMSpqAen29cU1s7'

    const options = {
      hostname: SLACK_WEBHOOK_HOSTNAME,
      port: 443,
      path: SLACK_WEBHOOK_PATH,
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': data.length,
      },
    }

    const req = https.request(options, (res) => {
      console.log(`posting NPS result to slack returned status code ${res.statusCode}`)
      if (res.statusCode !== 200) {
        reject(
          `got response status code ${res.statusCode} (not 200) from Slack webhook: ${res.statusMessage}`,
        )
      } else {
        resolve()
      }
    })

    req.on('error', (error) => {
      reject(error)
    })

    req.write(data)
    req.end()
  })

exports.handler = async function (event) {
  const { result } = JSON.parse(event.body)
  try {
    await postResultToSlack(result)
  } catch (error) {
    console.error(`error posting NPS result to slack: ${error}`)
    throw error
  }

  return {
    statusCode: 200,
  }
}
