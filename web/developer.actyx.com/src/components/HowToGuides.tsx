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
  padding-top: 40px;
  margin-bottom: 40px;
  border-top: 1px solid #e1e5e8;
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
              color="blue"
              title={howToGuides[index].contents[idx].title}
              link={howToGuides[index].contents[idx].link}
              positive
            />
          ))}
        </LinkWrapper>
      </HowToGuideWrapper>
    ))}
  </Wrapper>
)
