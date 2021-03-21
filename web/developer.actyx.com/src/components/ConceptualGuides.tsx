import React from 'react'
import styled from 'styled-components'
import conceptualGuides from '../contents/conceptual-guides'
import { Link } from '../components/Link'

const Wrapper = styled.div`
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
`

const ConceptualGuideWrapper = styled.div`
  display: flex;
  flex-direction: column;
  width: 45%;
  padding-left: 20px;
  padding-right: 20px;
  padding-top: 12px;
  padding-bottom: 20px;
  margin-right: 24px;
  border-radius: 4px;
  border: 1px solid #e1e5e8;
  margin-bottom: 24px;
  justify-content: space-between;
`

const Headline = styled.div`
  font-size: 24px;
`

const Description = styled.div`
  font-size: 15px;
  margin-bottom: 24px;
  float: bottom;
  align-self: flex-end;
`

const Body = styled.div`
  align-items: flex-end;
  align-self: flex-end;
`

type Props = Readonly<{
  guides: string
}>

export const ConceptualGuides: React.FC<Props> = ({ guides }: Props) => (
  <Wrapper>
    {conceptualGuides?.map((v, index) => (
      <ConceptualGuideWrapper>
        <Headline>{conceptualGuides[index].title}</Headline>
        <Body>
          <Description>{conceptualGuides[index].description}</Description>
          <Link link={conceptualGuides[index].link} title="Find out more" />
        </Body>
      </ConceptualGuideWrapper>
    ))}
  </Wrapper>
)
