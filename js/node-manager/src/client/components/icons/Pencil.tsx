import React from 'react'
import { IconType } from './types'
import clsx from 'clsx'

const Icon: IconType = ({ height = 5, width = 5 }) => (
  <svg
    className={clsx(`w-${width} h-${height}`)}
    xmlns="http://www.w3.org/2000/svg"
    xmlSpace="preserve"
    style={{
      fillRule: 'evenodd',
      clipRule: 'evenodd',
      strokeLinejoin: 'round',
      strokeMiterlimit: 2,
    }}
    viewBox="0 0 64 64"
  >
    <path
      d="M-1024 0H256v800h-1280z"
      style={{
        fill: 'none',
      }}
    />
    <path
      d="M55.944 51.712v4.201l-33.652-.027 4.71-4.174h28.942ZM48.389 8c1.649 0 2.505.128 4.752 2.011 2.294 1.921 2.707 3.419 2.803 5.087.102 1.795-.504 3.976-2.188 5.681L21.795 52.74c-.52.475-.629.45-.977.553L10.592 55.85c-1.472.299-2.854-1.049-2.55-2.55l2.557-10.226c.1-.334.133-.517.553-.976 10.696-10.697 21.195-21.594 32.09-32.087C44.663 8.676 46.739 8 48.389 8ZM16.014 43.182l-1.477 1.477-1.566 6.262 6.262-1.566 1.479-1.474-4.698-4.699ZM46.19 22.609l-4.802-4.801-22.493 22.493 4.712 4.713c7.549-7.448 15.196-14.801 22.583-22.405Zm2.826-2.936c.618-.648 1.234-1.298 1.848-1.951 1.673-1.826.443-5.454-2.307-5.578-.056-.002-.112-.002-.168-.002a3.406 3.406 0 0 0-2.312.977l-1.807 1.808 4.746 4.746Z"
      style={{
        fillRule: 'nonzero',
      }}
    />
  </svg>
)

export default Icon
