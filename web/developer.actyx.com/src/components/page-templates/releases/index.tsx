import React from 'react'
import Layout from '@theme/Layout'
import styles from './index.module.css'
import { Product, ReleaseHistory, Release } from './types'
import { useState } from 'react'

const Row = ({ children, className = '' }) => <div className={`row ${className}`}>{children}</div>
const Col = ({ width, children, className = '' }) => (
  <div className={`col col--${width} ${className}`}>{children}</div>
)

const ReleasesList: React.FC<{
  product: Product
  productDisplayName: string
  releases: Release[]
}> = ({ product, productDisplayName, releases: allReleases }) => {
  const [showAll, setShowAll] = useState(false)
  const NUM_SHOW_INITIAL = 3
  const releases = showAll
    ? allReleases
    : allReleases.length > NUM_SHOW_INITIAL
    ? allReleases.slice(0, NUM_SHOW_INITIAL)
    : allReleases

  return (
    <Row>
      <Col width="12">
        <h2>{productDisplayName}</h2>
        <ul>
          {releases.map(({ version, time, changes }, i) => (
            <li key={product + version}>
              <a href={`/releases/${product}/${version}`}>
                {productDisplayName} {version}
              </a>
              {i === 0 && ' (latest)'}
              <span style={{ paddingLeft: '8px', color: 'gray', fontSize: 'small' }}>{time}</span>
              {changes.map((c) => (
                <div style={{ color: 'gray' }}>
                  <i>{c}</i>
                </div>
              ))}
            </li>
          ))}
          {!showAll && allReleases.length > NUM_SHOW_INITIAL && (
            <li className={styles.showMore}>
              <span onClick={() => setShowAll(true)}>
                Show {allReleases.length - releases.length} more release
                {allReleases.length - releases.length > 1 ? 's' : ''} ...
              </span>
            </li>
          )}
        </ul>
      </Col>
    </Row>
  )
}

const Page: React.FC<{ data: ReleaseHistory }> = ({ data: history }) => {
  console.log(`history:`)
  console.log(history)

  return (
    <Layout title="Actyx Releases" description="">
      <div className={'container ' + styles.root}>
        <Row>
          <Col width="12">
            <h1 className={styles.heading}>Releases</h1>
          </Col>
        </Row>
        <ReleasesList productDisplayName="Actyx" product="actyx" releases={history.actyx} />
        <ReleasesList
          productDisplayName="Actyx Node Manager"
          product="node-manager"
          releases={history['node-manager']}
        />
        <ReleasesList productDisplayName="Actyx CLI" product="cli" releases={history.cli} />
        <ReleasesList productDisplayName="Actyx Pond" product="pond" releases={history.pond} />
      </div>
    </Layout>
  )
}

export default Page
