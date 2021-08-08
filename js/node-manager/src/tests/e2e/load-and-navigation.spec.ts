import { sleep } from '../../common/util'
import { app } from '../util'

describe('App load and navigation', () => {
  beforeAll(async () => {
    await app.start()
  })

  afterAll(async () => {
    await app.stop()
  })

  it('opens a window', async () => {
    expect(await app.client.getWindowCount()).toBe(1)
  })
  it('loads the correct initial screen', async () => {
    const title = await app.client.$('#Layout_Title')
    expect(title).toBeTruthy()
    expect(await title.getText()).toBe('Nodes')
  })
  it('can load About screen', async () => {
    const button = await app.client.$('button*=About')
    expect(button).toBeTruthy()
    await button.click()
    const title = await app.client.$('#Layout_Title')
    expect(title).toBeTruthy()
    expect(await title.getText()).toBe('About')
  })
  it('can load NodeAuth screen', async () => {
    const button = await app.client.$('button*=Node Auth')
    expect(button).toBeTruthy()
    await button.click()
    const title = await app.client.$('#Layout_Title')
    expect(title).toBeTruthy()
    expect(await title.getText()).toBe('Node Authentication')
  })
  it('can load AppSigning screen', async () => {
    const button = await app.client.$('button*=App Signing')
    expect(button).toBeTruthy()
    await button.click()
    const title = await app.client.$('#Layout_Title')
    expect(title).toBeTruthy()
    expect(await title.getText()).toBe('App Signing')
  })
  it('can load Diagnostics screen', async () => {
    const button = await app.client.$('button*=Diagnostics')
    expect(button).toBeTruthy()
    await button.click()
    const title = await app.client.$('#Layout_Title')
    expect(title).toBeTruthy()
    expect(await title.getText()).toBe('Diagnostics')
  })
})
