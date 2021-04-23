import React from 'react'
import Layout from '@theme/Layout'
import styled from 'styled-components'
import { Hero } from '../components/Hero'
import { HowPageIsStructured } from '../components/HowPageIsStructured'
import { ActyxEssentials } from '../components/ActyxEssentials'
// import { TypeForm } from '../icons/icons'
import useBaseUrl from '@docusaurus/useBaseUrl'

/* eslint-disable */
const PageWrapper = styled.div`
  max-width: 1200px;
  background-color: #ffffff;
  margin-left: auto;
  margin-right: auto;
`
const customStyle = {
  border: 'none',
  height: '54px',
  position: 'fixed',
  boxShadow: '0px 2px 12px rgba(0, 0, 0, 0.26), 0px 2px 4px rgba(0, 0, 0, 0.28)',
  right: '26px',
  bottom: '26px',
  borderRadius: '5px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  cursor: 'pointer',
  background: '#369AFF',
  overflow: 'hidden',
  lineHeight: 0,
}

function Home() {
  return (
    <>
      <PageWrapper>
        <Hero img={useBaseUrl('/images/homepage/hero-background.svg')} />
        <HowPageIsStructured />
        <ActyxEssentials img={useBaseUrl('/images/homepage/homepage-links.svg')} />
        {/* <script>
          {(function () {
            let js, q
            const d = document,
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
          })()}{' '}
        </script>

        <a
          className="typeform-share button"
          href="https://form.typeform.com/to/dHlLetfi?typeform-medium=embed-snippet"
          data-mode="popover"
          style={customStyle}
          data-hide-headers="true"
          data-hide-footer="true"
          data-submit-close-delay="2"
          target="_blank"
        >
          <span class="icon" style={{ marginRight: '12px' }}>
            <TypeForm color="blue" positive={false} />
          </span>
          Give us feedback
        </a> */}
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
