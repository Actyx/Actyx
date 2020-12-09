import React from 'react'
import styled from 'styled-components'

const Wrapper = styled.div`
  width: 100%;
  display: flex;
  border-radius: 0.4rem;
  padding-top: 20px;
  padding-bottom: 20px;
  flex-grow: 1;
  flex-direction: row;
  margin-bottom: 30px;
`

const ColLeft = styled.div`
  width: 30px;
  height: 100%;
  margin-left: 10px;
  margin-right: 10px;
  a {
    text-decoration: none;
  }
`

const ColMiddle = styled.div`
  width: 30px;
  height: 100%;
  margin-left: 10px;
  margin-right: 10px;
  a {
    text-decoration: none;
  }
`

const ColRight = styled.div`
  height: 100%;
  width: 30px;
  margin-left: 10px;
  margin-right: 10px;
  a {
    text-decoration: none;
  }
`

const ImageDiv = styled.div`
  width: 100%;
  height: 30px;
  border-radius: 0.4rem;
  background-color: #fff;
  overflow: hidden;
  img {
    height: 30px;
  }
`

type Props = Readonly<{
  img1: string
  img2: string
  img3: string
  img4: string
}>

export const StayInformed = ({ img1, img2, img3, img4 }: Props) => (
  <Wrapper>
    <ColLeft>
      <a href="https://www.twitter.com/actyx">
        <ImageDiv>
          <img src={img1} />
        </ImageDiv>
      </a>
    </ColLeft>
    <ColMiddle>
      <a href="https://github.com/actyx">
        <ImageDiv>
          <img src={img2} />
        </ImageDiv>
      </a>
    </ColMiddle>
    <ColMiddle>
      <a href="https://www.youtube.com/channel/UCRzKy5HoKqfRpM-v3s5fnPw">
        <ImageDiv>
          <img src={img3} />
        </ImageDiv>
      </a>
    </ColMiddle>
    <ColRight>
      <a href="https://www.linkedin.com/company/10114352">
        <ImageDiv>
          <img src={img4} />
        </ImageDiv>
      </a>
    </ColRight>
  </Wrapper>
)
