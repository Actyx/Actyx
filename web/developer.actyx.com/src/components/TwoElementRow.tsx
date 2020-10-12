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
  width: 50%;
  height: 100%;
  display: flex;
  flex-direction: column;
  padding-right: 35px;
  a {
    text-decoration: none;
  }
  @media (max-width: ${defaults.viewport.medium}px) {
    width: 100%;
    margin-bottom: 30px;
    padding-right: 0;
  }
`;

const ColRight = styled.div`
  border-left: 1px solid #eee;
  height: 100%;
  width: 50%;
  display: flex;
  flex-direction: column;
  padding-left: 35px;
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
  cursor: pointer;
  width: 100%;
  border-radius: 0.4rem;
  overflow: hidden;
  display: flex;
  justify-content: center;
  align-items: center;
  &:hover {
    box-shadow: 0 5px 20px rgba(50, 50, 93, 0.35);
    transform: translateY(-3px);
    transition: box-shadow 150ms ease-in-out;
    transition-property: box-shadow;
    transition-duration: 150ms;
    transition-timing-function: ease-in-out;
    transition-delay: 0s;
  }
`;

const Headline = styled.div`
  font-family: "system-ui", "-apple-system", "Helvetica", sans-serif;
  font-size: ${defaults.fontsizes.title};
  font-weight: ${defaults.fontWeights.regular};
  line-height: 1.3;
  margin-bottom: 10px;
  margin-top: 20px;
`;

const Subheadline = styled.div`
  font-family: "system-ui", "-apple-system", "Helvetica", sans-serif;
  font-size: ${defaults.fontsizes.body};
  color: #4e566d;
  margin-bottom: 20px;
`;

const Link = styled.div`
  font-family: "system-ui", "-apple-system", "Helvetica", sans-serif;
  font-size: ${defaults.fontsizes.body};
  font-weight: ${defaults.fontWeights.regular};
  color: ${defaults.colors.primary};
  text-decoration: none;
  margin-bottom: 5px;
`;

const ArrowStyled = styled.span`
  svg {
    path {
      fill: ${defaults.colors.primary};
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

const BadgeWrapper = styled.div<{
  showBadges?: boolean;
}>`
  display: ${(p) => (p.showBadges ? "flex" : "none")};
  flex-direction: row;
  padding-top: 15px;
  margin-top: 0px;
`;

const Badge = styled.div`
  font-family: "system-ui", "-apple-system", "Helvetica", sans-serif;
  font-size: 10px;
  font-weight: 700;
  display: inline-block;
  border-radius: 20px;
  background-color: #d1dde8;
  color: #fff;
  text-transform: uppercase;
  padding-right: 12px;
  padding-left: 12px;
  line-height: 20px;
  margin-right: 10px;
`;

type Props = Readonly<{
  img1: string;
  img2?: string;
  title1: string;
  title2?: string;
  body1: string;
  body2?: string;
  cta1: string[];
  cta2?: string[];
  link1?: string[];
  link2?: string[];
  badges1?: string[];
  badges2?: string[];
  showBadges?: boolean;
}>;

export const TwoElementRow = ({
  img1,
  img2,
  title1,
  title2,
  body1,
  body2,
  cta1,
  cta2,
  link1,
  link2,
  badges1,
  badges2,
  showBadges,
}: Props) => (
  <Wrapper>
    <ColLeft>
      <a href={link1[0]}>
        <ImageDiv>
          <img src={img1} />
        </ImageDiv>
      </a>
      <Headline>{title1}</Headline>
      <Subheadline>{body1}</Subheadline>
      {link1?.map((u, i) => (
        <Link key={i}>
          <a href={link1[i]}>
            {cta1[i]}
            <Arrow />
          </a>
        </Link>
      ))}
      <BadgeWrapper showBadges={showBadges}>
        {badges1?.map((u, i) => (
          <Badge key={i}>{badges1[i]}</Badge>
        ))}
      </BadgeWrapper>
    </ColLeft>
    <ColRight>
      <a href={link2[0]}>
        <ImageDiv>
          <img src={img2} />
        </ImageDiv>
      </a>
      <Headline>{title2}</Headline>
      <Subheadline>{body2}</Subheadline>
      {link2?.map((u, i) => (
        <Link key={i}>
          <a href={link2[i]}>
            {cta2[i]}
            <Arrow />
          </a>
        </Link>
      ))}
      <BadgeWrapper showBadges={showBadges}>
        {badges2?.map((u, i) => (
          <Badge key={i}>{badges2[i]}</Badge>
        ))}
      </BadgeWrapper>
    </ColRight>
  </Wrapper>
);
