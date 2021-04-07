import React from 'react'
import styled from 'styled-components'
import defaults from '../old-components/defaults'

const Wrapper = styled.div`
  width: 100%;
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: 30px;
  border-bottom: 1px solid #ddd;
  @media (max-width: ${defaults.viewport.medium}px) {
    flex-direction: column;
    width: 100%;
    height: auto;
  }
`

const LeftCol = styled.div`
  width: 54%;
  display: flex;
  align-items: flex-end;
  padding: 0;
  @media (max-width: ${defaults.viewport.medium}px) {
    width: 100%;
  }
`

const ImageDiv = styled.div`
  overflow: hidden;
  width: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  img {
    padding: 0;
  }
`

const RightCol = styled.div`
  padding-left: 20px;
  padding-bottom: 5px;
  width: 46%;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: flex-start;
  @media (max-width: ${defaults.viewport.medium}px) {
    width: 100%;
    margin-top: 20px;
  }
`

const TextWrapper = styled.div`
  flex-direction: column;
  align-items: flex-end;
  justify-content: flex-start;
`

const Headline = styled.div`
  font-family: 'system-ui', '-apple-system', 'Helvetica', sans-serif;
  font-size: 26px;
  font-weight: 500;
  margin-bottom: 10px;
`

const Subheadline = styled.div`
  font-family: 'system-ui', '-apple-system', 'Helvetica', sans-serif;
  font-size: ${defaults.fontsizes.subheadline};
  color: #4e566d;
  margin-bottom: 32px;
`

const Button = styled.div<{
  button?: boolean
}>`
  display: inline-block;
  cursor: pointer;
  text-align: center;
  border-radius: 3px;
  background-color: #1998ff;
  letter-spacing: 0.02em;
  white-space: nowrap;
  padding-left: ${(p) => (p.button ? '10px' : '0')};
  padding-right: ${(p) => (p.button ? '10px' : '0')};
  padding-top: 8px;
  padding-bottom: 8px;
  margin-bottom: 20px;
  font-size: ${defaults.fontsizes.medium};
  font-weight: 600;
  text-transform: uppercase;
  box-shadow: 0 4px 6px rgba(50, 50, 93, 0.11), 0 1px 3px rgba(0, 0, 0, 0.08);
  color: #fff;
  &:hover {
    transform: translateY(-0.15rem);
    transition: box-shadow 150ms ease-in-out;
    transition-property: box-shadow;
    transition-duration: 150ms;
    transition-timing-function: ease-in-out;
    transition-delay: 0s;
  }
`

type Props = Readonly<{
  img: string
  title: string
  subtitle: string
  cta?: string
  link?: string
  button?: boolean
}>

export const SectionHero: React.FC<Props> = ({
  img,
  title,
  subtitle,
  cta,
  link,
  button,
}: Props) => (
  <Wrapper>
    <LeftCol>
      <ImageDiv>
        <img src={img} />
      </ImageDiv>
    </LeftCol>
    <RightCol>
      <TextWrapper>
        <Headline>{title}</Headline>
        <Subheadline>{subtitle}</Subheadline>
        <a href={link}>
          <Button button={button}>{cta} </Button>
        </a>
      </TextWrapper>
    </RightCol>
  </Wrapper>
)
