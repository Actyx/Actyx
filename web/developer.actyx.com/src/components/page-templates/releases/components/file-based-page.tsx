import React from 'react'
import Layout from '@theme/Layout'
import styles from '../index.module.css'
import { Product, Version, Change, Hash } from '../types'
import { useHistory } from 'react-router-dom'
import { versionIsNewest, newestVersion } from '../util'
import Link from '@docusaurus/Link'
import { Download } from '../types'

const Row = ({ children, className = '' }) => <div className={`row ${className}`}>{children}</div>
const Col = ({ width, children, className = '' }) => (
  <div className={`col col--${width} ${className}`}>{children}</div>
)

const VersionSelector: React.FC<{
  version: Version
  otherVersions: Version[]
  onSelectVersion: (version: Version) => void
}> = ({ version, otherVersions, onSelectVersion }) => (
  <div className={styles.version}>
    <select
      name="version"
      id="version"
      value={version}
      onChange={(e) => onSelectVersion(e.target.value)}
    >
      {[version].concat(otherVersions).map((version) => (
        <option key={version} value={version}>
          {version}
        </option>
      ))}
    </select>
  </div>
)

export const Page: React.FC<{
  version: Version
  commit: Hash
  changes: Change[]
  otherVersions: Version[]
  product: Product
  productDisplayName: string
  downloads: Download[] | ((version: Version) => Download[])
}> = ({ version, commit, changes, otherVersions, product, productDisplayName, downloads }) => {
  const history = useHistory()
  const allVersions = [version].concat(otherVersions)

  const downloads_: Download[] = typeof downloads === 'function' ? downloads(version) : downloads

  const selectedOtherVersion = (version: Version) => {
    history.push(`/releases/${product}/${version}`)
  }

  const Download: React.FC<{
    platform: string
    ext: string
    files: { arch: string; target: string }[]
  }> = ({ platform, ext, files }) => (
    <li className={styles.download}>
      {platform} <code>{ext}</code>:{' '}
      {files.map(({ arch, target }, i) => (
        <React.Fragment key={arch + target + i}>
          {i !== 0 && ', '}
          <a className="" href={target.replace('$V', version).replace('$C', commit)}>
            {arch}
          </a>
        </React.Fragment>
      ))}
    </li>
  )

  return (
    <Layout title={`${productDisplayName} Releases`} description="">
      <div className={'container ' + styles.root}>
        <Row>
          <Col width="12">
            <p className={styles.subtitle}>Release</p>
            <h1 className={styles.heading}>
              {productDisplayName} {version}
            </h1>
            <p className={styles.commit}>
              <code>{commit.substr(0, 8)}</code>
            </p>
            {!versionIsNewest(version, otherVersions) && (
              <div className={styles.alert}>
                This is an outdated version. Outdated versions maybe contain bugs. It is recommended
                to use the{' '}
                <Link to={`/releases/${product}/${newestVersion(allVersions)}`}>
                  latest version ({newestVersion(allVersions)})
                </Link>
                .
              </div>
            )}
            {otherVersions.length > 0 && (
              <VersionSelector
                version={version}
                otherVersions={otherVersions}
                onSelectVersion={selectedOtherVersion}
              />
            )}
          </Col>
        </Row>
        <Row>
          <Col width="6">
            <h2>Release notes</h2>
            {changes.length < 1 ? (
              <p>None</p>
            ) : (
              <ul>
                {changes.map((change) => (
                  <li key={change}>{change}</li>
                ))}
              </ul>
            )}
          </Col>
          <Col width="6">
            <h2>Downloads</h2>
            <ul>
              {downloads_.map(({ platform, ext, files }) => (
                <Download key={platform} platform={platform} ext={ext} files={files} />
              ))}
            </ul>
          </Col>
        </Row>
      </div>
    </Layout>
  )
}
