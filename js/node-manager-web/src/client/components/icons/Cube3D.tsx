import React from "react"
import clsx from "clsx"
import { IconType } from "./types"

const Icon: IconType = ({ height = 5, width = 5, className }) => (
  <svg
    className={clsx(`w-${width} h-${height}`, className)}
    xmlns="http://www.w3.org/2000/svg"
    fill="none"
    viewBox="0 0 24 24"
    stroke="currentColor"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"
    />
  </svg>
)

export default Icon
