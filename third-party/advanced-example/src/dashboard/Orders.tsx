import * as React from 'react'
import { useRegistryFish } from '@actyx-contrib/react-pond'
import {
  Typography,
  Table,
  TableCell,
  TableBody,
  TableRow,
  TableHeader,
} from '@actyx/industrial-ui'
import { OrderFish } from '../fish/orderFish'

export const Orders = () => {
  // Get all order states with the useRegistryFish. If a state changes or the registry changes, the component gets redrawn
  const orders = useRegistryFish(OrderFish.registry, Object.keys, OrderFish.of)
  return (
    <Table alternateColor>
      <TableHeader>
        <TableCell>
          <Typography variant="standard" bold>
            Order Number
          </Typography>
        </TableCell>
        <TableCell>
          <Typography variant="standard" bold>
            Machine State
          </Typography>
        </TableCell>
        <TableCell>
          <Typography variant="standard" bold>
            Machine
          </Typography>
        </TableCell>
        <TableCell numeric>
          <Typography variant="standard" bold>
            Duration
          </Typography>
        </TableCell>
      </TableHeader>
      <TableBody>
        {orders
          .map((s) => {
            console.log(s)
            return s
          })
          .map((m, idx) => (
            <TableRow key={m.state.stateType !== 'undefined' ? m.state.name : idx}>
              {m.state.stateType !== 'undefined' && (
                <>
                  <TableCell>
                    <Typography variant="standard">{m.props}</Typography>
                  </TableCell>
                  <TableCell>
                    <Typography variant="standard">{m.state.stateType}</Typography>
                  </TableCell>
                  <TableCell>
                    <Typography variant="standard">{m.state.machine}</Typography>
                  </TableCell>
                  <TableCell numeric>
                    <Typography variant="standard">{m.state.duration}h</Typography>
                  </TableCell>
                </>
              )}
            </TableRow>
          ))}
      </TableBody>
    </Table>
  )
}
