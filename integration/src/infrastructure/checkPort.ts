import execa from 'execa'
import net from 'net'
import { Target } from './types'

export const getFreePort = (): Promise<number> =>
  new Promise((res, rej) => {
    const server = net.createServer()
    server.once('error', rej)
    server.once('listening', () => {
      const addr = server.address()
      if (typeof addr !== 'object' || addr === null) {
        server.close()
        rej(new Error(`listening server address was ${addr}`))
      } else {
        server.close(() => res(addr.port))
      }
    })
    server.listen()
  })

export const getFreeRemotePort = (target: Target): Promise<number> => {
  const script =
    target.os === 'windows'
      ? String.raw`Function Get-RandomPort
{
    return Get-Random -Max 32767 -Min 10001;
}

Function Test-PortInUse
{
    Param(
        [Parameter(Mandatory=$true)]
        [Int] $portToTest
    );
    $count = netstat -aon | find ":$portToTest " /c;
    return [bool]($count -gt 0);
}

Function Get-RandomUsablePort
{
    Param(
        [Int] $maxTries = 100
    );
    $result = -1;
    $tries = 0;
    DO
    {
        $randomPort = Get-RandomPort;
        if (-Not (Test-PortInUse($randomPort)))
        {
            $result = $randomPort;
        }
        $tries += 1;
    } While (($result -lt 0) -and ($tries -lt $maxTries));
    return $result;
}
Get-RandomUsablePort`
      : String.raw`comm -23 <(seq 49152 65535 | sort) <(ss -Htan | awk '{print $4}' | cut -d':' -f2 | sort -u) | shuf | head -n 1`
  return target.execute(script, []).process.then((x) => Number(x.stdout.trim()))
}

export const occupyPort = (port: number): Promise<net.Server> =>
  new Promise((res, rej) => {
    const server = net.createServer()
    server.once('error', rej)
    server.once('listening', () => {
      const addr = server.address()
      if (typeof addr !== 'object' || addr === null) {
        server.close()
        rej(new Error(`listening server address was ${addr}`))
      } else {
        res(server)
      }
    })
    server.listen(port, '0.0.0.0')
  })

export const occupyRemotePort = (target: Target, port: number): execa.ExecaChildProcess<string> => {
  const script =
    target.os === 'windows'
      ? String.raw`$Listener = [System.Net.Sockets.TcpListener]${port};
$Listener.Start();`
      : `ncat -l -p ${port}`
  return target.execute(script, []).process
}
