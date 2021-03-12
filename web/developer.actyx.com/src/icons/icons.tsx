import React from 'react'
import styled from 'styled-components'

const IconStyledBlue = styled.span`
  svg {
    path {
      fill: #1998ff;
    }
    height: 21px;
    margin-right: 10px;
    padding-top: 3px;
  }
`

const IconStyledGray = styled.span`
  svg {
    path {
      fill: #586069;
    }
    height: 18px;
    margin-right: 10px;
    padding-top: 3px;
  }
`
export const Calendar = () => (
  <IconStyledGray>
    <svg
      id="f7439ad1-7d3e-4e3e-a8ef-26e277f9b96e"
      data-name="Layer 1"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 256 256"
    >
      <path d="M197.18,37.76H195.5V10.24a10,10,0,0,0-20,0V37.76h-95V10.24a10,10,0,0,0-20,0V37.76H58.82A58.83,58.83,0,0,0,0,96.59V196.94a58.82,58.82,0,0,0,58.82,58.82H197.18A58.82,58.82,0,0,0,256,196.94V96.59A58.83,58.83,0,0,0,197.18,37.76Zm-138.36,20H60.5v12a10,10,0,0,0,20,0v-12h95v12a10,10,0,0,0,20,0v-12h1.68A38.87,38.87,0,0,1,236,96.59v14.17H20V96.59A38.87,38.87,0,0,1,58.82,57.76Zm138.36,178H58.82A38.86,38.86,0,0,1,20,196.94V130.76H236v66.18A38.86,38.86,0,0,1,197.18,235.76Z" />
    </svg>
  </IconStyledGray>
)

export const Commit = () => (
  <IconStyledGray>
    <svg
      id="a00db321-2efd-425e-ba2c-c9bb8ec8679d"
      data-name="Layer 1"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 256 256"
    >
      <path d="M243,118H187.15a60,60,0,0,0-118.3,0H13a10,10,0,0,0,0,20H68.85a60,60,0,0,0,118.3,0H243a10,10,0,0,0,0-20ZM128,168a40,40,0,1,1,40-40A40,40,0,0,1,128,168Z" />
    </svg>
  </IconStyledGray>
)

export const Laptop = () => (
  <IconStyledBlue>
    <svg
      id="f6d9f820-339e-4c18-981f-2722f95a9181"
      data-name="Layer 1"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 256 256"
    >
      <path d="M214.79,58A3.21,3.21,0,0,1,218,61.21V161H38V61.21A3.21,3.21,0,0,1,41.21,58H214.79m0-20H41.21A23.21,23.21,0,0,0,18,61.21V181H238V61.21A23.21,23.21,0,0,0,214.79,38Z" />
      <path d="M256,188H0v12a17,17,0,0,0,17,17H239a17,17,0,0,0,17-17V188Z" />
    </svg>
  </IconStyledBlue>
)
