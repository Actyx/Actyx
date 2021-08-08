import * as React from 'react'
import { Toolbar, Typography } from '@actyx/industrial-ui'
import { Machines } from './Machines'
import { Orders } from './Orders'

// responsive css to show the dashboard on a smartphone
import './main.css'

export const App = (): JSX.Element => (
  <>
    <Toolbar variant="dark">
      <div style={{ width: '32px', marginRight: '24px', marginLeft: '24px', marginTop: '6px' }}>
        <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 256 256">
          <path
            fill="#ffffff"
            d="M226.76,192.15V64.94a1.87,1.87,0,0,0-.94-1.63L116.6.25a1.91,1.91,0,0,0-1.88,0L7.38,62.22a1.89,1.89,0,0,0,0,3.27L60.11,95.93,114.72,64.4a1.86,1.86,0,0,1,1.88,0l53.67,31a1.89,1.89,0,0,1,.94,1.63v63.06h0l-29,16.74L116.6,191.6a1.86,1.86,0,0,1-1.88,0l-53.67-31a1.89,1.89,0,0,1-.94-1.63V95.93L5.5,127.5a1.87,1.87,0,0,0-.94,1.63v61.93a1.89,1.89,0,0,0,.94,1.63l109.22,63.06a1.91,1.91,0,0,0,1.88,0l54.61-31.53L196.83,239a1.88,1.88,0,0,0,1.89,0l51.78-29.9a1.88,1.88,0,0,0,0-3.26Z"
          />
        </svg>
      </div>
      <Typography variant="distance" color="#ffffff">
        Dashboard
      </Typography>
    </Toolbar>{' '}
    <div className="main" style={{ display: 'flex', flexDirection: 'column' }}>
      <div style={{ padding: '24px' }}>
        <Typography variant="distance" semiBold>
          Available Machines
        </Typography>
        <Machines />
      </div>
      <div style={{ padding: '24px' }}>
        <Typography variant="distance" semiBold>
          ERP Orders
        </Typography>
        <Orders />
      </div>
    </div>
  </>
)
