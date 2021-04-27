import { promises as fs } from 'fs'
import { pathExists } from 'fs-extra'
import { assertOK } from '../../assertOK'
import path from 'path'
import { settings } from '../../infrastructure/settings'
import { mkAx } from '../../stubs'
import { CLI } from '../../cli'

const FILE_PATH = path.resolve(settings().tempDir, 'temp-swarm-key')

const isBase64 = (data: string) => Buffer.from(data, 'base64').toString('base64') === data
const isLen44 = (data: string) => data.length === 44
const isKeyValid = (key?: string) => key && isBase64(key) && isLen44(key)

describe('ax swarms', () => {
  let ax: CLI

  beforeAll(async () => {
    ax = await mkAx()
  })
  describe('keygen', () => {
    test('return valid swarmKeys (44 length and base64)', async () => {
      const response = assertOK(await ax.swarms.keyGen())
      const key = response.result.swarmKey
      expect(response.result).toMatchObject({
        swarmKey: expect.any(String),
        outputPath: null,
      })
      expect(isKeyValid(key)).toBeTruthy()
    })

    test('return a unique valid swarmKeys', async () => {
      const response1 = assertOK(await ax.swarms.keyGen())
      const response2 = assertOK(await ax.swarms.keyGen())
      const key1 = response1.result.swarmKey
      const key2 = response2.result.swarmKey
      expect(key1).not.toBe(key2)
    })

    test('create a file with a valid swarmKey', async () => {
      const fileExists = await pathExists(FILE_PATH)
      if (fileExists) {
        await fs.unlink(FILE_PATH)
      }
      assertOK(await ax.swarms.keyGen(FILE_PATH))
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

      assertOK(await ax.swarms.keyGen(file1))
      assertOK(await ax.swarms.keyGen(file2))

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
      const response1 = await ax.swarms.keyGen(FILE_PATH)
      const response2 = await ax.swarms.keyGen(FILE_PATH)
      expect(response1).toMatchCodeOk()
      expect(response2).toMatchErrInvalidInput()

      await fs.unlink(FILE_PATH)
    })
  })
})
