import { sleep } from '../../common/util'
import { app, setTestKeys, removeTestKeys } from '../util'

const RANDOM_NODE_ADDR = '192.168.0.1'

describe('user key handling', () => {
  beforeEach(async () => {
    await removeTestKeys()
    await app.start()
  })

  afterEach(async () => {
    await removeTestKeys()
    await app.stop()
  })

  it('recognizes if user key is not available', async () => {
    const input = await app.client.$('<input>')
    await input.addValue(RANDOM_NODE_ADDR + '\uE006')
    await sleep(2500)
    const title = await app.client.$('#Layout_Title')
    expect(await title.getText()).toBe('Setup a user key')
  })

  it('recognizes if user key is available', async () => {
    await setTestKeys()
    const input = await app.client.$('<input>')
    await input.addValue(RANDOM_NODE_ADDR + '\uE006')
    await sleep(2500)
    const title = await app.client.$('#Layout_Title')
    expect(await title.getText()).toBe('Nodes')
  })

  it('is able to create a user key', async () => {
    const input = await app.client.$('<input>')
    await input.addValue(RANDOM_NODE_ADDR + '\uE006')
    await sleep(2000)
    const title = await app.client.$('#Layout_Title')
    expect(await title.getText()).toBe('Setup a user key')

    const button = await app.client.$('button*=Create new user key pair')
    expect(button).toBeTruthy()
    await button.click()
    await sleep(500)

    const button2 = await app.client.$('button*=Ok')
    expect(button2).toBeTruthy()
    await button2.click()
    await sleep(500)

    const title2 = await app.client.$('#Layout_Title')
    expect(await title2.getText()).toBe('Nodes')

    const nodeAddr = await app.client.$('p*=' + RANDOM_NODE_ADDR)
    expect(nodeAddr).toBeTruthy()
    expect(await nodeAddr.getText()).toBe(RANDOM_NODE_ADDR)
    await sleep(500)
  })
})
