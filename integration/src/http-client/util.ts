import { mkAuthHttpClient } from './ax-http-client'
import { mkEventService } from './event-service'
import { AppManifest, AxEventService } from './types'

export const trialManifest: AppManifest = {
  appId: 'com.example.my-app',
  displayName: 'My Example App',
  version: '1.0.0',
}

export const mkTrialHttpClient = mkAuthHttpClient(trialManifest)

export const mkESFromTrial = (httpApiOrigin: string): Promise<AxEventService> =>
  mkTrialHttpClient(httpApiOrigin).then(mkEventService)
