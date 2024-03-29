import React from 'react'
import styled from 'styled-components'
import defaults from '../components/defaults'

const IconStyled = styled.span<{
  color: string
  positive: boolean
}>`
  svg {
    path {
      fill: ${(p) =>
        p.color == 'lightgray' && p.positive
          ? defaults.colors.lightgray
          : p.color == 'black' && p.positive
          ? defaults.colors.black
          : p.color == 'green' && p.positive
          ? defaults.colors.green
          : p.color == 'blue' && p.positive
          ? defaults.colors.blue
          : p.color == 'purple' && p.positive
          ? defaults.colors.purple
          : p.color == 'orange' && p.positive
          ? defaults.colors.orange
          : p.color == 'dark' && p.positive
          ? defaults.colors.darkgray
          : p.color == 'white' && p.positive
          ? defaults.colors.darkgray
          : p.color == 'white' && p.positive == false
          ? defaults.colors.darkgray
          : defaults.colors.white};
    }
    height: 21px;
    margin-right: 10px;
    padding-top: 3px;
  }
`

const IconStyledLarge = styled.span<{
  color: string
  positive: boolean
}>`
  svg {
    path {
      fill: ${(p) =>
        p.color == 'lightgray' && p.positive
          ? defaults.colors.lightgray
          : p.color == 'black' && p.positive
          ? defaults.colors.black
          : p.color == 'green' && p.positive
          ? defaults.colors.green
          : p.color == 'blue' && p.positive
          ? defaults.colors.blue
          : p.color == 'purple' && p.positive
          ? defaults.colors.purple
          : p.color == 'orange' && p.positive
          ? defaults.colors.orange
          : p.color == 'dark' && p.positive
          ? defaults.colors.darkgray
          : p.color == 'white' && p.positive
          ? defaults.colors.darkgray
          : p.color == 'white' && p.positive == false
          ? defaults.colors.darkgray
          : defaults.colors.white};
    }
    height: 30px;
  }
`

type Props = Readonly<{
  color: string
  positive: boolean
}>

export const Calendar = ({ color, positive }: Props): React.ReactNode => (
  <IconStyled color={color} positive={positive}>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
      <path d="M197.18,37.76H195.5V10.24a10,10,0,0,0-20,0V37.76h-95V10.24a10,10,0,0,0-20,0V37.76H58.82A58.83,58.83,0,0,0,0,96.59V196.94a58.82,58.82,0,0,0,58.82,58.82H197.18A58.82,58.82,0,0,0,256,196.94V96.59A58.83,58.83,0,0,0,197.18,37.76Zm-138.36,20H60.5v12a10,10,0,0,0,20,0v-12h95v12a10,10,0,0,0,20,0v-12h1.68A38.87,38.87,0,0,1,236,96.59v14.17H20V96.59A38.87,38.87,0,0,1,58.82,57.76Zm138.36,178H58.82A38.86,38.86,0,0,1,20,196.94V130.76H236v66.18A38.86,38.86,0,0,1,197.18,235.76Z" />
    </svg>
  </IconStyled>
)

export const Commit = ({ color, positive }: Props): React.ReactNode => (
  <IconStyled color={color} positive={positive}>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
      <path d="M243,118H187.15a60,60,0,0,0-118.3,0H13a10,10,0,0,0,0,20H68.85a60,60,0,0,0,118.3,0H243a10,10,0,0,0,0-20ZM128,168a40,40,0,1,1,40-40A40,40,0,0,1,128,168Z" />
    </svg>
  </IconStyled>
)

export const Laptop = ({ color, positive }: Props): React.ReactNode => (
  <IconStyled color={color} positive={positive}>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
      <path d="M214.79,58A3.21,3.21,0,0,1,218,61.21V161H38V61.21A3.21,3.21,0,0,1,41.21,58H214.79m0-20H41.21A23.21,23.21,0,0,0,18,61.21V181H238V61.21A23.21,23.21,0,0,0,214.79,38Z" />
      <path d="M256,188H0v12a17,17,0,0,0,17,17H239a17,17,0,0,0,17-17V188Z" />
    </svg>
  </IconStyled>
)

