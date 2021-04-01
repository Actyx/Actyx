import React from 'react'
import Layout from '@theme/Layout'
import styled from 'styled-components'
import { Hero } from '../components/Hero'
import { HowPageIsStructured } from '../components/HowPageIsStructured'
import { ActyxEssentials } from '../components/ActyxEssentials'
import useBaseUrl from '@docusaurus/useBaseUrl'

/* eslint-disable */
const PageWrapper = styled.div`
  max-width: 1200px;
  background-color: #ffffff;
  margin-left: auto;
  margin-right: auto;
`
/*
typeform button 

const Form = styled.a`
  display: inline-block;
  text-decoration: none;
  background-color: #0445af;
  color: white;
  cursor: pointer;
  font-family: Helvetica, Arial, sans-serif;
  font-size: 20px;
  line-height: 50px;
  text-align: center;
  margin: 0;
  height: 50px;
  padding: 0px 33px;
  border-radius: 25px;
  max-width: 50%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: bold;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
`

<Form
          className="typeform-share button"
          href="https://form.typeform.com/to/bnewc6UP?typeform-medium=embed-snippet"
          data-mode="popup"
          data-size="50"
          target="_blank"
        >
          Launch me{' '}
        </Form>
        <script>
          {' '}
          {(function () {
            var qs,
              js,
              q,
              s,
              d = document,
              gi = d.getElementById,
              ce = d.createElement,
              gt = d.getElementsByTagName,
              id = 'typef_orm_share',
              b = 'https://embed.typeform.com/'
            if (!gi.call(d, id)) {
              js = ce.call(d, 'script')
              js.id = id
              js.src = b + 'embed.js'
              q = gt.call(d, 'script')[0]
              q.parentNode.insertBefore(js, q)
            }
          })()}
        </script>
*/
function Home() {
  return (
    <>
      <Hero img={useBaseUrl('/images/homepage/hero-background.svg')} />
      <PageWrapper>
        <HowPageIsStructured />
        <ActyxEssentials img={useBaseUrl('/images/homepage/homepage-links.svg')} />
      </PageWrapper>
    </>
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
