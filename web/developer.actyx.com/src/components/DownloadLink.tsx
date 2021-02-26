import React from 'react'
import styled from 'styled-components'
import defaults from '../components/defaults'

const Link = styled.div`
  font-family: 'system-ui', '-apple-system', 'Helvetica', sans-serif;
  font-size: ${defaults.fontsizes.body};
  font-weight: ${defaults.fontWeights.regular};
  color: ${defaults.colors.primary};
  margin-top: 7px;
  margin-bottom: 7px;
  a {
    text-decoration: none;
  }
`

const ArrowStyled = styled.span`
  svg {
    path {
      fill: ${defaults.colors.primary};
    }
    height: 20px;
    transform: translateY(3px);
    margin-right: 10px;
  }
`

const Arrow = () => (
  <ArrowStyled>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 30 408 408">
      <g>
        <path
          className="st0"
          d="M367.7,173.6H214v123.9l2.3-5.2c1.7-3.2,3.8-6.1,6.3-8.6l47.8-47.6c6-6.3,15.6-7.3,22.9-2.5
      c0.6,0.4,1.1,0.9,1.7,1.5c6.8,6.7,6.8,17.7,0.1,24.4l-86.3,86.3c-6.7,6.7-17.7,6.7-24.4,0l-86.3-86.3c-0.5-0.6-1.1-1.1-1.5-1.8
      c-5.7-7.7-4-18.5,3.7-24.1c7.2-4.9,16.9-3.8,22.9,2.5l47.7,47.7c2.9,2.9,5.2,6.2,6.9,9.8l1.7,3.7V173.6H40.3
      C18,173.8,0,191.8,0,214.1c0,0.2,0,0.5,0,0.7l23.4,174.6c0.3,22.2,18.3,40,40.5,40h280.4c22.2,0,40.2-17.8,40.5-40L408,214.8
      c0-0.2,0-0.5,0-0.7C408,191.8,390,173.8,367.7,173.6z"
        />
        <path
          className="st0"
          d="M337.9,105.4H206.3l-26-30.9c-1-1.2-2.5-1.8-4-1.8H70.2c-16.6,0-30,13.4-30,30v28.8h10h129.2H214h153.6
      C365.6,116.6,352.9,105.5,337.9,105.4z"
        />
      </g>
    </svg>
  </ArrowStyled>
)

type Props = Readonly<{
  link: string
  cta: string
}>

export const DownloadLink: React.FC<Props> = ({ link, cta }: Props) => (
  <Link>
    <a href={link}>
      <Arrow />
      {cta}
    </a>
  </Link>
)
