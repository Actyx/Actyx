import { sleep } from '../../common/util'
import { app, setTestKeys, removeTestKeys } from '../util'

const RANDOM_NODE_ADDR = 'xxxxxxxx'

describe('diagnostics', () => {
  beforeEach(async () => {
    await app.start()
  })

  afterEach(async () => {
    if (app.isRunning()) {
      await app.stop()
    }
  })

  it('toggle developer tools', async () => {
    expect(await app.client.getWindowCount()).toBe(1)
    await (await app.client.$('button*=Diagnostics')).click()
    await (await app.client.$('li*=Node Manager')).click()
    await (await app.client.$('button*=Toggle Dev Tools')).click()
    expect(await app.client.getWindowCount()).toBe(2)
    await (await app.client.$('button*=Toggle Dev Tools')).click()
    expect(await app.client.getWindowCount()).toBe(2)
  })

  // Can't test this as it seems the app shouldn't be killed (test times out)
  //it('exit app', async () => {
  //  await (await app.client.$('button*=Diagnostics')).click()
  //  await (await app.client.$('li*=Node Manager')).click()
  //  await (await app.client.$('button*=Exit Node Manager')).click()
  //})
})
