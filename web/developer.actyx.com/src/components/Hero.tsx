import React from 'react'
import styled from 'styled-components'
import { keyframes } from 'styled-components'
import SearchBarHomePage from '../theme/SearchBar-Homepage'

const Wrapper = styled.div<{
  img: string
}>`
  margin-left: auto;
  margin-right: auto;
  width: 100%;
  padding-top: 60px;
  padding-bottom: 60px;
  background-color: #ffffff;
  opacity: 1;
  background: url(${(props) => props.img}) no-repeat center;
  background-size: 1240px;
`

const waveAnimation = keyframes`
  0% {
    transform: rotate(0deg);
  }
  10% {
    transform: rotate(14deg);
  }
  20% {
    transform: rotate(-8deg);
  }
  30% {
    transform: rotate(14deg);
  }
  40% {
    transform: rotate(-4deg);
  }
  50% {
    transform: rotate(10deg);
  }
  60% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(0deg);
  }
`

const LineWrapper = styled.div`
  display: flex;
  flex-direction: row;
  justify-content: center;
`

const WaveAnimation = styled.div`
  width: 60px;
  font-size: 26px;
  position: relative;
  margin-left: 4px;
  animation-name: ${waveAnimation};
  animation-duration: 4s;
  animation-iteration-count: infinite;
`

const HeroHeadline = styled.div`
  max-width: 650px;
  font-size: 26px;
  font-weight: 900;
  text-align: center;
  background: white;
`

const HeroCopy = styled.div`
  font-size: 15px;
  max-width: 450px;
  text-align: center;
  margin-top: 16px;
  margin-bottom: 24px;
  background: white;
`
const ButtonWrapper = styled.div`
  margin-left: auto;
  margin-right: auto;
  margin-top: 32px;
  display: flex;
  max-width: 800px;
  flex-direction: row;
  justify-content: center;
`
const Button = styled.div<{
  color: string
}>`
  border-radius: 4px;
  border: 1px solid
    ${(p) =>
      p.color == 'green'
        ? '#15BE53'
        : p.color == 'blue'
        ? '#369AFF'
        : p.color == 'purple'
        ? '#635BFF'
        : p.color == 'orange'
        ? '#FF9933'
        : '#f5f5f5'};
  font-weight: 600;
  font-size: 15px;
  padding: 8px;
  color: ${(p) =>
    p.color == 'green'
      ? '#15BE53'
      : p.color == 'blue'
      ? '#369AFF'
      : p.color == 'purple'
      ? '#635BFF'
      : p.color == 'orange'
      ? '#FF9933'
      : '#f5f5f5'};
  margin-left: 16px;
  margin-right: 16px;
  cursor: pointer;
`

type Props = Readonly<{
  img: string
}>

export const Hero: React.FC<Props> = ({ img }: Props) => (
  <Wrapper img={img}>
    <LineWrapper>
      <HeroHeadline>Hey there!</HeroHeadline>
      <WaveAnimation>👋</WaveAnimation>
    </LineWrapper>
    <LineWrapper>
      <HeroHeadline>Welcome to the Actyx developer documentation.</HeroHeadline>
    </LineWrapper>
    <LineWrapper>
      <HeroCopy>
        The place where you find everything you need to digitize factory processes and build
        <a href="https://local-first-cooperation.github.io/website/"> local-first </a> solutions on
        the Actyx platform.
      </HeroCopy>
    </LineWrapper>
    <LineWrapper>
      <SearchBarHomePage />
    </LineWrapper>
    <ButtonWrapper>
      <a style={{ textDecoration: 'none' }} href="docs/tutorials/quickstart">
        <Button color="green">Ready to dive in? Check out our Quick Start</Button>
      </a>
      <a style={{ textDecoration: 'none' }} href="docs/conceptual-guides/how-actyx-works">
        <Button color="blue">New to Actyx? See how everything works</Button>
      </a>
    </ButtonWrapper>
  </Wrapper>
)
