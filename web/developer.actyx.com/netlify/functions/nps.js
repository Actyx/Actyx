const https = require('https')

const postResultToSlack = (score, feedback) =>
  new Promise((resolve, reject) => {
    console.log(`posting to slack; score=${score}, feedback=${feedback}`)
    let info = 'Got'
    if (score !== undefined) {
      info += ` a score of ${score}`
    }
    if (feedback !== undefined) {
      info += ` feedback:\n---\n${feedback}\n---`
    }
    const data = JSON.stringify({
      username: 'NPS results from developer.actyx.com',
      text: info,
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
  const { score, feedback } = JSON.parse(event.body)
  try {
    await postResultToSlack(score, feedback)
  } catch (error) {
    console.error(`error posting NPS result (score=${score}, feedback=${feedback}) to slack: ${error}`)
    throw error
  }

  return {
    statusCode: 200,
  }
}
