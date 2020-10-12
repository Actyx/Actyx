import React from "react";
import styled from "styled-components";
import defaults from "../components/defaults";

const Wrapper = styled.div`
  width: 100%;
  display: flex;
  flex-grow: 1;
  flex-direction: row;
  align-items: flex-start;
  margin-bottom: 30px;
  margin-top: 40px;
  @media (max-width: ${defaults.viewport.medium}px) {
    flex-direction: column;
    width: 100%;
    height: auto;
  }
`;

const ColLeft = styled.div`
  width: 33%;
  height: 100%;
  display: flex;
  flex-direction: column;
  a {
    text-decoration: none;
  }
  @media (max-width: ${defaults.viewport.medium}px) {
    width: 100%;
    margin-bottom: 30px;
    padding-right: 0;
  }
`;

const ColMiddle = styled.div`
  width: 33%;
  height: 100%;
  display: flex;
  flex-direction: column;
  margin-left: 20px;
  margin-right: 20px;
  a {
    text-decoration: none;
  }
  @media (max-width: ${defaults.viewport.medium}px) {
    width: 100%;
    margin-bottom: 30px;
    margin-right: 0;
    margin-left: 0;
  }
`;

const ColRight = styled.div`
  height: 100%;
  width: 33%;
  display: flex;
  flex-direction: column;
  a {
    text-decoration: none;
  }
  @media (max-width: ${defaults.viewport.medium}px) {
    padding-left: 0;
    border-left: none;
    width: 100%;
  }
`;

const ImageDiv = styled.div`
  width: 100%;
  height: 30px;
  border-radius: 0.4rem;
  background-color: #fff;
  overflow: hidden;
  img {
    height: 30px;
  }
`;

const Headline = styled.div`
  font-family: "system-ui", "-apple-system", "Helvetica", sans-serif;
  font-size: 17px;
  line-height: 1.3;
  color: #1998ff;
  font-weight: 500;
  margin-bottom: 10px;
  margin-top: 20px;
`;

const Subheadline = styled.div`
  font-family: "system-ui", "-apple-system", "Helvetica", sans-serif;
  font-size: 14px;
  color: #4e566d;
  margin-bottom: 20px;
`;

const Link = styled.div<{
  showLinks?: boolean;
}>`
  display: ${(p) => (p.showLinks ? "block" : "none")};
  font-family: "system-ui", "-apple-system", "Helvetica", sans-serif;
  font-size: 14px;
  font-weight: 500;
  color: #1998ff;
  text-decoration: none;
`;

const ArrowStyled = styled.span`
  svg {
    path {
      fill: #1998ff;
    }
    height: 15px;
    margin-left: 10px;
  }
`;

const Arrow = () => (
  <ArrowStyled>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -150 447 550">
      <path d="M26.9 255c1.8.3 3.7.4 5.5.4h315.5l-6.9 3.2c-6.7 3.2-12.8 7.5-18.1 12.8l-88.5 88.5c-11.7 11.1-13.6 29-4.6 42.4 10.4 14.3 30.5 17.4 44.7 6.9 1.2-.8 2.2-1.8 3.3-2.8l160-160c12.5-12.5 12.5-32.8 0-45.3l-160-160c-12.5-12.5-32.8-12.5-45.3.1-1 1-1.9 2-2.7 3.1-9 13.4-7 31.3 4.6 42.4l88.3 88.6c4.7 4.7 10.1 8.6 16 11.7l9.6 4.3H34.2c-16.3-.6-30.7 10.8-33.8 26.9-2.8 17.5 9 34 26.5 36.8z" />
    </svg>
  </ArrowStyled>
);

type Props = Readonly<{
  img1: string;
  img2: string;
  img3: string;
  title1: string;
  title2: string;
  title3: string;
  body1: string;
  body2: string;
  body3: string;
  cta1?: string;
  cta2?: string;
  cta3?: string;
  link1?: string;
  link2?: string;
  link3?: string;
  showLinks?: boolean;
}>;

export const ThreeElementRow = ({
  img1,
  img2,
  img3,
  title1,
  title2,
  title3,
  body1,
  body2,
  body3,
  cta1,
  cta2,
  cta3,
  link1,
  link2,
  link3,
  showLinks,
}: Props) => (
  <Wrapper>
    <ColLeft>
      <ImageDiv>
        <img src={img1} />
      </ImageDiv>
      <Headline>{title1}</Headline>
      <Subheadline>{body1}</Subheadline>
      <a href={link1}>
        <Link showLinks={showLinks}>
          {cta1}
          <Arrow />
        </Link>
      </a>
    </ColLeft>
    <ColMiddle>
      <ImageDiv>
        <img src={img2} />
      </ImageDiv>
      <Headline>{title2}</Headline>
      <Subheadline>{body2}</Subheadline>
      <a href={link2}>
        <Link showLinks={showLinks}>
          {cta2}
          <Arrow />
        </Link>
      </a>
    </ColMiddle>
    <ColRight>
      <ImageDiv>
        <img src={img3} />
      </ImageDiv>
      <Headline>{title3}</Headline>
      <Subheadline>{body3}</Subheadline>
      <a href={link3}>
        <Link showLinks={showLinks}>
          {cta3}
          <Arrow />
        </Link>
      </a>
    </ColRight>
  </Wrapper>
);