export const Arrow = ({ color, positive }: Props): React.ReactNode => (
  <IconStyled color={color} positive={positive}>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
      <path
        d="M67.2,215.6c-7.1-0.3-12.6-6.3-12.3-13.4c0.1-3.2,1.4-6.2,3.6-8.4l38.5-39.4L58.2,115c-5.3-4.7-5.8-12.8-1.1-18.2
	S70,91,75.4,95.7c0.4,0.3,0.8,0.7,1.1,1.1l47.8,48.7c5,5,5,13,0,18l-48,48.6C73.8,214.4,70.6,215.6,67.2,215.6z"
      />
    </svg>
  </IconStyled>
)

export const GitHub = ({ color, positive }: Props): React.ReactNode => (
  <IconStyled color={color} positive={positive}>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512">
      <path d="M400 32H48C21.5 32 0 53.5 0 80v352c0 26.5 21.5 48 48 48h352c26.5 0 48-21.5 48-48V80c0-26.5-21.5-48-48-48zM277.3 415.7c-8.4 1.5-11.5-3.7-11.5-8 0-5.4.2-33 .2-55.3 0-15.6-5.2-25.5-11.3-30.7 37-4.1 76-9.2 76-73.1 0-18.2-6.5-27.3-17.1-39 1.7-4.3 7.4-22-1.7-45-13.9-4.3-45.7 17.9-45.7 17.9-13.2-3.7-27.5-5.6-41.6-5.6-14.1 0-28.4 1.9-41.6 5.6 0 0-31.8-22.2-45.7-17.9-9.1 22.9-3.5 40.6-1.7 45-10.6 11.7-15.6 20.8-15.6 39 0 63.6 37.3 69 74.3 73.1-4.8 4.3-9.1 11.7-10.6 22.3-9.5 4.3-33.8 11.7-48.3-13.9-9.1-15.8-25.5-17.1-25.5-17.1-16.2-.2-1.1 10.2-1.1 10.2 10.8 5 18.4 24.2 18.4 24.2 9.7 29.7 56.1 19.7 56.1 19.7 0 13.9.2 36.5.2 40.6 0 4.3-3 9.5-11.5 8-66-22.1-112.2-84.9-112.2-158.3 0-91.8 70.2-161.5 162-161.5S388 165.6 388 257.4c.1 73.4-44.7 136.3-110.7 158.3zm-98.1-61.1c-1.9.4-3.7-.4-3.9-1.7-.2-1.5 1.1-2.8 3-3.2 1.9-.2 3.7.6 3.9 1.9.3 1.3-1 2.6-3 3zm-9.5-.9c0 1.3-1.5 2.4-3.5 2.4-2.2.2-3.7-.9-3.7-2.4 0-1.3 1.5-2.4 3.5-2.4 1.9-.2 3.7.9 3.7 2.4zm-13.7-1.1c-.4 1.3-2.4 1.9-4.1 1.3-1.9-.4-3.2-1.9-2.8-3.2.4-1.3 2.4-1.9 4.1-1.5 2 .6 3.3 2.1 2.8 3.4zm-12.3-5.4c-.9 1.1-2.8.9-4.3-.6-1.5-1.3-1.9-3.2-.9-4.1.9-1.1 2.8-.9 4.3.6 1.3 1.3 1.8 3.3.9 4.1zm-9.1-9.1c-.9.6-2.6 0-3.7-1.5s-1.1-3.2 0-3.9c1.1-.9 2.8-.2 3.7 1.3 1.1 1.5 1.1 3.3 0 4.1zm-6.5-9.7c-.9.9-2.4.4-3.5-.6-1.1-1.3-1.3-2.8-.4-3.5.9-.9 2.4-.4 3.5.6 1.1 1.3 1.3 2.8.4 3.5zm-6.7-7.4c-.4.9-1.7 1.1-2.8.4-1.3-.6-1.9-1.7-1.5-2.6.4-.6 1.5-.9 2.8-.4 1.3.7 1.9 1.8 1.5 2.6z" />
    </svg>
  </IconStyled>
)

