import React from 'react'
import Layout from '@theme/Layout'
import styled from 'styled-components'
import moment from 'moment'
import { toShortHash, flattenToActual, Tags } from '../release-util'

/* eslint-disable */

const PageWrapper = styled.div`
  max-width: 1140px;
  padding-left: 40px;
  padding-right: 40px;
  margin-left: auto;
  margin-right: auto;
  padding-bottom: 3rem;
  padding-top: 3rem;
  h2 {
    margin-top: 2rem;
  }
`

const Title = styled.h1``
const Description = styled.p`
  margin-top: 1rem;
`
const Link = styled.a`
  color: black;
  text-decoration: underline;
`

function ReleasesPage({ releases }) {
  releases = releases.sort((a, b) => Tags.compare(a.tag, b.tag)).reverse()
  return (
    <Layout title="Releases | Actyx Developer Documentation" description="">
      <PageWrapper>
        <Title>Releases</Title>
        <Description>
          This page lists past Actyx releases with release notes. You can access the documentation
          for any release by clicking the respective link.
        </Description>
        {releases.length < 1 ? (
          <p>No releases.</p>
        ) : (
          releases.map((release) => (
            <React.Fragment key={release.tag}>
              <h2>
                {Tags.isDraftRelease(release.tag) && 'Draft '}Release{' '}
                <span
                  style={{
                    fontFamily: 'monospace',
                    color: '#1998ff',
                    backgroundColor: '#e8e8e8',
                    padding: '5px 7px',
                  }}
                >
                  {release.tag}
                </span>
              </h2>
              <p>
                Released {moment(release.meta.at).format('D MMM YYYY [at] HH:mm')} from{' '}
                {toShortHash(release.commit)}
                {release.meta.sites.length > 0 && (
                  <>
                    {' '}
                    (
                    {release.meta.sites.map((site, index) => (
                      <React.Fragment key={release.tag + site.name}>
                        <Link href={site.permalink} target="_blank">
                          {site.name}
                        </Link>
                        {index === release.meta.sites.length - 1 ? '' : ', '}
                      </React.Fragment>
                    ))}
                    )
                  </>
                )}
                .
              </p>
              <h3>Release notes</h3>
              {Object.keys(release.notes).length < 1 && <p>None.</p>}
              <ul>
                {Object.entries(flattenToActual(release.notes)).map(([thing, notes]) => (
                  <li key={release.tag + thing}>
                    {thing}
                    <ul>
                      {notes.map((note, i) => (
                        <li key={release.tag + thing + i}>{note}</li>
                      ))}
                    </ul>
                  </li>
                ))}
              </ul>
            </React.Fragment>
          ))
        )}
      </PageWrapper>
    </Layout>
  )
}

export default ReleasesPage
