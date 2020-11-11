import { runOnEach } from '../runner/hosts'
import { promises as fs } from 'fs'
import { Reponse_Swarms_Keygen } from './types'
import { pathExists } from 'fs-extra'
import { stubNode } from '../stubs'

const FILE_PATH = 'temp-swarm-key'

const getKeyAndOutputFromResponse = (response: Reponse_Swarms_Keygen) =>
  response.code === 'OK'
    ? { swarmKey: response.result.swarmKey, outputPath: response.result.outputPath }
    : undefined

const isBase64 = (data: string) => Buffer.from(data, 'base64').toString('base64') === data
const isLen128 = (data: string) => data.length === 128
const isKeyValid = (key?: string) => key && isBase64(key) && isLen128(key)

describe('ax swarms', () => {
  describe('keygen', () => {
    // FIXME: should use localhocker instead
    test('return status OK for swarm state', async () => {
      const responses = await runOnEach([{}, {}], false, (node) => node.ax.Swarms.State())
      const areStatesValid = responses.every((r) => 'OK' in r)
      expect(areStatesValid).toBe(true)
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
