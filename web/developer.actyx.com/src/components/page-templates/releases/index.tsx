import React from 'react'
import Layout from '@theme/Layout'
import styles from './index.module.css'
import { Product, ReleaseHistory, Release } from './types'

const Row = ({ children, className = '' }) => <div className={`row ${className}`}>{children}</div>
const Col = ({ width, children, className = '' }) => (
  <div className={`col col--${width} ${className}`}>{children}</div>
)

const ReleasesList: React.FC<{
  product: Product
  productDisplayName: string
  releases: Release[]
}> = ({ product, productDisplayName, releases }) => (
  <Row>
    <Col width="12">
      <h2>{productDisplayName}</h2>
      <ul>
        {releases.map(({ version }, i) => (
          <li key={product + version}>
            <a href={`/releases/${product}/${version}`}>
              {productDisplayName} {version}
            </a>
            {i === 0 && ' (latest)'}
          </li>
        ))}
      </ul>
    </Col>
  </Row>
)

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
        <ReleasesList productDisplayName="Actyx CLI" product="cli" releases={history.cli} />
        <ReleasesList
          productDisplayName="Actyx Node Manager"
          product="node-manager"
          releases={history['node-manager']}
        />
        <ReleasesList productDisplayName="Actyx Pond" product="pond" releases={history.pond} />
      </div>
    </Layout>
  )
}

export default Page
