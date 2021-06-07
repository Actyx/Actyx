import React from 'react'
import { Product, Version, Change, Hash, Download } from './types'
import { Page as PackagerBasedPage } from './components/packager-based-page'
import { versionIsNewest } from './util';

const Page: React.FC<{
  data: {
    version: Version
    commit: Hash
    changes: Change[]
    otherVersions: Version[]
  }
}> = ({ data }) => {
  const { version, otherVersions } = data;
  const isLatest = versionIsNewest(version, otherVersions)
  return (
    <PackagerBasedPage {...data} product="pond" productDisplayName="Actyx Pond" installation={
      <>
        <p>Install using npm:</p>
        <p><code>npm i @actyx/pond{!isLatest && `@${version}`}</code></p>
        <p>Alternatively, view package on <a href={`https://www.npmjs.com/package/@actyx/pond/v/${version}`}>npm.js</a></p>
      </>
    } />
  )
}

export default Page
