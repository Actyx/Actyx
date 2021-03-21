import React from 'react'
import styled from 'styled-components'
import howToGuides from '../contents/how-to-guides'
import { Link } from '../components/Link'

const Wrapper = styled.div`
  display: flex;
  flex-direction: column;
`

const HowToGuideWrapper = styled.div`
  display: flex;
  flex-direction: column;
  width: 100%;
  padding-left: 20px;
  padding-right: 20px;
  padding-top: 12px;
  padding-bottom: 20px;
  border-radius: 4px;
  border: 1px solid #e1e5e8;
  margin-bottom: 24px;
`

const Headline = styled.div`
  font-size: 24px;
`

const Description = styled.div`
  font-size: 15px;
`
const LinkWrapper = styled.div`
  display: flex;
  flex-direction: column;
  margin-top: 24px;
`

type Props = Readonly<{
  guides: string
}>

export const HowToGuides: React.FC<Props> = ({ guides }: Props) => (
  <Wrapper>
    {howToGuides?.map((v, index) => (
      <HowToGuideWrapper>
        <Headline>{howToGuides[index].category}</Headline>
        <Description>{howToGuides[index].description}</Description>
        <LinkWrapper>
          {howToGuides[index]?.contents.map((u, idx) => (
            <Link
              title={howToGuides[index].contents[idx].title}
              link={howToGuides[index].contents[idx].link}
            />
          ))}
        </LinkWrapper>
      </HowToGuideWrapper>
    ))}
  </Wrapper>
)