export const LinkedIn = ({ color, positive }: Props): React.ReactNode => (
  <IconStyled color={color} positive={positive}>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512">
      <path d="M416 32H31.9C14.3 32 0 46.5 0 64.3v383.4C0 465.5 14.3 480 31.9 480H416c17.6 0 32-14.5 32-32.3V64.3c0-17.8-14.4-32.3-32-32.3zM135.4 416H69V202.2h66.5V416zm-33.2-243c-21.3 0-38.5-17.3-38.5-38.5S80.9 96 102.2 96c21.2 0 38.5 17.3 38.5 38.5 0 21.3-17.2 38.5-38.5 38.5zm282.1 243h-66.4V312c0-24.8-.5-56.7-34.5-56.7-34.6 0-39.9 27-39.9 54.9V416h-66.4V202.2h63.7v29.2h.9c8.9-16.8 30.6-34.5 62.9-34.5 67.2 0 79.7 44.3 79.7 101.9V416z" />
    </svg>
  </IconStyled>
)

export const Twitter = ({ color, positive }: Props): React.ReactNode => (
  <IconStyled color={color} positive={positive}>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512">
      <path d="M400 32H48C21.5 32 0 53.5 0 80v352c0 26.5 21.5 48 48 48h352c26.5 0 48-21.5 48-48V80c0-26.5-21.5-48-48-48zm-48.9 158.8c.2 2.8.2 5.7.2 8.5 0 86.7-66 186.6-186.6 186.6-37.2 0-71.7-10.8-100.7-29.4 5.3.6 10.4.8 15.8.8 30.7 0 58.9-10.4 81.4-28-28.8-.6-53-19.5-61.3-45.5 10.1 1.5 19.2 1.5 29.6-1.2-30-6.1-52.5-32.5-52.5-64.4v-.8c8.7 4.9 18.9 7.9 29.6 8.3a65.447 65.447 0 0 1-29.2-54.6c0-12.2 3.2-23.4 8.9-33.1 32.3 39.8 80.8 65.8 135.2 68.6-9.3-44.5 24-80.6 64-80.6 18.9 0 35.9 7.9 47.9 20.7 14.8-2.8 29-8.3 41.6-15.8-4.9 15.2-15.2 28-28.8 36.1 13.2-1.4 26-5.1 37.8-10.2-8.9 13.1-20.1 24.7-32.9 34z" />
    </svg>
  </IconStyled>
)

export const TypeForm = ({ color, positive }: Props): React.ReactNode => (
  <IconStyledLarge color={color} positive={positive}>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
      <path
        d="M181,292.3l66.1,38.2c0.7,0.4,1.6,0.4,2.3,0l31.5-18.2l35.7-20.6V214c0-0.8-0.4-1.6-1.1-2l-66.1-38.2
		c-0.7-0.4-1.6-0.4-2.3,0l-67.2,38.8l0,77.7C179.9,291.1,180.3,291.9,181,292.3z"
      />
      <path
        d="M498.4,473.6l-22.8-114c4-8.3,7.4-16.9,10.4-25.6h-0.8C537.1,181.7,428,21.6,267,13.8
		C128.3,7.4,13.2,118.4,13.2,256c0,133.6,108.6,242.3,242.1,242.5c35.7,0.1,71-7.9,103.3-23.2c126.4,25,116.5,23.2,119.4,23.2
		c11.5,0,20.8-9.3,20.8-20.8C498.8,476.3,498.7,474.9,498.4,473.6z M415.1,351.2c-0.2,0.4-0.5,0.6-0.8,0.8l-63.8,36.8
		c-0.7,0.4-1.6,0.4-2.3,0l-31.5-18.2l-67.2,38.8c-0.7,0.4-1.6,0.4-2.3,0l-134.5-77.6c-0.7-0.4-1.2-1.2-1.2-2v-76.2
		c0-0.8,0.4-1.6,1.2-2l67.2-38.9l-64.9-37.5c-1.1-0.6-1.5-2.1-0.8-3.2c0.2-0.4,0.5-0.6,0.8-0.8l132.1-76.3c0.7-0.4,1.6-0.4,2.3,0
		l134.5,77.6c0.7,0.4,1.2,1.2,1.2,2v156.6l29.2,16.9C415.3,348.6,415.7,350.1,415.1,351.2z"
      />{' '}
    </svg>
  </IconStyledLarge>
)
