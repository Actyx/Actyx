import { promises as fs } from 'fs'
import { Reponse_Swarms_Keygen } from '../../cli/types'
import { pathExists } from 'fs-extra'
import { stubNode } from '../../stubs'
import { SettingsInput } from '../../cli/exec'
import { quickstartDirs } from '../../setup-projects/quickstart'
import { resetTestEviroment } from '../local-docker-util'

const FILE_PATH = 'temp-swarm-key'

const getKeyAndOutputFromResponse = (response: Reponse_Swarms_Keygen) =>
  response.code === 'OK'
    ? { swarmKey: response.result.swarmKey, outputPath: response.result.outputPath }
    : undefined

const isBase64 = (data: string) => Buffer.from(data, 'base64').toString('base64') === data
const isLen128 = (data: string) => data.length === 128
const isKeyValid = (key?: string) => key && isBase64(key) && isLen128(key)

describe('ax swarms', () => {
  beforeAll(async () => {
    await resetTestEviroment()
  })
  afterAll(async () => {
    await resetTestEviroment()
  })

  describe('keygen', () => {
    test('return status OK for swarm state', async () => {
      const scope = 'com.actyx.os'
      await stubNode.ax.Settings.Set(
        scope,
        SettingsInput.FromFile(`${quickstartDirs.quickstart}/misc/local-sample-node-settings.yml`),
      )
      const response = await stubNode.ax.Swarms.State(4457)
      const responseShape = {
        Ok: {
          store: { block_count: expect.any(Number), block_size: expect.any(Number) },
          swarm: {
            listen_addrs: expect.any(Array),
            peer_id: expect.any(String),
            peers: expect.any(Object),
          },
        },
      }
      expect(response).toMatchObject(responseShape)
      await stubNode.ax.Settings.Unset(scope)
    })

    test('return valid swarmKeys (128 length and base64)', async () => {
      const response = await stubNode.ax.Swarms.KeyGen()
      const responseShape = {
        code: 'OK',
        result: {
          swarmKey: expect.any(String),
          outputPath: null,
        },
      }
      const key = getKeyAndOutputFromResponse(response)?.swarmKey
      expect(response).toMatchObject(responseShape)
      expect(isKeyValid(key)).toBeTruthy()
    })

    test('return a unique valid swarmKeys', async () => {
      const response1 = await stubNode.ax.Swarms.KeyGen()
      const response2 = await stubNode.ax.Swarms.KeyGen()
      const key1 = getKeyAndOutputFromResponse(response1)?.swarmKey
      const key2 = getKeyAndOutputFromResponse(response2)?.swarmKey
      expect(key1).not.toBe(key2)
    })

    test('create a file with a valid swarmKey', async () => {
      const fileExists = await pathExists(FILE_PATH)
      if (fileExists) {
        await fs.unlink(FILE_PATH)
      }
      await stubNode.ax.Swarms.KeyGen(FILE_PATH)
      const swarmKeyFile = await fs.readFile(FILE_PATH, 'utf-8')
      expect(isKeyValid(swarmKeyFile)).toBe(true)

      await fs.unlink(FILE_PATH)
    })

    test('create files with a unique valid swarmKey', async () => {
      const file1 = `${FILE_PATH}0`
      const file2 = `${FILE_PATH}1`
      const file1Exists = await pathExists(file1)
      const file2Exists = await pathExists(file2)
      if (file1Exists) {
        await fs.unlink(file1)
      }
      if (file2Exists) {
        await fs.unlink(file2)
      }

      await stubNode.ax.Swarms.KeyGen(file1)
      await stubNode.ax.Swarms.KeyGen(file2)

      const key1 = await fs.readFile(file1, 'utf-8')
      const key2 = await fs.readFile(file2, 'utf-8')
      expect(key1).not.toBe(key2)

      await fs.unlink(file1)
      await fs.unlink(file2)
    })

    test('return ERR_INVALID_INPUT when cannot write a swarm key on existing file', async () => {
      const fileExists = await pathExists(FILE_PATH)
      if (fileExists) {
        await fs.unlink(FILE_PATH)
      }
      const response1 = await stubNode.ax.Swarms.KeyGen(FILE_PATH)
      const response2 = await stubNode.ax.Swarms.KeyGen(FILE_PATH)
      expect(response1).toMatchCodeOk()
      expect(response2).toMatchErrInvalidInput()

      await fs.unlink(FILE_PATH)
    })
  })
})
