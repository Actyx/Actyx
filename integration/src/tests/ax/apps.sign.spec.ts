import { readFile, copyFile, remove } from 'fs-extra'
import { runOnEvery } from '../../infrastructure/hosts'
import { resolve } from 'path'
import { OS } from '../../../jest/types'

const appManifestPath = resolve('.', 'fixtures/app_manifest.json')
const cpAppManifestPath = resolve('.', 'fixtures/app_manifest_cp.json')
const devCertPath = resolve('.', 'fixtures/dev_cert.json')


// It seems like the error code is different on Windows
const SYS_ERROR_NO_FILE = (os: OS) => {
  switch(os) {
    case 'windows': return 'The system cannot find the path specified. (os error 3)';
    default: return "No such file or directory (os error 2)";
  }
}

describe('ax', () => {
  describe('apps sign', () => {
    it('should fail when no files are provided', async () =>
      runOnEvery(async (node) => {
        const res = await node.ax.apps.sign('', '');
        expect(res.code).toEqual('ERR_IO');
        expect((res as any).message.startsWith('Failed to read developer certificate (')).toBeTruthy();
        expect((res as any).message.endsWith(')')).toBeTruthy();
      }))

    it('should fail when app manifest is not provided', async () =>
      runOnEvery(async (node) => {
        const res = await node.ax.apps.sign(devCertPath, '');
        expect(res.code).toEqual('ERR_IO');
        expect((res as any).message.startsWith('Failed to read app manifest (')).toBeTruthy();
        expect((res as any).message.endsWith(')')).toBeTruthy();
    }))

    it('should fail when dev cert content is malformed', () =>
      runOnEvery((node) =>
        expect(node.ax.apps.sign(appManifestPath, appManifestPath)).resolves.toEqual({
          code: 'ERR_INVALID_INPUT',
          message:
            'Failed to deserialize developer certificate (missing field `devPrivkey` at line 5 column 1)',
        }),
      ))

    it('should fail when app manifest content is malformed', () =>
      runOnEvery((node) =>
        expect(node.ax.apps.sign(devCertPath, devCertPath)).resolves.toEqual({
          code: 'ERR_INVALID_INPUT',
          message: 'Failed to deserialize app manifest (missing field `appId` at line 9 column 1)',
        }),
      ))

    it('should sign manifest', () =>
      runOnEvery(async (node) => {
        const expectedManifest = {
          appId: 'com.actyx.auth-test',
          displayName: 'auth test app',
          version: 'v0.0.1',
          signature:
            'v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w==',
        }
        await copyFile(appManifestPath, cpAppManifestPath)
        await expect(node.ax.apps.sign(devCertPath, cpAppManifestPath)).resolves.toEqual({
          code: 'OK',
          result: expectedManifest,
        })

        const result = await readFile(cpAppManifestPath, 'utf-8')
        expect(JSON.parse(result)).toEqual(expectedManifest)
        await remove(cpAppManifestPath)
      }))
  })
})
