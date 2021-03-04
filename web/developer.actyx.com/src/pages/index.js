import React from 'react'
import Layout from '@theme/Layout'
import styled from 'styled-components'
import { PageTag } from '../components/PageTag'
import { APIReference } from '../components/APIReference'

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
      <APIReference>```kdfjsndfv```</APIReference>
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
