import React from 'react'
import Layout from '../plugins/analytics/theme/Layout'
import styled from 'styled-components'
import moment from 'moment'
import { toShortHash, flattenToActual, Tags } from '../release-util'
import { BuildNumber } from '../components/BuildNumber'
import { Calendar, Commit, Laptop } from '../icons/icons'

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

const Description = styled.div`
  margin-bottom: 42px;
`
const Link = styled.a`
  color: black;
  text-decoration: underline;
`

const Wrapper = styled.div`
  display: grid;
  grid-template-columns: [first] 220px [line2] auto [end];
  grid-template-rows: auto;
  margin-bottom: 42px;
`

const Left = styled.div`
  grid-column-start: first;
  grid-column-end: line2;
  text-align: right;
  padding-top: 14px;
  padding-right: 16px;
  border-right: 1px solid #e1e5e8;
`

const Right = styled.div`
  grid-column-start: line2;
  grid-column-end: end;
  text-align: left;
  padding-left: 16px;
`

const MetaWrapperLeft = styled.div`
  font-size: 13px;
  color: #586069;
  display: flex;
  flex-direction: row;
  justify-content: flex-end;
  margin-top: 4px;
`

const MetaWrapperRight = styled.div`
  font-size: 15px;
  color: #1998ff;
  display: flex;
  flex-direction: row;
  justify-content: flex-start;
  margin-top: 4px;
`

const Meta = styled.div`
  text-decoration: none;
`

const Headline = styled.div`
  margin-top: 8px;
  font-size: 24px;
  font-weight: 600;
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
            <>
              <React.Fragment key={release.tag}></React.Fragment>
              <Wrapper>
                <Left>
                  <BuildNumber build={release.tag} pre="Release: " />
                  <MetaWrapperLeft>
                    <Calendar />
                    <Meta>{moment(release.meta.at).format('D MMM YYYY [at] HH:mm')}</Meta>
                  </MetaWrapperLeft>
                  <MetaWrapperLeft>
                    <Commit />
                    <Meta>{toShortHash(release.commit)}</Meta>
                  </MetaWrapperLeft>
                </Left>
                <Right>
                  <Headline>Release notes</Headline>
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
                  <Headline>Websites</Headline>
                  {Object.keys(release.meta.sites).length < 1 && <p>None.</p>}
                  {release.meta.sites.length > 0 && (
                    <>
                      {' '}
                      {release.meta.sites.map((site, index) => (
                        <React.Fragment key={release.tag + site.name}>
                          <MetaWrapperRight>
                            <Laptop />
                            <Meta>
                              <Link
                                style={{ color: 'inherit', textDecoration: 'inherit' }}
                                href={site.permalink}
                                target="_blank"
                              >
                                {site.name}
                              </Link>
                            </Meta>
                          </MetaWrapperRight>
                        </React.Fragment>
                      ))}
                    </>
                  )}
                </Right>
              </Wrapper>
            </>
          ))
        )}
      </PageWrapper>
    </Layout>
  )
}

export default ReleasesPage
