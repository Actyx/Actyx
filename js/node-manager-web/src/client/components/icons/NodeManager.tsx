import React from "react";
import { IconType } from "./types";

const Icon: IconType = ({ height = 5, width = 5 }) => (
  <svg
    className={`w-${width} h-${height}`}
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 1024 1024"
  >
    <defs>
      <linearGradient
        id="b"
        x1="512.05"
        x2="512.05"
        y1="1007.22"
        y2="16"
        gradientUnits="userSpaceOnUse"
      >
        <stop offset="0" stopColor="#d1d5dc" />
        <stop offset="1" stopColor="#e9eff4" />
      </linearGradient>
      <filter id="a">
        <feGaussianBlur in="SourceAlpha" stdDeviation="5" />
        <feOffset dy="6" />
        <feMerge>
          <feMergeNode />
          <feMergeNode in="SourceGraphic" />
        </feMerge>
      </filter>
    </defs>
    <g filter="url(#a)" opacity=".5">
      <rect
        width="991.22"
        height="986.87"
        x="16.44"
        y="18.41"
        fill="#7f7f7f"
        rx="221.25"
      />
    </g>
    <rect
      width="991.22"
      height="991.22"
      x="16.44"
      y="16"
      fill="url(#b)"
      rx="220.4"
    />
    <path
      fill="#374151"
      d="M835.57 710.22v-394a5.81 5.81 0 00-2.92-5.05L494.38 115.91a5.84 5.84 0 00-5.83 0L156.12 307.84a5.83 5.83 0 000 10.1l163.3 94.28 169.13-97.65a5.84 5.84 0 015.83 0l166.22 96a5.83 5.83 0 012.92 5v195.3l-89.79 51.84-79.35 45.81a5.84 5.84 0 01-5.83 0l-166.22-96a5.82 5.82 0 01-2.91-5.05V412.22L150.28 510a5.85 5.85 0 00-2.91 5v191.86a5.84 5.84 0 002.91 5l338.27 195.3a5.84 5.84 0 005.83 0l169.14-97.65 79.34 45.81a5.86 5.86 0 005.84 0l160.38-92.6a5.83 5.83 0 000-10.1z"
    />
  </svg>
);

export default Icon;
