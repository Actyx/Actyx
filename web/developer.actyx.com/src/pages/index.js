import React from 'react'
import Layout from '@theme/Layout'
import styled from 'styled-components'
import useBaseUrl from '@docusaurus/useBaseUrl'

/* eslint-disable */
const PageWrapper = styled.div`
  max-width: 1200px;
  height: 50vh;
  background-color: #f4f5f7;
  margin-top: 40px;
  margin-left: auto;
  margin-right: auto;
`

function Home() {
  return (
    <PageWrapper>
    </PageWrapper>
  )
}

function HomePage() {
  return (
    <Layout title="Actyx Developer Documentation" description="">
      <Home />
    </Layout>
  )
}

export default HomePage
